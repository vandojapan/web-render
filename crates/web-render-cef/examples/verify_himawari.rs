//! Exercise all three unchanged upstream objects through real CEF CPU images.
use std::{collections::HashMap, path::PathBuf, time::Duration};
use web_render_cef::{
    Client, FrameInfo, Parameter, ParameterValue, RenderRequest, RenderResponseData,
};

type Image = (u32, u32, Vec<u8>);

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(
        (3..=4).contains(&args.len()),
        "verify_himawari <cef-server.exe> <web-project> <proof-directory> [stress-requests]"
    );
    let stress_requests: usize = args
        .get(3)
        .map(|s| s.to_string_lossy().parse())
        .transpose()?
        .unwrap_or(0);
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
        .current_dir(executable.parent().unwrap())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut process = command.spawn()?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut client = loop {
        anyhow::ensure!(process.try_wait()?.is_none(), "CEF exited before readiness");
        match Client::connect(format!("http://[::1]:{port}")).await {
            Ok(client) => break client,
            Err(error) if tokio::time::Instant::now() >= deadline => return Err(error.into()),
            Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    };
    let initialized = client
        .initialize(project.to_string_lossy(), Some(Duration::from_secs(40)))
        .await?;
    println!("Initialized {}", initialized.project_name);
    let mut notifications = client.subscribe_notifications().await?;
    let notification_path = proof.join("notifications.txt");
    let notification_task = tokio::spawn(async move {
        use std::io::Write;
        let mut file = std::fs::File::create(notification_path)?;
        while let Some(notification) = notifications.message().await? {
            writeln!(file, "{notification:?}")?;
        }
        Ok::<(), anyhow::Error>(())
    });
    let order = [
        0, 3, 9, 15, 24, 30, 45, 48, 51, 54, 57, 60, 63, 117, 120, 237, 240, 243, 252, 288, 336,
        429, 432, 450, 15, 63, 240, 450, 0,
    ];
    let mut results = vec![];
    let mut nonce = 0;
    let mut issues = vec![];
    let mut stress_reference = HashMap::new();
    for (object, width, height) in [
        ("drum", 64, 184),
        ("kaiwai-phrase", 64, 72),
        ("synth-solo", 640, 144),
    ] {
        let mut images: HashMap<usize, Image> = HashMap::new();
        let mut frames = vec![];
        let mut differences = vec![];
        for frame in order {
            nonce += 1;
            let started = std::time::Instant::now();
            let image = capture(&mut client, request(nonce, object, frame)).await?;
            let render_ms = started.elapsed().as_secs_f64() * 1000.0;
            anyhow::ensure!(
                (image.0, image.1) == (width, height),
                "{object}: wrong canvas dimensions"
            );
            let painted = image.2.chunks_exact(4).filter(|p| p[3] > 0).count();
            let alpha_sum: usize = image.2.chunks_exact(4).map(|p| p[3] as usize).sum();
            let repeated = images.contains_key(&frame);
            if let Some(previous) = images.get(&frame) {
                if previous != &image {
                    let count = previous
                        .2
                        .chunks_exact(4)
                        .zip(image.2.chunks_exact(4))
                        .filter(|(a, b)| a != b)
                        .count();
                    differences.push(serde_json::json!({"frame":frame,"different_pixels":count}));
                    save(
                        &proof.join(format!("{object}-repeat-{frame:04}.png")),
                        &image,
                    )?;
                }
            } else {
                save(
                    &proof.join(format!("{object}-frame-{frame:04}.png")),
                    &image,
                )?;
                images.insert(frame, image);
            }
            frames.push(serde_json::json!({"frame":frame,"global_time":frame as f64/30.0,"painted_pixels":painted,"alpha_sum":alpha_sum,"repeated":repeated,"render_ms":render_ms}));
        }
        if !differences.is_empty() {
            issues.push(format!("{object}: reverse seek pixel differences"));
        }
        anyhow::ensure!(
            images[&15].2.iter().skip(3).step_by(4).any(|a| *a > 0),
            "{object}: empty active frame"
        );
        let mut checks = serde_json::Map::new();
        if object == "kaiwai-phrase" {
            let alpha = |frame: usize| -> usize {
                images[&frame]
                    .2
                    .iter()
                    .skip(3)
                    .step_by(4)
                    .map(|a| *a as usize)
                    .sum()
            };
            let fade = alpha(48) > alpha(54) && alpha(54) > 0 && alpha(57) == 0;
            let restart = alpha(60) > 0 && alpha(117) == 0 && alpha(120) > 0;
            checks.insert("phrase_fade".into(), fade.into());
            checks.insert("phrase_section_restart".into(), restart.into());
            if !fade || !restart {
                issues.push("Kaiwai phrase fade/section check failed".into());
            }
        }
        if object == "synth-solo" {
            let at = |frame: usize, y: usize| images[&frame].2[(y * 640 + 300) * 4 + 3];
            let bends = at(9, 44) == 255
                && at(9, 56) == 0
                && at(15, 56) == 255
                && at(15, 44) == 0
                && at(24, 50) == 255;
            checks.insert("pitch_bend_up_down_reset".into(), bends.into());
            if !bends {
                issues.push("Synth pitch bend geometry check failed".into());
            }
            let last_empty = images[&450].2.iter().skip(3).step_by(4).all(|a| *a == 0);
            checks.insert(
                "past_notes_remain_until_scroll_out".into(),
                (!last_empty).into(),
            );
            // Past notes stay visible until they have scrolled beyond the view.
            nonce += 1;
            let tail = capture(&mut client, request(nonce, object, 480)).await?;
            let tail_empty = tail.2.iter().skip(3).step_by(4).all(|a| *a == 0);
            checks.insert("after_scroll_out_transparent".into(), tail_empty.into());
            if !tail_empty {
                issues.push("Synth did not clear after scroll out".into());
            }
        }
        if object != "drum" {
            nonce += 1;
            let mut offset = request(nonce, object, 15);
            offset.frame_info.current_frame = 0;
            offset.frame_info.current_time = 0.0;
            let offset_image = capture(&mut client, offset).await?;
            let same = offset_image == images[&15];
            checks.insert("global_time_with_local_zero".into(), same.into());
            if !same {
                issues.push(format!("{object}: global time mismatch"));
            }
        }
        if object == "drum" {
            nonce += 1;
            let mut flipped = request(nonce, object, 15);
            flipped.parameters[0].value = ParameterValue::Bool(true);
            let flipped = capture(&mut client, flipped).await?;
            let changed = flipped != images[&15];
            checks.insert("flip_changes_image".into(), changed.into());
            save(&proof.join("drum-flipped.png"), &flipped)?;
            if !changed {
                issues.push("Drum Flip produced identical image".into());
            }
        }
        if object == "synth-solo" {
            nonce += 1;
            let mut moved = request(nonce, object, 15);
            moved.parameters[0].value = ParameterValue::Number(160.0);
            let moved = capture(&mut client, moved).await?;
            let changed = moved != images[&15];
            checks.insert("playhead_x_changes_image".into(), changed.into());
            save(&proof.join("synth-moved-playhead.png"), &moved)?;
            if !changed {
                issues.push("Synth Playhead X produced identical image".into());
            }
        }
        // Reinitialize to detect differences hidden by per-instance state.
        client
            .initialize(project.to_string_lossy(), Some(Duration::from_secs(40)))
            .await?;
        nonce += 1;
        let fresh = capture(&mut client, request(nonce, object, 15)).await?;
        let same = fresh == images[&15];
        checks.insert("reinitialization_full_rgba".into(), same.into());
        if !same {
            issues.push(format!("{object}: reinitialization mismatch"));
        }
        println!(
            "{object}: {} frames, reverse seek={}, checks={checks:?}",
            order.len(),
            differences.is_empty()
        );
        let mut times: Vec<_> = frames
            .iter()
            .skip(1)
            .map(|f| f["render_ms"].as_f64().unwrap())
            .collect();
        times.sort_by(f64::total_cmp);
        let median = times[times.len() / 2];
        let p95 = times[(times.len() as f64 * 0.95).ceil() as usize - 1];
        println!("{object}: steady IPC median={median:.2}ms, p95={p95:.2}ms (setup excluded)");
        results.push(serde_json::json!({"object":object,"width":width,"height":height,"frames":frames,"reverse_seek_full_rgba":differences.is_empty(),"differences":differences,"checks":checks,"timing":{"steady_samples":times.len(),"median_ms":median,"p95_ms":p95,"includes":"IPC + CEF paint and RGBA copy; excludes PNG encoding and first setup"}}));
        if object == "synth-solo" {
            stress_reference = images;
        }
    }
    // Span multiple parent-watch intervals while checking every complete RGBA image.
    // Keep encoding and equality checks outside the timed IPC interval.
    let mut stress = vec![];
    for index in 0..stress_requests {
        let frame = [15, 240, 450, 0][index % 4];
        nonce += 1;
        let started = std::time::Instant::now();
        let image = capture(&mut client, request(nonce, "synth-solo", frame)).await?;
        let render_ms = started.elapsed().as_secs_f64() * 1000.0;
        anyhow::ensure!(
            image == stress_reference[&frame],
            "Stress image mismatch at request {index}, frame {frame}"
        );
        stress.push(serde_json::json!({"frame":frame,"render_ms":render_ms}));
    }
    // A live notification stream prevents tonic's graceful server shutdown.
    // Drop it before requesting exit, rather than terminating the CEF process.
    notification_task.abort();
    let _ = notification_task.await;
    tokio::time::timeout(Duration::from_secs(10), client.shutdown()).await??;
    let exit = tokio::time::timeout(Duration::from_secs(10), process.wait()).await??;
    anyhow::ensure!(exit.success(), "CEF did not shut down successfully");
    std::fs::write(
        proof.join("result.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "status":if issues.is_empty() {"passed"} else {"issues_detected"},
            "objects":results,"issues":issues,"image_requests":nonce,"graceful_shutdown":true,
            "stress":stress,
            "p5_version":"2.3.2","upstream_commit":"378ea63dd998bb579f8d3ca74c19f9560584a6e3",
        }))?,
    )?;
    println!("Completed: {nonce} images, issues={issues:?}, graceful shutdown=PASS");
    Ok(())
}

