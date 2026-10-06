use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout},
};
use web_render_processing::{Project, Request, Response, VERSION};

pub static FINGERPRINT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn set_fingerprint(root: Option<&Path>) -> anyhow::Result<()> {
    use std::hash::{Hash, Hasher};
    let fingerprint = if let Some(root) = root {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        // This manifest contains backend version, AA/sampling, seed and all scene sources.
        std::fs::read(root.join(web_render_processing::MANIFEST))?.hash(&mut hash);
        (
            "libprocessing:0b1a55dd/noSmooth-v1/adapter-v1",
            VERSION,
            1u32,
            "rgba8-srgb-straight",
        )
            .hash(&mut hash);
        hash.finish()
    } else {
        0
    };
    FINGERPRINT.store(fingerprint, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

pub struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    pub project: Project,
}
impl Client {
    pub async fn start(root: &Path) -> anyhow::Result<Self> {
        let project = Project::load(root)?;
        let executable = process_path::get_dylib_path()
            .ok_or_else(|| anyhow::anyhow!("Missing module path"))?
            .with_file_name("web-render-processing-server.exe");
        let mut command = tokio::process::Command::new(&executable);
        command
            .arg(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW: worker has no visible console.
        let mut child = command.spawn()?;
        let input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let stderr = child.stderr.take().unwrap();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                aviutl2::tracing::debug!("libprocessing: {line}");
            }
        });
        let mut ready = String::new();
        tokio::time::timeout(Duration::from_secs(30), output.read_line(&mut ready)).await??;
        let ready: serde_json::Value = serde_json::from_str(&ready)?;
        anyhow::ensure!(
            ready["ready"] == VERSION && ready["project"] == project.name,
            "Native readiness mismatch"
        );
        aviutl2::tracing::info!(
            "libprocessing worker ready, pid={:?}, project={}",
            child.id(),
            project.name
        );
        Ok(Self {
            child,
            input,
            output,
            project,
        })
    }

    pub fn objects(&self) -> Vec<web_render_cef::ObjectInfo> {
        self.project
            .objects
            .iter()
            .map(|o| web_render_cef::ObjectInfo {
                id: o.id.clone(),
                label: o.label.clone(),
                parameter_definitions: vec![web_render_cef::ParameterDefinition {
                    key: "count".into(),
                    label: "個数".into(),
                    parameter_type: web_render_cef::ParameterType::Number {
                        step: 1.0.try_into().unwrap(),
                        min: 0.,
                        max: 5000.,
                    },
                    default_value: Some(web_render_cef::Parameter {
                        key: "count".into(),
                        value: web_render_cef::ParameterValue::Number(o.count as f64),
                    }),
                }],
            })
            .collect()
    }

    async fn render_one(
        &mut self,
        request: web_render_cef::RenderRequest,
    ) -> anyhow::Result<web_render_cef::RenderResponse> {
        anyhow::ensure!(
            self.child.try_wait()?.is_none(),
            "libprocessing worker exited; reload the project"
        );
        let count = self
            .project
            .objects
            .iter()
            .find(|o| o.id == request.object)
            .ok_or_else(|| anyhow::anyhow!("Unknown native object"))?
            .count;
        let count = match request
            .parameters
            .iter()
            .find(|p| p.key == "count")
            .map(|p| &p.value)
        {
            Some(web_render_cef::ParameterValue::Number(n))
                if n.is_finite() && *n >= 0. && *n <= 5000. =>
            {
                *n as u32
            }
            None => count,
            _ => anyhow::bail!("Invalid native count parameter"),
        };
        let native = Request {
            version: VERSION,
            nonce: request.render_nonce,
            object: request.object,
            object_id: request.object_id,
            width: request.frame_info.screen_width.try_into()?,
            height: request.frame_info.screen_height.try_into()?,
            time: request.frame_info.current_time,
            count,
        };
        native.validate()?;
        let start = std::time::Instant::now();
        self.input
            .write_all(format!("{}\n", serde_json::to_string(&native)?).as_bytes())
            .await?;
        self.input.flush().await?;
        let mut header = String::new();
        self.output.read_line(&mut header).await?;
        anyhow::ensure!(header.len() <= 16384, "Native response header too large");
        let response: Response = serde_json::from_str(&header)?;
        anyhow::ensure!(
            response.version == VERSION
                && response.nonce == native.nonce
                && response.width == native.width
                && response.height == native.height,
            "Native response mismatch"
        );
        if let Some(error) = response.error {
            anyhow::bail!("libprocessing: {error}");
        }
        web_render_cef::image::validate_rgba(
            response.width as usize,
            response.height as usize,
            response.bytes,
        )?;
        let mut image_data = vec![0; response.bytes];
        self.output.read_exact(&mut image_data).await?;
        aviutl2::tracing::info!(
            "Native render nonce={} object={} offline={} worker_ms={:.3} ipc_total_ms={:.3} created_graphics={}",
            native.nonce,
            native.object,
            request.is_offline,
            response.render_ms,
            start.elapsed().as_secs_f64() * 1000.,
            response.created_graphics
        );
        Ok(web_render_cef::RenderResponse {
            render_nonce: native.nonce,
            response: web_render_cef::RenderResponseData::Success {
                width: response.width as i32,
                height: response.height as i32,
                image_data,
            },
        })
    }

    pub async fn batch_render(
        &mut self,
        requests: Vec<web_render_cef::RenderRequest>,
    ) -> anyhow::Result<Vec<web_render_cef::RenderResponse>> {
        let mut responses = Vec::with_capacity(requests.len());
        for request in requests {
            match tokio::time::timeout(Duration::from_secs(30), self.render_one(request)).await {
                Ok(Ok(response)) => responses.push(response),
                result => {
                    // A cancelled/partial exchange loses framing. Never reuse that process.
                    let _ = self.child.kill().await;
                    return Err(anyhow::anyhow!(
                        "Native render exchange failed: {result:?}; reload the project"
                    ));
                }
            }
        }
        Ok(responses)
    }
    pub async fn shutdown(mut self) {
        let graceful = tokio::time::timeout(Duration::from_secs(5), async {
            self.input.write_all(b"shutdown\n").await?;
            self.input.flush().await?;
            self.child.wait().await
        })
        .await;
        if !matches!(graceful, Ok(Ok(_))) {
            let _ = self.child.kill().await;
        }
    }
}
