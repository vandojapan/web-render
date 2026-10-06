//! Real CEF images for the explicitly supplied MIDI Pattern Grid p5 object.
use std::{collections::HashMap, path::PathBuf, time::Duration};
use web_render_cef::{
    Client, FrameInfo, Parameter, ParameterValue, RenderRequest, RenderResponseData,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(
        args.len() == 3,
        "verify_p5_midi <cef-server.exe> <web-project> <evidence-directory>"
    );
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
        .current_dir(executable.parent().unwrap())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut process = command.spawn()?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut client = loop {
        if let Some(status) = process.try_wait()? {
            anyhow::bail!("CEF exited before readiness: {status}");
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
    println!("Initialized {}", info.project_name);
    let mut images: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut rows = Vec::new();
    let mut repeated_differences = Vec::new();
    // At 30fps: eighth steps 7, 8, 12, 16 and 23 surround the tempo/row boundaries.
    let order = [0, 53, 60, 85, 108, 150, 85, 60, 0, 108];
    for (index, frame) in order.into_iter().enumerate() {
        let request = request(index as i32 + 1, frame);
        let rgba = capture(&mut client, request).await?;
        let counts = cell_coverage(&rgba);
        let eighth = if frame < 60 {
            frame as f64 / 7.5
        } else {
            8.0 + (frame - 60) as f64 / 6.0
        };
        let step = eighth.floor() as usize;
        let current_group = step / 8;
        for row in 0..2 {
            for column in 0..8 {
                let note_step = if row == 0 {
                    current_group * 8 + column
                } else {
                    current_group.saturating_sub(1) * 8 + column
                };
                let visible = if row == 0 {
                    column <= step % 8 && note_step < 24
                } else {
                    current_group > 0 && note_step < 24
                };
                anyhow::ensure!(
                    (counts[row * 8 + column] > 0) == visible,
                    "frame {frame}, row {row}, column {column}: coverage {}, expected visible={visible}",
                    counts[row * 8 + column]
                );
            }
        }
        anyhow::ensure!(
            rgba[..4] == [0, 0, 0, 0],
            "Transparent border or metadata crop is incorrect"
        );
        if let Some(previous) = images.get(&frame) {
            if previous != &rgba {
                image::save_buffer(
                    evidence.join(format!("repeat-mismatch-{frame:04}.png")),
                    &rgba,
                    824,
                    200,
                    image::ColorType::Rgba8,
                )?;
                let pixels = previous
                    .chunks_exact(4)
                    .zip(rgba.chunks_exact(4))
                    .filter(|(a, b)| a != b)
                    .count();
                repeated_differences
                    .push(serde_json::json!({"frame":frame,"different_pixels":pixels}));
                println!("ISSUE repeated frame {frame}: {pixels} different pixels");
            }
        } else {
            image::save_buffer(
                evidence.join(format!("p5-frame-{frame:04}.png")),
                &rgba,
                824,
                200,
                image::ColorType::Rgba8,
            )?;
            images.insert(frame, rgba);
        }
        rows.push(serde_json::json!({"frame":frame,"time":frame as f64 / 30.0,"eighth_position":eighth,"coverage":counts}));
        println!("PASS frame {frame}, eighth={eighth}, cell coverage={counts:?}");
    }
    // Same pitch/velocity in the previous row at steps 8 and 12: Bayer fade
    // must remove approximately half the painted pixels, including antialiasing.
    let full = cell_coverage(&images[&60]);
    let half = cell_coverage(&images[&85]);
    let full_total: usize = full[8..].iter().sum();
    let half_total: usize = half[8..].iter().sum();
    let ratio = half_total as f64 / full_total as f64;
    anyhow::ensure!(
        (0.40..0.60).contains(&ratio),
        "Previous row fade coverage is not half: {ratio}"
    );
    // Global Sync is equivalent to local time when its global time is the same.
    let mut global = request(100, 0);
    global.frame_info.global_time = 85.0 / 30.0;
    global.frame_info.global_frame = 85;
    global
        .parameters
        .iter_mut()
        .find(|p| p.key == "syncGlobal")
        .unwrap()
        .value = ParameterValue::Bool(true);
    let synced = capture(&mut client, global).await?;
    let global_sync = synced == images[&85];
    image::save_buffer(
        evidence.join("p5-global-sync.png"),
        &synced,
        824,
        200,
        image::ColorType::Rgba8,
    )?;
    // Diagnose the supplied script's floating-point boundary separately;
    // do not hide this one-frame discrepancy by calling the entire test passed.
    let boundary = capture(&mut client, request(102, 84)).await?;
    let boundary_coverage = cell_coverage(&boundary);
    let boundary_issue = boundary_coverage[4] == 0;
    image::save_buffer(
        evidence.join("p5-frame-0084-boundary.png"),
        &boundary,
        824,
        200,
        image::ColorType::Rgba8,
    )?;
    println!("BOUNDARY frame 84: missing step 12={boundary_issue}, coverage={boundary_coverage:?}");
    client
        .initialize(project.to_string_lossy(), Some(Duration::from_secs(40)))
        .await?;
    let reinitialized = capture(&mut client, request(101, 60)).await?;
    let reinitialization = reinitialized == images[&60];
    tokio::time::timeout(Duration::from_secs(5), client.shutdown()).await??;
    let status = tokio::time::timeout(Duration::from_secs(10), process.wait()).await??;
    anyhow::ensure!(status.success(), "CEF did not exit successfully: {status}");
    std::fs::write(
        evidence.join("result.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "status":if boundary_issue || !repeated_differences.is_empty() || !global_sync || !reinitialization { "issues_detected" } else { "passed" },
            "frames":rows,"width":824,"height":200,
            "dither_half_coverage_ratio":ratio,"reverse_seek_full_rgba":repeated_differences.is_empty(),
            "repeated_differences":repeated_differences,
            "global_sync":global_sync,"reinitialization":reinitialization,"graceful_shutdown":true,
            "source_object":"midi-pattern-grid","midi":"generated-song.mid",
            "boundary_issue":{"frame":84,"time":2.8,"missing_step_12":boundary_issue,"coverage":boundary_coverage},
        }))?,
    )?;
    println!(
        "Completed: Bayer ratio={ratio}, Global Sync identical={global_sync}, reinitialization identical={reinitialization}, graceful shutdown=PASS"
    );
    Ok(())
}