fn request(nonce: i32, object: &str, frame: usize) -> RenderRequest {
    let parameters = match object {
        "drum" => vec![Parameter {
            key: "flip".into(),
            value: ParameterValue::Bool(false),
        }],
        "synth-solo" => vec![Parameter {
            key: "playheadX".into(),
            value: ParameterValue::Number(320.0),
        }],
        _ => vec![],
    };
    RenderRequest {
        render_nonce: nonce,
        object: object.into(),
        object_id: match object {
            "drum" => 42,
            "kaiwai-phrase" => 43,
            _ => 44,
        },
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
            total_frames: 480,
            total_time: 16.0,
            framerate: 30.0,
            global_frame: frame,
            global_time: frame as f64 / 30.0,
        },
    }
}

async fn capture(client: &mut Client, request: RenderRequest) -> anyhow::Result<Image> {
    let object = request.object.clone();
    let nonce = request.render_nonce;
    let mut responses =
        tokio::time::timeout(Duration::from_secs(35), client.batch_render(vec![request])).await??;
    anyhow::ensure!(
        responses.len() == 1 && responses[0].render_nonce == nonce,
        "Uncorrelated response"
    );
    match responses.remove(0).response {
        RenderResponseData::Success {
            width,
            height,
            image_data,
        } => {
            anyhow::ensure!(
                width > 0 && height > 0 && image_data.len() == width as usize * height as usize * 4,
                "Invalid dimensions or RGBA length"
            );
            Ok((width as u32, height as u32, image_data))
        }
        RenderResponseData::Error(message) => anyhow::bail!("{object}: {message}"),
    }
}

fn save(path: &std::path::Path, image: &Image) -> anyhow::Result<()> {
    image::save_buffer(path, &image.2, image.0, image.1, image::ColorType::Rgba8)?;
    Ok(())
}
