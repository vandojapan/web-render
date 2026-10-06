mod image_buffers;
mod module;
mod native_processing;
mod project_root;
#[cfg(feature = "runtime-debug")]
mod runtime_debug;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use aviutl2::{anyhow, tracing as log};
use tap::prelude::*;
use tokio::io::AsyncBufReadExt;

type Vi5Server = Arc<
    tokio::sync::Mutex<
        Option<(
            Arc<tokio::sync::Mutex<tokio::process::Child>>,
            web_render_cef::Client,
        )>,
    >,
>;

#[aviutl2::plugin(GenericPlugin)]
struct Vi5Aux2 {
    pub runtime: Arc<std::sync::RwLock<Option<tokio::runtime::Runtime>>>,
    server: Vi5Server,
    native: Arc<tokio::sync::Mutex<Option<native_processing::Client>>>,
    project_dir: Arc<tokio::sync::Mutex<Option<String>>>,
    notification_task: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    generation: Arc<AtomicU64>,

    plugin: aviutl2::generic::SubPlugin<crate::module::InternalModule>,
}

static CURRENT_PROJECT_FILE: std::sync::Mutex<Option<std::path::PathBuf>> =
    std::sync::Mutex::new(None);
static VAR_PREFIX: &str = "WEB_RENDER_AUX2_";

fn get_script_dir(project_name: &str) -> std::path::PathBuf {
    aviutl2::config::app_data_path()
        .join("Script")
        .join(format!("web-render.aux2_{}", project_name))
}

static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle = aviutl2::generic::GlobalEditHandle::new();

