//! Equivalent Canvas scenes measured through the real CEF IPC and RGBA readback.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use web_render_cef::{
    Client, FrameInfo, Parameter, ParameterValue, RenderRequest, RenderResponseData,
};
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("benchmark_native_scenes <cef-server.exe> <project> <proof-dir>".into());
    }
    let executable = std::fs::canonicalize(PathBuf::from(&args[0]))?;
    let project = std::fs::canonicalize(PathBuf::from(&args[1]))?;
    let proof = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&proof)?;
    let listener = std::net::TcpListener::bind("[::1]:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    let mut command = tokio::process::Command::new(&executable);
    command
        .arg("--port")
        .arg(port.to_string())
        .arg("--parent-process")
        .arg(std::process::id().to_string())
        .current_dir(executable.parent().ok_or("no executable directory")?)
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let start = Instant::now();
    let mut process = command.spawn()?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut client = loop {
        if let Some(status) = process.try_wait()? {
            return Err(format!("CEF exited: {status}").into());
        }
        match Client::connect(format!("http://[::1]:{port}")).await {
            Ok(client) => break client,
            Err(e) if Instant::now() >= deadline => return Err(e.into()),
            _ => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    };
    let info = client
        .initialize(project.to_string_lossy(), Some(Duration::from_secs(40)))
        .await?;
    let init_ms = start.elapsed().as_secs_f64() * 1000.;
    let mut nonce = 0;
    let mut results = Vec::new();
    for (w, h) in [(1920, 1080), (3840, 2160)] {
        for scene in ["shapes", "text", "image"] {
            let mut samples = Vec::new();
            let mut cached = Vec::new();
            let mut last = Vec::new();
            let mut first = 0.;
            for frame in 0..70 {
                nonce += 1;
                let mut request = RenderRequest {
                    render_nonce: nonce,
                    object: format!("{scene}-{w}"),
                    object_id: 42,
                    is_offline: true,
                    parameters: vec![Parameter {
                        key: "scene".into(),
                        value: ParameterValue::Str(scene.into()),
                    }],
                    frame_info: FrameInfo {
                        x: 0.,
                        y: 0.,
                        z: 0.,
                        screen_width: w,
                        screen_height: h,
                        current_frame: frame,
                        current_time: frame as f64 / 60.,
                        total_frames: 1000,
                        total_time: 1000. / 60.,
                        framerate: 60.,
                        global_frame: frame,
                        global_time: frame as f64 / 60.,
                    },
                };
                let t = Instant::now();
                let response = tokio::time::timeout(
                    Duration::from_secs(30),
                    client.batch_render(vec![request.clone()]),
                )
                .await??
                .pop()
                .ok_or("no response")?;
                let ms = t.elapsed().as_secs_f64() * 1000.;
                let RenderResponseData::Success {
                    width,
                    height,
                    image_data,
                } = response.response
                else {
                    return Err(format!("render error {:?}", response.response).into());
                };
                if (width, height) != (w as i32, h as i32) {
                    return Err(format!("wrong dimensions {width}x{height}").into());
                }
                if frame == 0 {
                    first = ms;
                }
                if frame >= 10 {
                    samples.push(ms);
                }
                last = image_data;
                // Same-frame redraws check determinism. The aux2 frame cache
                // is above this direct IPC client, so these are not cache hits.
                if frame >= 60 {
                    nonce += 1;
                    request.render_nonce = nonce;
                    let t = Instant::now();
                    let result = tokio::time::timeout(
                        Duration::from_secs(30),
                        client.batch_render(vec![request]),
                    )
                    .await??;
                    cached.push(t.elapsed().as_secs_f64() * 1000.);
                    if !matches!(&result[0].response,RenderResponseData::Success{image_data,..} if image_data==&last)
                    {
                        return Err("cached image mismatch".into());
                    }
                }
            }
            std::fs::write(proof.join(format!("cef-{w}-{scene}.rgba")), &last)?;
            let rows = samples
                .iter()
                .map(|ms| format!("{ms}\n"))
                .collect::<String>();
            std::fs::write(
                proof.join(format!("cef-{w}-{scene}-samples.csv")),
                format!("total_ipc_ms\n{rows}"),
            )?;
            samples.sort_by(f64::total_cmp);
            cached.sort_by(f64::total_cmp);
            let median = samples[samples.len() / 2];
            let p95 = samples[(samples.len() as f64 * 0.95).ceil() as usize - 1];
            results.push(serde_json::json!({"width":w,"height":h,"scene":scene,"samples":60,"median_ms":median,"p95_ms":p95,"first_frame_ms":first,"same_frame_samples":cached.len(),"same_frame_redraw_median_ms":cached[cached.len()/2],"host_frame_cache_present":false}));
            println!("CEF {w} {scene}: median {median:.3}ms p95 {p95:.3}ms");
        }
    }
    client.shutdown().await?;
    let status = match tokio::time::timeout(Duration::from_secs(10), process.wait()).await {
        Ok(status) => status?,
        Err(_) => {
            process.kill().await?;
            process.wait().await?;
            return Err("CEF did not shut down".into());
        }
    };
    if !status.success() {
        return Err(format!("CEF exit {status}").into());
    }
    std::fs::write(
        proof.join("cef-result.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"status":"passed","renderer":info.renderer_version,"init_ms":init_ms,"includes":"JS, CEF CPU paint/readback, compression, IPC, decoding; PNG encoding excluded","results":results,"graceful_shutdown":true}),
        )?,
    )?;
    Ok(())
}