fn request(nonce: i32, frame: usize) -> RenderRequest {
    let mut parameters = vec![
        Parameter {
            key: "midiPath".into(),
            value: ParameterValue::Str("/song.mid".into()),
        },
        Parameter {
            key: "syncGlobal".into(),
            value: ParameterValue::Bool(false),
        },
        Parameter {
            key: "ditherMode".into(),
            value: ParameterValue::Str("bayer4".into()),
        },
    ];
    for (key, value) in [
        ("track", 1.0),
        ("baseNote", 0.0),
        ("cellWidth", 88.0),
        ("cellHeight", 88.0),
        ("spacingX", 104.0),
        ("spacingY", 104.0),
        ("originX", 8.0),
        ("originY", 8.0),
        ("ditherScale", 1.0),
    ] {
        parameters.push(Parameter {
            key: key.into(),
            value: ParameterValue::Number(value),
        });
    }
    RenderRequest {
        render_nonce: nonce,
        object: "midi-pattern-grid".into(),
        object_id: 42,
        is_offline: true,
        parameters,
        frame_info: FrameInfo {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            screen_width: 1920,
            screen_height: 1080,
            current_frame: frame,
            current_time: frame as f64 / 30.0,
            total_frames: 240,
            total_time: 8.0,
            framerate: 30.0,
            global_frame: frame + 30,
            global_time: (frame + 30) as f64 / 30.0,
        },
    }
}

async fn capture(client: &mut Client, request: RenderRequest) -> anyhow::Result<Vec<u8>> {
    let nonce = request.render_nonce;
    let mut responses =
        tokio::time::timeout(Duration::from_secs(35), client.batch_render(vec![request])).await??;
    anyhow::ensure!(
        responses.len() == 1 && responses[0].render_nonce == nonce,
        "Mismatched frame response"
    );
    match responses.remove(0).response {
        RenderResponseData::Success {
            width,
            height,
            image_data,
        } => {
            anyhow::ensure!(
                (width, height) == (824, 200),
                "Unexpected p5 canvas dimensions: {width}x{height}"
            );
            anyhow::ensure!(image_data.len() == 824 * 200 * 4, "Incorrect image length");
            Ok(image_data)
        }
        RenderResponseData::Error(message) => anyhow::bail!("p5 render failed: {message}"),
    }
}

fn cell_coverage(rgba: &[u8]) -> [usize; 16] {
    let mut counts = [0; 16];
    for row in 0..2 {
        for column in 0..8 {
            for y in (8 + row * 104)..(96 + row * 104) {
                for x in (8 + column * 104)..(96 + column * 104) {
                    if rgba[(y * 824 + x) * 4 + 3] > 0 {
                        counts[row * 8 + column] += 1;
                    }
                }
            }
        }
    }
    counts
}