#[aviutl2::generic::menus]
impl Vi5Aux2 {
    #[config(name = "[web-render.aux2] プロジェクトフォルダの設定")]
    fn select_project_dir(&mut self, _hwnd: aviutl2::Win32WindowHandle) -> anyhow::Result<()> {
        let current_dir = match CURRENT_PROJECT_FILE.lock().unwrap().as_ref() {
            Some(path) => {
                let path = std::path::Path::new(&path);
                if let Some(parent) = path.parent() {
                    parent.to_path_buf()
                } else {
                    "".into()
                }
            }
            None => "".into(),
        };
        let dir = rfd::FileDialog::new()
            .set_directory(current_dir)
            .set_title("プロジェクトフォルダを選択してください")
            .pick_folder();
        let Some(dir) = dir else {
            return Ok(());
        };
        let dir_str = dir.to_string_lossy().to_string();
        self.set_project_dir(dir_str)?;
        Ok(())
    }
}
impl Vi5Aux2 {
    fn clear_project(&mut self) {
        *self.project_dir.blocking_lock() = None;
        let _ = native_processing::set_fingerprint(None);
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        crate::module::clear_render_cache();
        if let Some(task) = self.notification_task.blocking_lock().take() {
            task.abort();
        }
        let server = self.server.clone();
        let native = self.native.clone();
        let current_generation = self.generation.clone();
        self.get_runtime_handle().spawn(async move {
            let mut guard = server.lock().await;
            if current_generation.load(Ordering::SeqCst) != generation {
                return;
            }
            if let Some(client) = native.lock().await.take() {
                client.shutdown().await;
            }
            if let Some((child, mut client)) = guard.take() {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(2), client.shutdown())
                    .await;
                let mut child = child.lock().await;
                if tokio::time::timeout(std::time::Duration::from_secs(5), child.wait())
                    .await
                    .is_err()
                {
                    let _ = child.kill().await;
                }
            }
        });
    }

    fn set_project_dir(&mut self, dir: String) -> anyhow::Result<()> {
        log::info!("Setting project directory to: {}", dir);
        project_root::validate(std::path::Path::new(&dir))?;
        native_processing::set_fingerprint(
            std::path::Path::new(&dir)
                .join(web_render_processing::MANIFEST)
                .is_file()
                .then_some(std::path::Path::new(&dir)),
        )?;

        *self.project_dir.blocking_lock() = Some(dir.clone());
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        crate::module::clear_render_cache();

        let runtime_handle = self.get_runtime_handle();
        let project_dir = self.project_dir.clone();
        let server = self.server.clone();
        let native = self.native.clone();
        let notification_task = self.notification_task.clone();
        let current_generation = self.generation.clone();
        runtime_handle.spawn(async move {
            if let Err(e) = Self::initialize_project_dir(
                dir,
                project_dir,
                server,
                native,
                notification_task,
                current_generation,
                generation,
            )
            .await
            {
                log::error!("Failed to initialize project directory: {}", e);
            }
        });
        Ok(())
    }

    async fn initialize_project_dir(
        dir: String,
        project_dir: Arc<tokio::sync::Mutex<Option<String>>>,
        server: Vi5Server,
        native: Arc<tokio::sync::Mutex<Option<native_processing::Client>>>,
        notification_task: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
        current_generation: Arc<AtomicU64>,
        generation: u64,
    ) -> anyhow::Result<()> {
        {
            let guard = project_dir.lock().await;
            if guard.as_deref() != Some(dir.as_str()) {
                return Ok(());
            }
        }

        let mut server_guard = server.lock().await;
        if current_generation.load(Ordering::SeqCst) != generation {
            return Ok(());
        }
        if let Some(task) = notification_task.lock().await.take() {
            task.abort();
        }
        if std::path::Path::new(&dir)
            .join(web_render_processing::MANIFEST)
            .is_file()
        {
            let mut native_guard = native.lock().await;
            if let Some(client) = native_guard.take() {
                client.shutdown().await;
            }
            // Switching from a browser project releases its CEF worker as well.
            if let Some((child, mut client)) = server_guard.take() {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(2), client.shutdown())
                    .await;
                let mut child = child.lock().await;
                if tokio::time::timeout(std::time::Duration::from_secs(5), child.wait())
                    .await
                    .is_err()
                {
                    let _ = child.kill().await;
                }
            }
            let client = native_processing::Client::start(std::path::Path::new(&dir)).await?;
            if current_generation.load(Ordering::SeqCst) != generation {
                client.shutdown().await;
                return Ok(());
            }
            Self::update_script_dir(&client.project.name, &client.objects()).await?;
            #[cfg(feature = "runtime-debug")]
            runtime_debug::schedule(client.project.name.clone());
            *native_guard = Some(client);
            return Ok(());
        }
        if let Some(client) = native.lock().await.take() {
            client.shutdown().await;
        }
        if let Some((child, _)) = server_guard.as_ref()
            && child.lock().await.try_wait()?.is_some()
        {
            *server_guard = None;
        }
        if server_guard.is_none() {
            let (child, client) = Self::start_web_render_cef_server().await.inspect_err(|e| {
                let _ = native_dialog::DialogBuilder::message()
                    .set_title("web-render.aux2")
                    .set_text(format!(
                        "web-render-cef サーバーの起動に失敗しました:\n{}",
                        e
                    ))
                    .set_level(native_dialog::MessageLevel::Error)
                    .alert()
                    .show();
            })?;
            let child = Arc::new(tokio::sync::Mutex::new(child));
            Self::spawn_web_render_cef_exit_logger(child.clone());
            *server_guard = Some((child, client));
        }
        let client = server_guard.as_mut().map(|(_, client)| client).unwrap();

        let info = client
            .initialize(&dir, Some(std::time::Duration::from_secs(60)))
            .await
            .map_err(|e| {
                anyhow::anyhow!("web-render-cef クライアントの初期化に失敗しました: {}", e)
            })?;
        log::info!("web-render-cef initialized successfully.");
        #[cfg(feature = "runtime-debug")]
        runtime_debug::schedule(info.project_name.clone());
        if current_generation.load(Ordering::SeqCst) == generation {
            *notification_task.lock().await = Some(tokio::spawn(Self::notification_listener_task(
                info.project_name.clone(),
                client.clone(),
                current_generation,
                generation,
            )));
        }
        Ok(())
    }

    async fn update_script_dir(
        project_name: &str,
        object_infos: &[web_render_cef::ObjectInfo],
    ) -> anyhow::Result<()> {
        let mut requires_restart = false;
        let mut requires_reload = false;

        let module_name = process_path::get_dylib_path()
            .expect("Failed to get dylib path (unreachable on Windows)")
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let script_dir = get_script_dir(project_name);
        log::info!("Project script directory: {:?}", script_dir);
        if !script_dir.exists() {
            tokio::fs::create_dir_all(&script_dir).await?;
            requires_restart = true;
        }

        for object in object_infos {
            let base_script = include_str!("./script.lua").to_string();
            let param_defs = object
                .parameter_definitions
                .iter()
                .map(|param| {
                    let key = &param.key;
                    let label = &param.label;
                    match param.parameter_type {
                        web_render_cef::ParameterType::String => {
                            let default_value = match &param.default_value {
                                Some(web_render_cef::Parameter {
                                    value: web_render_cef::ParameterValue::Str(value),
                                    ..
                                }) => value.clone(),
                                _ => "".to_string(),
                            };
                            let default_value = serde_json::to_string(&default_value).unwrap();
                            format!(r#"--value@{VAR_PREFIX}{key}:{label},{default_value}"#)
                        }
                        web_render_cef::ParameterType::Text => {
                            let default_value = match &param.default_value {
                                Some(web_render_cef::Parameter {
                                    value: web_render_cef::ParameterValue::Text(value),
                                    ..
                                }) => value.clone(),
                                _ => "".to_string(),
                            };
                            // --text defaults are escaped text, not Lua string literals.
                            let escaped = serde_json::to_string(&default_value).unwrap();
                            let default_value = &escaped[1..escaped.len() - 1];
                            format!(r#"--text@{VAR_PREFIX}{key}:{label},{default_value}"#)
                        }
                        web_render_cef::ParameterType::Boolean => {
                            let default_value = match &param.default_value {
                                Some(web_render_cef::Parameter {
                                    value: web_render_cef::ParameterValue::Bool(value),
                                    ..
                                }) => *value,
                                _ => false,
                            };
                            let default_value = if default_value { "true" } else { "false" };
                            format!(r#"--check@{VAR_PREFIX}{key}:{label},{default_value}"#)
                        }
                        web_render_cef::ParameterType::Number { step, min, max } => {
                            let min_str = min.to_string();
                            let max_str = max.to_string();
                            let step = step.as_str();
                            let default_value = match &param.default_value {
                                Some(web_render_cef::Parameter {
                                    value: web_render_cef::ParameterValue::Number(value),
                                    ..
                                }) => *value,
                                _ => min,
                            };
                            format!(
                                r#"--track@{VAR_PREFIX}{key}:{label},{min_str},{max_str},{default_value},{step}"#
                            )
                        }
                        web_render_cef::ParameterType::Color => {
                            let default_value = match &param.default_value {
                                Some(web_render_cef::Parameter {
                                    value: web_render_cef::ParameterValue::Color(value),
                                    ..
                                }) => {
                                    if value.a == 0 {
                                        "nil".to_string()
                                    } else {
                                        let r = value.r as u32;
                                        let g = value.g as u32;
                                        let b = value.b as u32;
                                        format!("0x{:02X}{:02X}{:02X}", r, g, b)
                                    }
                                }
                                _ => "nil".to_string(),
                            };
                            format!(r#"--color@{VAR_PREFIX}{key}:{label},{default_value}"#)
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let keys = object
                .parameter_definitions
                .iter()
                .map(|param| serde_json::to_string(&param.key).unwrap())
                .collect::<Vec<_>>()
                .join(",");
            let values = object
                .parameter_definitions
                .iter()
                .map(|param| {
                    let key = &param.key;
                    match param.parameter_type {
                        web_render_cef::ParameterType::Number { .. } => {
                            format!(r#"{VAR_PREFIX}{key}"#)
                        }
                        _ => {
                            format!(r#"{VAR_PREFIX}{key}"#)
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            let types = object
                .parameter_definitions
                .iter()
                .map(|param| match param.parameter_type {
                    web_render_cef::ParameterType::String => r#""Str""#.to_string(),
                    web_render_cef::ParameterType::Text => r#""Text""#.to_string(),
                    web_render_cef::ParameterType::Boolean => r#""Bool""#.to_string(),
                    web_render_cef::ParameterType::Number { .. } => r#""Number""#.to_string(),
                    web_render_cef::ParameterType::Color => r#""Color""#.to_string(),
                })
                .collect::<Vec<_>>()
                .join(",");

            let script_content = base_script
                .replace("--PARAMETER_DEFINITIONS--", param_defs.as_str())
                .replace(
                    "--LABEL--",
                    format!("--label:web-render.aux2\\{}", project_name).as_str(),
                )
                .replace("--MODULE_NAME--", module_name.as_str())
                .replace("--PARAMETER_KEYS--", keys.as_str())
                .replace("--PARAMETER_VALUES--", values.as_str())
                .replace(r#"--PARAMETER_TYPES--"#, types.as_str())
                .replace(
                    r#""--OBJECT_ID--""#,
                    serde_json::to_string(&object.id).unwrap().as_str(),
                );
            let script_path = script_dir.join(format!("{}.obj2", object.label));
            log::info!(
                "Loaded script for object '{}': {:?}",
                object.id,
                script_path
            );
            if script_path.exists() {
                let existing_content = tokio::fs::read_to_string(&script_path).await?;
                let existing_headers = existing_content
                    .lines()
                    .take_while(|line| line != &"--END_HEADER")
                    .collect::<Vec<_>>()
                    .join("\n");
                let new_headers = script_content
                    .lines()
                    .take_while(|line| line != &"--END_HEADER")
                    .collect::<Vec<_>>()
                    .join("\n");
                log::debug!(
                    "Comparing existing and new script contents for object '{}'",
                    object.id
                );
                log::debug!("Existing headers:\n{}", existing_headers);
                log::debug!("New headers:\n{}", new_headers);
                if existing_content == script_content {
                    log::info!(
                        "Script file for object '{}' is up to date: {:?}",
                        object.id,
                        script_path
                    );
                } else {
                    tokio::fs::write(&script_path, script_content).await?;
                    log::info!(
                        "Updated script file for object '{}': {:?}",
                        object.id,
                        script_path
                    );
                    if existing_headers != new_headers {
                        log::warn!("Script file for object '{}' has updated headers", object.id,);
                        requires_restart = true;
                    } else {
                        requires_reload = true;
                    }
                }
            } else {
                tokio::fs::write(&script_path, script_content).await?;
                log::info!(
                    "Created script file for object '{}': {:?}",
                    object.id,
                    script_path
                );
                requires_restart = true;
            }
        }

        #[cfg(feature = "runtime-debug")]
        if runtime_debug::enabled() {
            log::info!(
                "Debug bootstrap: script update restart={requires_restart}, reload={requires_reload}"
            );
            return Ok(());
        }
        if requires_restart {
            log::info!("Script directory updated requiring restart.");
            let will_restart = native_dialog::DialogBuilder::message()
                .set_title("web-render.aux2")
                .set_text("オブジェクトが更新されました。\n反映にはAviUtl2の再起動が必要です。今すぐ再起動しますか？")
                .confirm()
                .spawn()
                .await?;
            if will_restart {
                log::info!("Restarting AviUtl2...");
                EDIT_HANDLE.restart_host_app();
            }
        } else if requires_reload {
            log::info!("Script directory updated.");
            native_dialog::DialogBuilder::message()
                .set_title("web-render.aux2")
                .set_text(
                    "オブジェクトが更新されました。\nF5を押してスクリプトをリロードしてください。",
                )
                .alert()
                .spawn()
                .await?;
        } else {
            log::info!("Script directory is up to date.");
        }

        Ok(())
    }

    async fn start_web_render_cef_server()
    -> anyhow::Result<(tokio::process::Child, web_render_cef::Client)> {
        let listener = std::net::TcpListener::bind("[::1]:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        log::info!("Starting web-render-cef server on port {}", port);
        let plugin_path = process_path::get_dylib_path()
            .ok_or_else(|| anyhow::anyhow!("Cannot locate plugin DLL"))?;
        let plugin_dir = plugin_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot locate plugin directory"))?;
        let cef_server_path = plugin_dir.join("web-render-cef-server.exe");
        let mut child = tokio::process::Command::new(&cef_server_path)
            .arg("--port")
            .arg(port.to_string())
            .arg("--parent-process")
            .arg(std::process::id().to_string())
            .env("NO_COLOR", "1")
            .env("RUST_LOG", "info,web_render_cef=trace")
            // NOTE: C:\Windows\System32 で起動するとなぜかlibcef.dllを見つけられなくて落ちるので、カレントディレクトリを実行ファイルのディレクトリにする
            .current_dir(cef_server_path.parent().unwrap())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .tap(|cmd| {
                log::info!("Launching web-render-cef server: {cmd:?}");
            })
            .spawn()
            .map_err(|e| anyhow::anyhow!("web-render-cef サーバーの起動に失敗しました: {}", e))?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        tokio::spawn(async move {
            let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
            while let Some(line) = stdout_reader.next_line().await.transpose() {
                if let Ok(line) = line {
                    log::debug!("[web-render-cef-server stdout] {}", line);
                }
            }
        });
        tokio::spawn(async move {
            let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();
            while let Some(line) = stderr_reader.next_line().await.transpose() {
                if let Ok(line) = line {
                    log::error!("[web-render-cef-server stderr] {}", line);
                }
            }
        });
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let client = loop {
            if let Some(code) = child.try_wait()? {
                anyhow::bail!("CEF exited before readiness: {code}");
            }
            match web_render_cef::Client::connect(format!("http://[::1]:{port}")).await {
                Ok(client) => break client,
                Err(error) if tokio::time::Instant::now() >= deadline => return Err(error.into()),
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
            }
        };
        Ok((child, client))
    }

    async fn notification_listener_task(
        project_name: String,
        mut client: web_render_cef::Client,
        current_generation: Arc<AtomicU64>,
        generation: u64,
    ) {
        let mut stream = match client.subscribe_notifications().await {
            Ok(stream) => stream,
            Err(e) => {
                log::error!("Failed to subscribe notifications: {}", e);
                return;
            }
        };
        log::info!("Started notification listener task");

        loop {
            if current_generation.load(Ordering::SeqCst) != generation {
                break;
            }
            match stream.message().await {
                Ok(Some(notification))
                    if current_generation.load(Ordering::SeqCst) == generation =>
                {
                    match notification {
                        web_render_cef::Notification::Log(log) => match log.level {
                            web_render_cef::LogNotificationLevel::Info => {
                                log::info!("vi5 notification: {}", log.message);
                            }
                            web_render_cef::LogNotificationLevel::Warn => {
                                log::warn!("vi5 notification: {}", log.message);
                            }
                            web_render_cef::LogNotificationLevel::Error => {
                                log::error!("vi5 notification: {}", log.message);
                            }
                        },
                        web_render_cef::Notification::ObjectInfos(object_infos) => {
                            log::info!(
                                "Received object infos notification with {} objects",
                                object_infos.object_infos.len()
                            );
                            if let Err(e) =
                                Self::update_script_dir(&project_name, &object_infos.object_infos)
                                    .await
                            {
                                log::error!("Failed to update script directory: {}", e);
                            }
                        }
                    }
                }
                Ok(Some(_)) => break,
                Ok(None) => {
                    log::info!("Notification stream closed");
                    break;
                }
                Err(e) => {
                    log::error!("Notification stream error: {}", e);
                    break;
                }
            }
        }
    }

    fn spawn_web_render_cef_exit_logger(child: Arc<tokio::sync::Mutex<tokio::process::Child>>) {
        tokio::spawn(async move {
            loop {
                let status = {
                    let mut child = child.lock().await;
                    match child.try_wait() {
                        Ok(Some(status)) => Some(status),
                        Ok(None) => None,
                        Err(e) => {
                            log::error!("Failed to wait for web-render-cef server process: {}", e);
                            return;
                        }
                    }
                };
                if let Some(status) = status {
                    match status.code() {
                        Some(0) => {
                            log::info!("web-render-cef server has exited normally.");
                        }
                        Some(code) => {
                            log::error!("web-render-cef server has exited (exit code: {})", code);
                        }
                        None => {
                            log::error!("web-render-cef server has exited (unknown exit code)");
                        }
                    }
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    fn get_runtime_handle(&self) -> RuntimeHandle {
        RuntimeHandle {
            runtime: self.runtime.clone(),
        }
    }

    pub async fn with_client<F, R>(&self, f: F) -> anyhow::Result<R>
    where
        F: AsyncFnOnce(&mut web_render_cef::Client) -> anyhow::Result<R>,
    {
        let mut server_guard = self.server.lock().await;
        let Some((_, client)) = server_guard.as_mut() else {
            anyhow::bail!("web-render-cef server is not running")
        };
        f(client).await
    }

    async fn render_batch(
        &self,
        requests: Vec<web_render_cef::RenderRequest>,
    ) -> anyhow::Result<Vec<web_render_cef::RenderResponse>> {
        let mut native = self.native.lock().await;
        if let Some(client) = native.as_mut() {
            return client.batch_render(requests).await;
        }
        drop(native);
        self.with_client(async move |client| {
            Ok(tokio::time::timeout(
                std::time::Duration::from_secs(35),
                client.batch_render(requests),
            )
            .await??)
        })
        .await
    }
}

struct RuntimeHandle {
    runtime: Arc<std::sync::RwLock<Option<tokio::runtime::Runtime>>>,
}
impl RuntimeHandle {
    fn spawn<F>(&self, fut: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let guard = self.runtime.read().unwrap();
        let runtime = guard.as_ref().expect("Runtime has been shut down");
        runtime.spawn(fut);
    }
}

impl aviutl2::generic::GenericPlugin for Vi5Aux2 {
    fn new(info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        let _ = aviutl2::tracing_subscriber::fmt()
            .with_max_level(aviutl2::tracing::Level::DEBUG)
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .try_init();
        Ok(Self {
            runtime: Arc::new(std::sync::RwLock::new(Some(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap(),
            ))),
            server: Arc::new(tokio::sync::Mutex::new(None)),
            native: Arc::new(tokio::sync::Mutex::new(None)),
            project_dir: Arc::new(tokio::sync::Mutex::new(None)),
            notification_task: Arc::new(tokio::sync::Mutex::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
            plugin: aviutl2::generic::SubPlugin::new_script_module(&info)?,
        })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "web-render.aux2".to_string(),
            information: format!(
                "web-render.aux2 / https://github.com/sevenc-nanashi/vi5.aux2 / v{}",
                env!("CARGO_PKG_VERSION")
            ),
        }
    }

    fn register(&mut self, host_app_handle: &mut aviutl2::generic::HostAppHandle) {
        host_app_handle.register_menus::<Vi5Aux2>();
        host_app_handle.register_script_module(None, &self.plugin);
        EDIT_HANDLE.init(host_app_handle.create_edit_handle());
    }

    fn on_project_load(&mut self, project: &mut aviutl2::generic::ProjectFile) {
        *CURRENT_PROJECT_FILE.lock().unwrap() = project.get_path();
        // An invalid saved path must not leave the previous project's server/cache active.
        self.clear_project();
        if matches!(
            project.get_param_string("project_dir"),
            Err(aviutl2::generic::ProjectFileError::RetrievalFailed(_))
        ) {
            log::info!("No web project configured in this project file.");
            return;
        }
        match project.deserialize::<String>("project_dir") {
            Ok(dir) => {
                if dir.is_empty() {
                    log::info!("No project directory set in project file.");
                } else {
                    log::info!("Loaded project directory from project file: {}", dir);
                    if let Err(e) = self.set_project_dir(dir) {
                        log::error!("Failed to set project directory: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to get project parameter: {}", e);
            }
        }
    }

    fn on_clear_cache(&mut self, _edit_section: &aviutl2::generic::EditSection) {
        crate::module::clear_render_cache();
        self.get_runtime_handle().spawn({
            let server = Arc::clone(&self.server);
            async move {
                let mut server = server.lock().await;
                let Some((_, client)) = server.as_mut() else {
                    log::warn!("web-render-cef server is not running, cannot purge cache");
                    return;
                };
                if let Err(e) = client.purge_cache().await {
                    log::error!("Failed to purge web-render-cef cache: {}", e);
                }
            }
        });
    }

    fn on_project_save(&mut self, project: &mut aviutl2::generic::ProjectFile) {
        if let Some(path) = project.get_path() {
            *CURRENT_PROJECT_FILE.lock().unwrap() = Some(path);
        }
        project.clear_params();
        if let Err(e) = project.serialize(
            "project_dir",
            &self.project_dir.blocking_lock().as_deref().unwrap_or(""),
        ) {
            log::error!("Failed to set project parameter: {}", e);
        }
    }
}

impl Drop for Vi5Aux2 {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(task) = self.notification_task.blocking_lock().take() {
            task.abort();
        }
        let server = self.server.clone();
        let native = self.native.clone();
        if let Some(runtime) = self.runtime.write().unwrap().take() {
            // Include acquisition of the server lock in the deadline: an
            // initialization/render RPC may still own it when the host closes.
            runtime.block_on(async {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(7), async {
                    if let Some(client) = native.lock().await.take() {
                        client.shutdown().await;
                    }
                })
                .await;
                let stopped = tokio::time::timeout(std::time::Duration::from_secs(12), async {
                    let Some((child, mut client)) = server.lock().await.take() else {
                        return;
                    };
                    log::info!("Shutting down web-render-cef server...");
                    let graceful = tokio::time::timeout(std::time::Duration::from_secs(8), async {
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_secs(2),
                            client.shutdown(),
                        )
                        .await;
                        child.lock().await.wait().await
                    })
                    .await;
                    match graceful {
                        Ok(Ok(status)) => log::info!("CEF stopped: {status}"),
                        result => {
                            log::warn!("CEF graceful shutdown failed: {result:?}");
                            // try_wait polling releases this lock frequently.
                            // Bound even the fallback rather than hanging host exit.
                            let _ =
                                tokio::time::timeout(std::time::Duration::from_secs(2), async {
                                    let mut child = child.lock().await;
                                    child.kill().await?;
                                    child.wait().await
                                })
                                .await;
                        }
                    }
                })
                .await;
                if stopped.is_err() {
                    log::warn!("CEF shutdown deadline expired; stopping runtime tasks");
                }
            });
            log::info!("Shutting down Tokio runtime...");
            runtime.shutdown_timeout(std::time::Duration::from_secs(5));
        }
    }
}

aviutl2::register_generic_plugin!(Vi5Aux2);
