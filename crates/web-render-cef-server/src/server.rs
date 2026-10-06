use std::pin::Pin;
use std::sync::Arc;

use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

pub struct MainServer {
    render_loop: crate::render_loop::RenderLoop,
    render_gate: tokio::sync::Mutex<()>,
    processes: tokio::sync::Mutex<Vec<tokio::process::Child>>,
    shutdown_tx: tokio::sync::Mutex<Option<Arc<tokio::sync::mpsc::UnboundedSender<()>>>>,
}

#[tonic::async_trait]
impl crate::protocol::libserver::lib_server_server::LibServer for MainServer {
    type SubscribeNotificationsStream = Pin<
        Box<
            dyn futures::Stream<
                    Item = Result<crate::protocol::libserver::Notification, tonic::Status>,
                > + Send
                + 'static,
        >,
    >;

    async fn initialize(
        &self,
        request: tonic::Request<crate::protocol::libserver::InitializeRequest>,
    ) -> Result<tonic::Response<crate::protocol::libserver::InitializeResponse>, tonic::Status>
    {
        let _render_guard = self.render_gate.lock().await;
        let req = request.into_inner();

        let mut processes_guard = self.processes.lock().await;
        if !processes_guard.is_empty() {
            for mut process in processes_guard.drain(..) {
                match process.kill().await {
                    Ok(_) => {
                        tracing::info!(
                            "Successfully killed existing vi5 process with PID: {}",
                            process.id().unwrap_or(0)
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to kill existing vi5 process with PID: {}: {}",
                            process.id().unwrap_or(0),
                            e
                        );
                    }
                }
            }
        }

        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| tonic::Status::internal(format!("Cannot allocate Vite port: {e}")))?;
        let random_port = listener
            .local_addr()
            .map_err(|e| tonic::Status::internal(e.to_string()))?
            .port();
        drop(listener);
        tracing::info!("Received initialize request: {:?}", req);
        // Node 24 cannot resolve an extended-length \\?\ drive path as its main script.
        // Keep Unicode/space paths intact while converting only the Windows prefix.
        let root_path = if let Some(unc) = req.root_path.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc}")
        } else {
            req.root_path
                .strip_prefix(r"\\?\")
                .unwrap_or(&req.root_path)
                .to_string()
        };
        let path = std::path::Path::new(&root_path);
        let cli = path
            .join("node_modules")
            .join("web-render")
            .join("dist")
            .join("cli.mjs");
        if !cli.is_file() {
            return Err(tonic::Status::failed_precondition(format!(
                "Missing web-render runtime {}; run npm install in this project",
                cli.display()
            )));
        }
        let mut command = tokio::process::Command::new("node");
        command
            .arg(cli)
            .arg("start")
            .arg("--port")
            .arg(random_port.to_string())
            .current_dir(path)
            .env("WEB_RENDER_PARENT_PID", std::process::id().to_string())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let mut process = command
            .spawn()
            .map_err(|e| tonic::Status::internal(format!("Failed to start vi5 process: {}", e)))?;
        tracing::info!(
            "Started vi5 process with PID: {}",
            process.id().unwrap_or(0)
        );
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .map_err(|e| tonic::Status::internal(e.to_string()))?;
        let url = format!("http://localhost:{random_port}/vi5");
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            if let Some(code) = process
                .try_wait()
                .map_err(|e| tonic::Status::internal(e.to_string()))?
            {
                return Err(tonic::Status::internal(format!(
                    "Vite exited before readiness: {code}"
                )));
            }
            if let Ok(response) = http.get(&url).send().await
                && response.status().is_success()
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(tonic::Status::deadline_exceeded("Vite readiness timed out"));
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        let response = self
            .render_loop
            .initialize(&url)
            .await
            .map_err(|e| tonic::Status::internal(format!("Initialization failed: {}", e)))?;
        processes_guard.push(process);
        let response = crate::protocol::libserver::InitializeResponse {
            project_name: response.project_name,
            renderer_version: response.renderer_version,
        };

        tracing::info!("Initialization completed: {:?}", response);

        Ok(tonic::Response::new(response))
    }

    async fn batch_render(
        &self,
        request: tonic::Request<crate::protocol::common::BatchRenderRequest>,
    ) -> Result<tonic::Response<crate::protocol::libserver::BatchRenderResponse>, tonic::Status>
    {
        let _render_guard = self.render_gate.lock().await;
        let req = request.into_inner();
        tracing::info!("Received batch render request: {:?}", req);
        let mut render_results = crate::protocol::libserver::BatchRenderResponse {
            render_responses: Vec::with_capacity(req.render_requests.len()),
        };
        for render_request in req.render_requests {
            let response = self
                .render_loop
                .batch_render(crate::protocol::common::BatchRenderRequest {
                    render_requests: vec![render_request],
                })
                .await
                .map_err(|e| tonic::Status::internal(format!("Frame render failed: {}", e)))?;
            render_results
                .render_responses
                .extend(response.render_responses);
        }
        let compress = crate::compression::should_compress(&render_results);
        let mut response = tonic::Response::new(render_results);
        if !compress {
            response.disable_compression();
        }
        Ok(response)
    }

    async fn purge_cache(
        &self,
        _request: tonic::Request<crate::protocol::common::Void>,
    ) -> Result<tonic::Response<crate::protocol::common::Void>, tonic::Status> {
        let _render_guard = self.render_gate.lock().await;
        tracing::info!("Received purge cache request");
        self.render_loop
            .purge_cache()
            .await
            .map_err(|e| tonic::Status::internal(format!("Purge cache failed: {}", e)))?;
        Ok(tonic::Response::new(crate::protocol::common::Void {}))
    }

    async fn subscribe_notifications(
        &self,
        _request: tonic::Request<crate::protocol::common::Void>,
    ) -> Result<tonic::Response<Self::SubscribeNotificationsStream>, tonic::Status> {
        let (backlog, rx) = self.render_loop.subscribe_notifications();
        let backlog_stream = futures::stream::iter(backlog.into_iter().map(Ok));
        let live_stream = BroadcastStream::new(rx).filter_map(|item| async move {
            match item {
                Ok(notification) => Some(Ok(notification)),
                Err(err) => {
                    tracing::warn!("Notification stream lagged: {}", err);
                    None
                }
            }
        });
        let stream = backlog_stream.chain(live_stream);
        Ok(tonic::Response::new(Box::pin(stream)))
    }

    async fn shutdown(
        &self,
        _request: tonic::Request<crate::protocol::common::Void>,
    ) -> Result<tonic::Response<crate::protocol::common::Void>, tonic::Status> {
        tracing::info!("Received shutdown request");
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
        Ok(tonic::Response::new(crate::protocol::common::Void {}))
    }
}

impl MainServer {
    pub fn new(
        render_loop: crate::render_loop::RenderLoop,
        shutdown_tx: Arc<tokio::sync::mpsc::UnboundedSender<()>>,
    ) -> Self {
        Self {
            render_loop,
            render_gate: tokio::sync::Mutex::new(()),
            processes: tokio::sync::Mutex::new(Vec::new()),
            shutdown_tx: tokio::sync::Mutex::new(Some(shutdown_tx)),
        }
    }
}

impl Drop for MainServer {
    fn drop(&mut self) {
        for mut process in futures::executor::block_on(self.processes.lock()).drain(..) {
            match futures::executor::block_on(process.kill()) {
                Ok(_) => {
                    tracing::info!(
                        "Successfully killed vi5 process with PID: {}",
                        process.id().unwrap_or(0)
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to kill vi5 process with PID: {}: {}",
                        process.id().unwrap_or(0),
                        e
                    );
                }
            }
        }
    }
}
