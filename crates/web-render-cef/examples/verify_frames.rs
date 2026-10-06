//! CEF end-to-end evidence. Uses the actual IPC-returned RGBA, not browser screenshots.
use std::{path::PathBuf, time::Duration};
use web_render_cef::{
    Client, FrameInfo, Parameter, ParameterValue, RenderRequest, RenderResponseData,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "Usage: verify_frames <cef-server.exe> <web-project> <evidence-directory>".into(),
        );
    }
    let executable = std::fs::canonicalize(PathBuf::from(&args[0]))?;
    let project = std::fs::canonicalize(PathBuf::from(&args[1]))?;
    let evidence = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&evidence)?;
    let listener = std::net::TcpListener::bind("[::1]:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    let mut command = tokio::process::Command::new(&executable);
    command
        .arg("--port")
        .arg(port.to_string())
        .arg("--parent-process")
        .arg(std::process::id().to_string())
        .current_dir(executable.parent().ok_or("Executable has no parent")?)
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut process = command.spawn()?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut client = loop {
        if let Some(exit) = process.try_wait()? {
            return Err(format!("CEF exited before readiness: {exit}").into());
        }
        match Client::connect(format!("http://[::1]:{port}")).await {
            Ok(client) => break client,
            Err(error) if tokio::time::Instant::now() >= deadline => return Err(error.into()),
            Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    };
    let info = client
        .initialize(project.to_string_lossy(), Some(Duration::from_secs(40)))
        .await?;
    println!(
        "Initialized project {} / {}",
        info.project_name, info.renderer_version
    );
    let mut images = std::collections::HashMap::new();
    let mut probe_request = None;
    let order = [0, 1, 30, 90, 120, 120, 90, 30, 1, 0, 90, 1, 120, 30, 0];
    for (index, frame) in order.into_iter().enumerate() {
        let request = RenderRequest {
            render_nonce: index as i32 + 1,
            object: "frame-proof".into(),
            object_id: 42,
            is_offline: true,
            parameters: vec![Parameter {
                key: "caption".into(),
                value: ParameterValue::Text("日本語\n\"quoted\" \\ path".into()),
            }],
            frame_info: FrameInfo {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                screen_width: 320,
                screen_height: 180,
                current_frame: frame,
                current_time: frame as f64 / (30000.0 / 1001.0),
                total_frames: 300,
                total_time: 300.0 / (30000.0 / 1001.0),
                framerate: 30000.0 / 1001.0,
                global_frame: frame + 100,
                global_time: (frame + 100) as f64 / (30000.0 / 1001.0),
            },
        };
        probe_request = Some(request.clone());
        let responses =
            tokio::time::timeout(Duration::from_secs(35), client.batch_render(vec![request]))
                .await??;
        let response = responses.into_iter().next().ok_or("No frame response")?;
        let RenderResponseData::Success {
            width,
            height,
            image_data,
        } = response.response
        else {
            return Err(format!("Frame {frame} failed: {:?}", response.response).into());
        };
        if (width, height) != (320, 180) {
            return Err(format!("Incorrect dimensions {width}x{height}").into());
        }
        let expected = [
            (frame % 251) as u8,
            (frame * 7 % 251) as u8,
            (frame * 13 % 251) as u8,
            255,
        ];
        // Verify both sides of the returned image, away from the metadata rows.
        for x in [40usize, 280] {
            let offset = (100 * 320 + x) * 4;
            if image_data[offset..offset + 4] != expected {
                return Err(format!(
                    "Frame {frame}: image pixel at x={x} is {:?}, expected {expected:?}",
                    &image_data[offset..offset + 4]
                )
                .into());
            }
        }
        // Independent barcodes on both edges must encode the requested frame.
        for bit in 0..8 {
            let value = if frame & (1 << bit) == 0 { 0 } else { 255 };
            for x in [8usize, 311] {
                let offset = ((bit * 16 + 8) * 320 + x) * 4;
                if image_data[offset..offset + 4] != [value, value, value, 255] {
                    return Err(
                        format!("Frame {frame}: incorrect barcode bit {bit} at x={x}").into(),
                    );
                }
            }
        }
        if let Some(previous) = images.get(&frame) {
            if previous != &image_data {
                return Err(format!("Frame {frame}: repeated RGBA differs").into());
            }
        } else {
            image::save_buffer(
                evidence.join(format!("cef-frame-{frame:04}.png")),
                &image_data,
                width as u32,
                height as u32,
                image::ColorType::Rgba8,
            )?;
            images.insert(frame, image_data);
        }
        println!("PASS request {} / frame {frame}", index + 1);
    }
    let mut probe = probe_request.ok_or("No probe request")?;
    probe.render_nonce = 1001;
    probe.object = "missing-object-for-validation".into();
    let failure = tokio::time::timeout(
        Duration::from_secs(35),
        client.batch_render(vec![probe.clone()]),
    )
    .await??;
    if !matches!(&failure[0].response, RenderResponseData::Error(message) if message.contains("Object not found"))
    {
        return Err("Missing object did not return a correlated error".into());
    }
    println!("PASS correlated scene error");
    probe.object = "frame-proof".into();
    probe.render_nonce = 1002;
    let recovered = tokio::time::timeout(
        Duration::from_secs(35),
        client.batch_render(vec![probe.clone()]),
    )
    .await??;
    if !matches!(&recovered[0].response, RenderResponseData::Success { image_data, .. } if image_data == &images[&0])
    {
        return Err("Frame after scene error differs".into());
    }
    println!("PASS frame after scene error");
    client
        .initialize(project.to_string_lossy(), Some(Duration::from_secs(40)))
        .await?;
    probe.render_nonce = 1003;
    let reinitialized =
        tokio::time::timeout(Duration::from_secs(35), client.batch_render(vec![probe])).await??;
    if !matches!(&reinitialized[0].response, RenderResponseData::Success { image_data, .. } if image_data == &images[&0])
    {
        return Err("Frame after project reinitialization differs".into());
    }
    println!("PASS project reinitialization");
    tokio::time::timeout(Duration::from_secs(5), client.shutdown()).await??;
    let status = match tokio::time::timeout(Duration::from_secs(10), process.wait()).await {
        Ok(status) => status?,
        Err(_) => {
            process.kill().await?;
            process.wait().await?;
            return Err("CEF did not exit within ten seconds after shutdown".into());
        }
    };
    if !status.success() {
        return Err(format!("CEF shutdown failed: {status}").into());
    }
    println!("PASS graceful shutdown");
    std::fs::write(
        evidence.join("cef-frame-results.txt"),
        format!(
            "CEF IPC RGBA: {} frame requests passed\nframes: {order:?}\nfps: 30000/1001\n320x180; full-color pixels + both edge barcodes + full RGBA equality\nCorrelated scene error: PASS\nFrame after error: PASS\nProject reinitialization: PASS\nGraceful shutdown (exit 0, within 10s): PASS\n",
            order.len()
        ),
    )?;
    Ok(())
}
