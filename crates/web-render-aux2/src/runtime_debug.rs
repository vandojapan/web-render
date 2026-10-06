//! Opt-in SDK probe for a copied sample. Excluded from ordinary builds.
use crate::EDIT_HANDLE;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

static SCHEDULED: AtomicBool = AtomicBool::new(false);

pub fn enabled() -> bool {
    std::env::var_os("WEB_RENDER_DEBUG_OUTPUT").is_some()
        && std::env::var_os("WEB_RENDER_DEBUG_PROJECT").is_some()
}

pub fn schedule(project_name: String) {
    if !enabled() || SCHEDULED.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::task::spawn_blocking(move || {
        let result = run(&project_name);
        let output = PathBuf::from(std::env::var_os("WEB_RENDER_DEBUG_OUTPUT").unwrap());
        let report = match result {
            Ok(value) => value,
            Err(error) => serde_json::json!({"status":"error", "error": format!("{error:#}")}),
        };
        let _ = std::fs::create_dir_all(&output);
        let _ = std::fs::write(
            output.join("result.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        );
        aviutl2::tracing::info!("AviUtl2 SDK sample probe: {report}");
        close_own_host();
    });
}

fn run(project_name: &str) -> anyhow::Result<serde_json::Value> {
    let expected = PathBuf::from(std::env::var_os("WEB_RENDER_DEBUG_PROJECT").unwrap());
    let current = crate::CURRENT_PROJECT_FILE
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| anyhow::anyhow!("No project is loaded"))?;
    anyhow::ensure!(
        std::fs::canonicalize(&current)? == std::fs::canonicalize(&expected)?,
        "Debug probe refused a project other than the explicit sample copy"
    );
    let output = PathBuf::from(std::env::var_os("WEB_RENDER_DEBUG_OUTPUT").unwrap());
    std::fs::create_dir_all(&output)?;
    if std::env::var("WEB_RENDER_DEBUG_MODE").is_ok_and(|mode| mode.starts_with("native-")) {
        return run_native(&expected, &output, project_name);
    }
    if std::env::var("WEB_RENDER_DEBUG_MODE").is_ok_and(|mode| mode.starts_with("p5-")) {
        return run_p5(&expected, &output, project_name);
    }
    if std::env::var("WEB_RENDER_DEBUG_MODE").is_ok_and(|mode| mode.starts_with("himawari-")) {
        return run_himawari(&expected, &output, project_name);
    }
    let script = crate::get_script_dir(project_name).join("フレーム検証.obj2");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let script_ready = || {
        std::fs::read_to_string(&script).is_ok_and(|text| {
            text.lines().next() == Some("--text@WEB_RENDER_AUX2_caption:文字,Frame proof")
        })
    };
    while !script_ready() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    anyhow::ensure!(
        script_ready(),
        "Initial object notification did not generate {}",
        script.display()
    );
    if std::env::var("WEB_RENDER_DEBUG_MODE").as_deref() == Ok("bootstrap") {
        return Ok(serde_json::json!({"status":"bootstrap", "script":script}));
    }
    let effects = EDIT_HANDLE.get_effects();
    let matches: Vec<_> = effects
        .iter()
        .filter(|effect| effect.name.contains("フレーム検証"))
        .map(|effect| effect.name.clone())
        .collect();
    std::fs::write(
        output.join("effects.json"),
        serde_json::to_vec_pretty(&matches)?,
    )?;
    let effect = matches.first().ok_or_else(|| {
        anyhow::anyhow!("Generated script is not registered; restart after bootstrap")
    })?;
    let alias = format!(
        "[Object]\nframe=0,149\n[Object.0]\neffect.name={effect}\n[Object.1]\neffect.name=標準描画\n"
    );
    let probe_effect = effect.clone();
    let replay = std::env::var("WEB_RENDER_DEBUG_MODE").as_deref() == Ok("replay");
    EDIT_HANDLE.call_edit_section(move |section| -> anyhow::Result<()> {
        let object = if replay {
            let object = section
                .find_object_after(0, 0)?
                .ok_or_else(|| anyhow::anyhow!("Saved sample object is missing"))?;
            anyhow::ensure!(
                section.get_object_name(object)?.as_deref() == Some("Web Render frame probe"),
                "Refused an object other than the saved debug probe"
            );
            object
        } else {
            anyhow::ensure!(
                section.find_object_after(0, 0)?.is_none(),
                "Sample layer 0 already contains an object"
            );
            let object = section.create_object_from_alias(&alias, 0, 0, 150)?;
            section.set_object_name(object, Some("Web Render frame probe"))?;
            object
        };
        anyhow::ensure!(
            section.get_object_effect_item(object, &probe_effect, 0, "文字")? == "Frame proof",
            "Host text default includes unexpected escaping/quotes"
        );
        Ok(())
    })??;
    // Save the copied project before rendering or requesting a normal host close.
    EDIT_HANDLE.save_project_file(&expected)?;
    if std::env::var("WEB_RENDER_DEBUG_MODE").is_ok_and(|m| m.starts_with("movement")) {
        return run_movement(&expected, &output);
    }
    let mut images = std::collections::HashMap::new();
    let mut frames = Vec::new();
    for frame in [0u32, 90, 30, 90, 120, 1, 0] {
        let (width, height, rgba) = capture_scene(frame)?;
        image::save_buffer(
            output.join(format!("aviutl-frame-{frame:04}.png")),
            &rgba,
            width,
            height,
            image::ColorType::Rgba8,
        )?;
        anyhow::ensure!(
            width >= 320 && height >= 180,
            "Unexpected host image dimensions"
        );
        let left = (width as usize - 320) / 2;
        let top = (height as usize - 180) / 2;
        let expected_pixel = [
            (frame % 251) as u8,
            (frame * 7 % 251) as u8,
            (frame * 13 % 251) as u8,
            255,
        ];
        for x in [40, 280] {
            let offset = ((top + 100) * width as usize + left + x) * 4;
            anyhow::ensure!(
                rgba[offset..offset + 4] == expected_pixel,
                "Host frame {frame}: pixel {x} is {:?}, expected {expected_pixel:?}",
                &rgba[offset..offset + 4]
            );
        }
        for bit in 0..8 {
            let value = if frame & (1 << bit) == 0 { 0 } else { 255 };
            for x in [8, 311] {
                let offset = ((top + bit * 16 + 8) * width as usize + left + x) * 4;
                anyhow::ensure!(
                    rgba[offset..offset + 4] == [value, value, value, 255],
                    "Host frame {frame}: barcode mismatch"
                );
            }
        }
        if let Some(previous) = images.get(&frame) {
            anyhow::ensure!(&rgba == previous, "Repeated host frame {frame} differs");
        } else {
            images.insert(frame, rgba);
        }
        frames
            .push(serde_json::json!({"frame":frame,"width":width,"height":height,"verified":true}));
    }
    set_freeze(effect, true)?;
    for (index, frame) in [90u32, 30, 90].into_iter().enumerate() {
        let (width, height, rgba) = capture_scene(frame)?;
        anyhow::ensure!(
            rgba == images[&frame],
            "Freeze frame {frame} differs from the unfrozen image"
        );
        image::save_buffer(
            output.join(format!("freeze-{index}-frame-{frame:04}.png")),
            &rgba,
            width,
            height,
            image::ColorType::Rgba8,
        )?;
    }
    set_freeze(effect, false)?;
    let (_, _, rgba) = capture_scene(90)?;
    anyhow::ensure!(rgba == images[&90], "Image after unfreeze differs");
    EDIT_HANDLE.save_project_file(&expected)?;
    Ok(
        serde_json::json!({"status":"passed", "project":expected,"effect":effect,"frames":frames,
        "text_default":"Frame proof", "freeze_sequence":[90,30,90], "unfreeze":true, "replay":replay}),
    )
}

fn run_movement(
    expected: &std::path::Path,
    output: &std::path::Path,
) -> anyhow::Result<serde_json::Value> {
    let mut references = std::collections::HashMap::new();
    for local in [0, 1, 30, 90, 120, 149] {
        references.insert(local, capture_scene(local)?);
    }
    if std::env::var("WEB_RENDER_DEBUG_MODE").is_ok_and(|m| m.ends_with("movement-gui")) {
        EDIT_HANDLE.call_edit_section(|section| -> anyhow::Result<()> {
            section.set_cursor_layer_frame(0, 30)?;
            section.set_display_layer_frame(0, 0)?;
            Ok(())
        })??;
        std::fs::write(output.join("gui-ready.json"), "{}")?;
        let deadline = std::time::Instant::now() + Duration::from_secs(180);
        while !output.join("gui-continue.json").is_file() {
            anyhow::ensure!(std::time::Instant::now() < deadline, "GUI move timed out");
            std::thread::sleep(Duration::from_millis(100));
        }
        let start = EDIT_HANDLE.call_edit_section(|section| -> anyhow::Result<usize> {
            let object = section
                .find_object_after(0, 0)?
                .ok_or_else(|| anyhow::anyhow!("GUI object missing"))?;
            Ok(section.get_object_layer_frame(object)?.start)
        })??;
        let mut checks = Vec::new();
        for local in [149, 0, 120, 30, 1, 90] {
            let (w, h, rgba) = capture_scene(start as u32 + local)?;
            let reference = &references[&local];
            image::save_buffer(
                output.join(format!("gui-local-{local}.png")),
                &rgba,
                w,
                h,
                image::ColorType::Rgba8,
            )?;
            checks.push(serde_json::json!({"start":start,"local":local,"matches_reference":rgba==reference.2}));
        }
        EDIT_HANDLE.save_project_file(expected)?;
        return Ok(
            serde_json::json!({"status":if start>0 && checks.iter().all(|c| c["matches_reference"]==true) {"passed"} else {"failed"},"checks":checks}),
        );
    }
    let mut previous_layer = 0usize;
    let mut previous_start = 0usize;
    let mut checks = Vec::new();
    for (layer, start) in [(0usize, 60usize), (0, 300), (0, 10), (2, 20), (0, 0)] {
        EDIT_HANDLE.call_edit_section(move |section| -> anyhow::Result<()> {
            let object = section
                .find_object_after(previous_layer, previous_start)?
                .ok_or_else(|| anyhow::anyhow!("Movement probe object missing"))?;
            anyhow::ensure!(
                section.get_object_name(object)?.as_deref() == Some("Web Render frame probe"),
                "Refused another object"
            );
            section.move_object(object, layer, start)?;
            section.set_cursor_layer_frame(layer, start)?;
            Ok(())
        })??;
        for local in [149, 0, 120, 30, 1, 90] {
            let frame = start as u32 + local;
            let (w, h, rgba) = capture_scene(frame)?;
            let reference = &references[&local];
            let equal = (w, h) == (reference.0, reference.1) && rgba == reference.2;
            image::save_buffer(
                output.join(format!("layer-{layer}-start-{start}-local-{local}.png")),
                &rgba,
                w,
                h,
                image::ColorType::Rgba8,
            )?;
            checks.push(serde_json::json!({"layer":layer,"start":start,"local":local,"frame":frame,"matches_reference":equal}));
        }
        previous_layer = layer;
        previous_start = start;
    }
    let (w, h, reference) = &references[&30];
    for (x, y) in [(100i32, 0i32), (400, 240), (-400, -240), (0, 0)] {
        EDIT_HANDLE.call_edit_section(move |section| -> anyhow::Result<()> {
            let object = section
                .find_object_after(0, 0)?
                .ok_or_else(|| anyhow::anyhow!("Movement probe missing"))?;
            section.set_object_effect_item(object, "標準描画", 0, "X", &x.to_string())?;
            section.set_object_effect_item(object, "標準描画", 0, "Y", &y.to_string())?;
            Ok(())
        })??;
        let (_, _, rgba) = capture_scene(30)?;
        let mut translated = reference.clone();
        for row in 0..*h as i32 {
            for col in 0..*w as i32 {
                let dest = ((row as u32 * w + col as u32) * 4) as usize;
                let src_x = col - x;
                let src_y = row - y;
                let src = if (0..*w as i32).contains(&src_x) && (0..*h as i32).contains(&src_y) {
                    ((src_y as u32 * w + src_x as u32) * 4) as usize
                } else {
                    0
                };
                translated[dest..dest + 4].copy_from_slice(&reference[src..src + 4]);
            }
        }
        image::save_buffer(
            output.join(format!("position-{x}-{y}.png")),
            &rgba,
            *w,
            *h,
            image::ColorType::Rgba8,
        )?;
        checks.push(
            serde_json::json!({"x":x,"y":y,"frame":30,"matches_reference":rgba == translated}),
        );
    }
    EDIT_HANDLE.save_project_file(expected)?;
    let passed = checks.iter().all(|c| c["matches_reference"] == true);
    Ok(
        serde_json::json!({"status":if passed {"passed"} else {"failed"},"project":expected,"checks":checks}),
    )
}

fn capture_scene(frame: u32) -> anyhow::Result<(u32, u32, Vec<u8>)> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    EDIT_HANDLE.rendering_scene_video(frame, move |video| {
        // SDK callback memory is valid only during this callback. Copy it now.
        let copied = web_render_cef::image::packed_rows(
            video.buffer,
            video.width as usize,
            video.height as usize,
            video.pitch as usize,
        )
        .map(|rgba| (video.width, video.height, rgba));
        let _ = sender.send(copied);
    })?;
    Ok(receiver.recv_timeout(Duration::from_secs(45))??)
}

fn run_native(
    expected: &std::path::Path,
    output: &std::path::Path,
    project_name: &str,
) -> anyhow::Result<serde_json::Value> {
    let root = std::env::var_os("WEB_RENDER_DEBUG_NATIVE_ROOT")
        .ok_or_else(|| anyhow::anyhow!("Missing explicit native fixture"))?;
    let project = web_render_processing::Project::load(std::path::Path::new(&root))?;
    anyhow::ensure!(project.name == project_name, "Native project mismatch");
    for object in &project.objects {
        anyhow::ensure!(
            crate::get_script_dir(project_name)
                .join(format!("{}.obj2", object.label))
                .is_file(),
            "Native object script missing"
        );
    }
    let mode = std::env::var("WEB_RENDER_DEBUG_MODE")?;
    if mode == "native-bootstrap" {
        return Ok(
            serde_json::json!({"status":"bootstrap","project":project.name,"objects":project.objects.len()}),
        );
    }
    let replay = mode == "native-replay";
    let effects = EDIT_HANDLE.get_effects();
    for object in &project.objects {
        anyhow::ensure!(
            effects.iter().any(|e| e.name == object.label),
            "Restart to register {}",
            object.label
        );
    }
    if mode == "native-movement-gui" {
        EDIT_HANDLE.call_edit_section(|section| -> anyhow::Result<()> {
            anyhow::ensure!(section.find_object_after(0,0)?.is_none(), "Use an empty native sample copy");
            let alias = "[Object]\nframe=0,149\n[Object.0]\neffect.name=Native Image noSmooth\n[Object.1]\neffect.name=標準描画\n";
            let object = section.create_object_from_alias(alias,0,0,150)?;
            section.set_object_name(object,Some("Web Render frame probe"))?;
            Ok(())
        })??;
        EDIT_HANDLE.save_project_file(expected)?;
        return run_movement(expected, output);
    }
    if mode == "native-export" {
        let frame = 435u32;
        EDIT_HANDLE.call_edit_section(|section| -> anyhow::Result<()> {
            section.set_cursor_layer_frame(0, frame as usize)?;
            section.set_select_range(frame as usize, frame as usize)?;
            Ok(())
        })??;
        let (w, h, rgba) = capture_scene(frame)?;
        image::save_buffer(
            output.join("preview.png"),
            &rgba,
            w,
            h,
            image::ColorType::Rgba8,
        )?;
        let export = export_native_png(output, &rgba)?;
        return Ok(
            serde_json::json!({"status":"passed","offline_export":export,"offline_matches_preview":true,"frame":frame}),
        );
    }
    let objects = project.objects.clone();
    EDIT_HANDLE.call_edit_section(move |section|->anyhow::Result<()> {
        for (i,object) in objects.iter().enumerate() {
            let start=i*60;
            let name=format!("{} native test",object.label);
            if replay {
                let existing=section.find_object_after(0,start)?.ok_or_else(||anyhow::anyhow!("Saved native object missing"))?;
                anyhow::ensure!(section.get_object_name(existing)?.as_deref()==Some(&name),"Refused another saved object");
            } else {
                anyhow::ensure!(section.find_object_after(0,start)?.is_none(),"Use an empty native sample copy");
                let alias=format!("[Object]\nframe={start},{}\n[Object.0]\neffect.name={}\nBatch Size=1\n[Object.1]\neffect.name=標準描画\n",start+59,object.label);
                let handle=section.create_object_from_alias(&alias,0,start,60)?;
                section.set_object_name(handle,Some(&name))?;
            }
        }
        section.set_cursor_layer_frame(0,0)?;
        Ok(())
    })??;
    EDIT_HANDLE.save_project_file(expected)?;
    let mut results = Vec::new();
    let mut references = std::collections::HashMap::new();
    for (i, object) in project.objects.iter().enumerate() {
        let mut times = Vec::new();
        let mut first_ms = 0.;
        for local in 0..40u32 {
            let now = std::time::Instant::now();
            let (w, h, rgba) = capture_scene(i as u32 * 60 + local)?;
            let ms = now.elapsed().as_secs_f64() * 1000.;
            anyhow::ensure!((w, h) == (1920, 1080), "Wrong native host dimensions");
            anyhow::ensure!(
                rgba.chunks_exact(4)
                    .any(|p| p[0] != 0 || p[1] != 0 || p[2] != 0),
                "Blank native host frame"
            );
            if local == 0 {
                first_ms = ms;
            }
            if local >= 10 {
                times.push(ms);
            }
            if [0, 1, 15, 29].contains(&local) {
                image::save_buffer(
                    output.join(format!("{}-{local:02}.png", object.id)),
                    &rgba,
                    w,
                    h,
                    image::ColorType::Rgba8,
                )?;
                references.insert((i, local), rgba);
            }
        }
        for local in [29, 1, 15, 0, 29] {
            let (_, _, rgba) = capture_scene(i as u32 * 60 + local)?;
            anyhow::ensure!(
                rgba == references[&(i, local)],
                "Reverse seek changed {} frame {local}",
                object.id
            );
        }
        if object.animated {
            anyhow::ensure!(
                references[&(i, 0)] != references[&(i, 15)],
                "Animated native frames did not change"
            );
        }
        if object.scene == web_render_processing::Scene::Image && object.no_smooth {
            let rgba = &references[&(i, 0)];
            for (x, expected) in [(100, [255, 0, 0, 255]), (1820, [0, 0, 255, 255])] {
                let p = (540 * 1920 + x) * 4;
                anyhow::ensure!(
                    rgba[p..p + 4] == expected,
                    "Nearest image pixel differs: {:?}",
                    &rgba[p..p + 4]
                );
            }
        }
        if object.scene == web_render_processing::Scene::Alpha {
            // Host's scene capture uses straight RGBA; retain full bytes and explicit alpha probes.
            let rgba = &references[&(i, 0)];
            let left = (540 * 1920 + 100) * 4;
            let right = (540 * 1920 + 1820) * 4;
            std::fs::write(
                output.join("alpha-probes.json"),
                serde_json::to_vec_pretty(
                    &serde_json::json!({"left":&rgba[left..left+4],"right":&rgba[right..right+4]}),
                )?,
            )?;
            anyhow::ensure!(
                rgba[left + 3] == 64 && rgba[right + 3] == 192,
                "Host alpha values differ"
            );
        }
        times.sort_by(f64::total_cmp);
        results.push(serde_json::json!({"object":object.id,"no_smooth":object.no_smooth,"first_frame_ms":first_ms,
            "steady_samples":times.len(),"median_host_ms":times[times.len()/2],"p95_host_ms":times[(times.len()*95).div_ceil(100)-1],
            "reverse_seek_rgba":true,"animated":object.animated}));
    }
    // Compare freeze/unfreeze on the animated object, with real Lua/module cache keys.
    let animated = project.objects.iter().position(|o| o.animated).unwrap();
    let effect = project.objects[animated].label.clone();
    let start = animated * 60;
    EDIT_HANDLE.call_edit_section(|section| -> anyhow::Result<()> {
        let object = section.find_object_after(0, start)?.unwrap();
        section.set_object_effect_item(object, &effect, 0, "Freeze", "1")?;
        Ok(())
    })??;
    for local in [1, 15, 1] {
        let (_, _, rgba) = capture_scene(start as u32 + local)?;
        anyhow::ensure!(
            rgba == references[&(animated, local)],
            "Frozen native frame differs"
        );
    }
    EDIT_HANDLE.call_edit_section(|section| -> anyhow::Result<()> {
        let object = section.find_object_after(0, start)?.unwrap();
        section.set_object_effect_item(object, &effect, 0, "Freeze", "0")?;
        section.set_cursor_layer_frame(0, start + 15)?;
        // PNG output processes one frame using the host's saving/offline flag.
        section.set_select_range(start + 15, start + 15)?;
        Ok(())
    })??;
    std::fs::write(
        output.join("preview-result.json"),
        serde_json::to_vec_pretty(&serde_json::json!({"cases":results,"freeze_unfreeze":true}))?,
    )?;
    let exported = export_native_png(output, &references[&(animated, 15)])?;
    EDIT_HANDLE.save_project_file(expected)?;
    Ok(
        serde_json::json!({"status":"passed","project":expected,"backend":"libprocessing","replay":replay,
        "cases":results,"freeze_unfreeze":true,"offline_export":exported,"offline_matches_preview":true}),
    )
}

fn export_native_png(output: &std::path::Path, reference: &[u8]) -> anyhow::Result<PathBuf> {
    let export = output.join("native-export.png");
    if let Err(error) = EDIT_HANDLE.output_file(
        &export,
        "PNGファイル出力",
        None::<fn(&mut aviutl2::generic::ProjectFile)>,
    ) {
        // This host exposes the SDK entry but its CommonEditSection rejects output_file.
        // The display-2 supervisor opens the built-in PNG menu for this explicit sample.
        std::fs::write(
            output.join("export-request.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"file":export,"sdk_error":error.to_string()}),
            )?,
        )?;
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(300);
    let exported = loop {
        let mut found = None;
        for entry in std::fs::read_dir(output)? {
            let path = entry?.path();
            if path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("native-export")
            {
                if let Ok(img) = image::open(&path) {
                    found = Some((path, img.into_rgba8()));
                    break;
                }
            }
        }
        if let Some(value) = found {
            break value;
        }
        anyhow::ensure!(
            std::time::Instant::now() < deadline,
            "Native PNG export timed out"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    anyhow::ensure!(
        exported.1.dimensions() == (1920, 1080),
        "Native export dimensions differ"
    );
    anyhow::ensure!(
        exported.1.as_raw() == reference,
        "Offline PNG differs from preview"
    );
    std::thread::sleep(Duration::from_millis(500));
    Ok(exported.0)
}

fn run_p5(
    expected: &std::path::Path,
    output: &std::path::Path,
    project_name: &str,
) -> anyhow::Result<serde_json::Value> {
    let script = crate::get_script_dir(project_name).join("MIDI Pattern Grid.obj2");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !std::fs::read_to_string(&script)
        .is_ok_and(|s| s.starts_with("--value@WEB_RENDER_AUX2_midiPath:MIDI Path,\"/song.mid\""))
    {
        anyhow::ensure!(
            std::time::Instant::now() < deadline,
            "MIDI Pattern Grid obj2 was not generated"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    if std::env::var("WEB_RENDER_DEBUG_MODE").as_deref() == Ok("p5-bootstrap") {
        return Ok(serde_json::json!({"status":"bootstrap","script":script}));
    }
    let effect = EDIT_HANDLE
        .get_effects()
        .into_iter()
        .find(|effect| effect.name == "MIDI Pattern Grid")
        .ok_or_else(|| anyhow::anyhow!("Restart after bootstrap to register MIDI Pattern Grid"))?
        .name;
    let probe_effect = effect.clone();
    let p5_replay = std::env::var("WEB_RENDER_DEBUG_MODE").as_deref() == Ok("p5-replay");
    EDIT_HANDLE.call_edit_section(move |section| -> anyhow::Result<()> {
        let object = if p5_replay {
            let object=section.find_object_after(0,0)?.ok_or_else(|| anyhow::anyhow!("Saved p5 probe object missing"))?;
            anyhow::ensure!(section.get_object_name(object)?.as_deref()==Some("MIDI Pattern Grid test"),"Refused a different object");
            object
        } else {
            anyhow::ensure!(section.find_object_after(0,0)?.is_none(),"Use a new empty p5 sample copy");
            let alias=format!("[Object]\nframe=0,239\n[Object.0]\neffect.name={probe_effect}\n[Object.1]\neffect.name=標準描画\n");
            let object=section.create_object_from_alias(&alias,0,0,240)?;
            section.set_object_name(object,Some("MIDI Pattern Grid test"))?;
            object
        };
        anyhow::ensure!(section.get_object_effect_item(object, &probe_effect, 0, "MIDI Path")? == "/song.mid", "MIDI URL default is incorrect");
        Ok(())
    })??;
    EDIT_HANDLE.save_project_file(expected)?;
    // Preview starts async setup. Warm up with distinct frames because the
    // host can cache a transient blank/error scene frame while setup is pending.
    let mut ready = false;
    for frame in 1..=20 {
        let (width, height, rgba) = capture_scene(frame)?;
        anyhow::ensure!(
            (width, height) == (1920, 1080),
            "Unexpected warmup dimensions"
        );
        let offset = (492 * 1920 + 600) * 4;
        if rgba[offset] > 80 && rgba[offset + 1] > 0 {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    anyhow::ensure!(ready, "p5 preview setup did not become ready");
    let mut images = std::collections::HashMap::new();
    let mut frames = Vec::new();
    let mut differences = Vec::new();
    for frame in [53u32, 0, 60, 85, 108, 150, 85, 60, 0] {
        let (width, height, rgba) = capture_scene(frame)?;
        anyhow::ensure!(
            (width, height) == (1920, 1080),
            "Unexpected p5 host scene dimensions"
        );
        let mut counts = [0usize; 16];
        for row in 0..2 {
            for column in 0..8 {
                for y in (440 + 8 + row * 104)..(440 + 96 + row * 104) {
                    for x in (548 + 8 + column * 104)..(548 + 96 + column * 104) {
                        let pixel = &rgba[(y * 1920 + x) * 4..][..3];
                        if pixel.iter().any(|&c| c != 0) {
                            counts[row * 8 + column] += 1;
                        }
                    }
                }
            }
        }
        let step = if frame < 60 {
            (frame as f64 / 7.5).floor() as usize
        } else {
            (8.0 + (frame - 60) as f64 / 6.0).floor() as usize
        };
        for row in 0..2 {
            for column in 0..8 {
                let visible = if row == 0 {
                    column <= step % 8
                } else {
                    step >= 8
                };
                anyhow::ensure!(
                    (counts[row * 8 + column] > 0) == visible,
                    "Host frame {frame}: missing/extra cell {row}/{column}, counts={counts:?}"
                );
            }
        }
        let repeated = images.contains_key(&frame);
        if let Some(previous) = images.get(&frame) {
            if previous != &rgba {
                differences.push(frame);
            }
        } else {
            images.insert(frame, rgba.clone());
        }
        let prefix = if repeated { "repeated" } else { "aviutl" };
        image::save_buffer(
            output.join(format!("{prefix}-p5-frame-{frame:04}.png")),
            &rgba,
            width,
            height,
            image::ColorType::Rgba8,
        )?;
        frames.push(serde_json::json!({"frame":frame,"colored_pixels_per_cell":counts}));
    }
    EDIT_HANDLE.save_project_file(expected)?;
    Ok(
        serde_json::json!({"status":if differences.is_empty() {"passed"} else {"issues_detected"},
        "project":expected,"effect":effect,"frames":frames,"reverse_seek_full_rgba":differences.is_empty(),
        "repeated_differences":differences,"width":1920,"height":1080,"p5_canvas_width":824,"p5_canvas_height":200}),
    )
}

fn run_himawari(
    expected: &std::path::Path,
    output: &std::path::Path,
    project_name: &str,
) -> anyhow::Result<serde_json::Value> {
    let definitions: [(&str, i32, i32, i32); 3] = [
        ("Drum", -650, 64, 184),
        ("Kaiwai Phrase", -450, 64, 72),
        ("Synth Solo", 350, 640, 144),
    ];
    let scripts = crate::get_script_dir(project_name);
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    for (label, _, _, _) in definitions {
        let path = scripts.join(format!("{label}.obj2"));
        while !std::fs::read_to_string(&path).is_ok_and(|s| s.contains("WEB_RENDER_AUX2")) {
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "{label} obj2 was not generated"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    if std::env::var("WEB_RENDER_DEBUG_MODE").as_deref() == Ok("himawari-bootstrap") {
        return Ok(serde_json::json!({"status":"bootstrap","scripts":scripts}));
    }
    let effects = EDIT_HANDLE.get_effects();
    for (label, _, _, _) in definitions {
        anyhow::ensure!(
            effects.iter().any(|effect| effect.name == label),
            "Restart after bootstrap to register {label}"
        );
    }
    let replay = std::env::var("WEB_RENDER_DEBUG_MODE").as_deref() == Ok("himawari-replay");
    EDIT_HANDLE.call_edit_section(move |section| -> anyhow::Result<()> {
        for (layer, (label, x, _, _)) in definitions.into_iter().enumerate() {
            let name = format!("{label} himawari test");
            if replay {
                let object = section.find_object_after(layer, 0)?.ok_or_else(|| anyhow::anyhow!("Saved {label} object missing"))?;
                anyhow::ensure!(section.get_object_name(object)?.as_deref() == Some(&name), "Refused a different saved object");
            } else {
                anyhow::ensure!(section.find_object_after(layer, 0)?.is_none(), "Use an empty himawari sample copy");
                let alias = format!("[Object]\nframe=0,479\n[Object.0]\neffect.name={label}\n[Object.1]\neffect.name=標準描画\nX={x}\n");
                let object = section.create_object_from_alias(&alias, layer, 0, 480)?;
                section.set_object_name(object, Some(&name))?;
            }
        }
        Ok(())
    })??;
    if !replay {
        EDIT_HANDLE.save_project_file(expected)?;
    }
    let coverage = |rgba: &[u8]| -> Vec<usize> {
        definitions
            .iter()
            .map(|(_, x, width, height)| {
                let left = (960 + x - width / 2) as usize;
                let top = (540 - height / 2) as usize;
                (top..top + *height as usize)
                    .flat_map(|y| (left..left + *width as usize).map(move |x| (y * 1920 + x) * 4))
                    .filter(|offset| rgba[*offset..*offset + 3].iter().any(|c| *c > 0))
                    .count()
            })
            .collect()
    };
    let mut ready = false;
    for frame in 1..=20 {
        let (width, height, rgba) = capture_scene(frame)?;
        anyhow::ensure!(
            (width, height) == (1920, 1080),
            "Unexpected himawari scene dimensions"
        );
        if coverage(&rgba).iter().all(|count| *count > 0) {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    anyhow::ensure!(ready, "Three himawari previews did not become ready");
    if std::env::var("WEB_RENDER_DEBUG_MODE").as_deref() == Ok("himawari-movement") {
        let locals = [15u32, 63, 237, 432, 450];
        let mut references = std::collections::HashMap::new();
        for local in locals {
            let (w, h, rgba) = capture_scene(local)?;
            image::save_buffer(
                output.join(format!("reference-{local}.png")),
                &rgba,
                w,
                h,
                image::ColorType::Rgba8,
            )?;
            references.insert(local, rgba);
        }
        let mut old_start = 0usize;
        let mut checks = Vec::new();
        for start in [120usize, 600, 30, 0] {
            EDIT_HANDLE.call_edit_section(move |section| -> anyhow::Result<()> {
                for layer in 0..3 {
                    let object = section
                        .find_object_after(layer, old_start)?
                        .ok_or_else(|| anyhow::anyhow!("Himawari move object missing"))?;
                    anyhow::ensure!(
                        section
                            .get_object_name(object)?
                            .is_some_and(|n| n.ends_with(" himawari test")),
                        "Refused another object"
                    );
                    section.move_object(object, layer, start)?;
                }
                section.set_cursor_layer_frame(0, start + 15)?;
                Ok(())
            })??;
            for local in locals.into_iter().rev() {
                let (w, h, rgba) = capture_scene(start as u32 + local)?;
                image::save_buffer(
                    output.join(format!("start-{start}-local-{local}.png")),
                    &rgba,
                    w,
                    h,
                    image::ColorType::Rgba8,
                )?;
                checks.push(serde_json::json!({"start":start,"local":local,"matches_reference":rgba==references[&local],"colored_pixels":coverage(&rgba)}));
            }
            old_start = start;
        }
        EDIT_HANDLE.save_project_file(expected)?;
        return Ok(
            serde_json::json!({"status":if checks.iter().all(|c|c["matches_reference"]==true) {"passed"} else {"failed"},"checks":checks}),
        );
    }
    let mut images = std::collections::HashMap::new();
    let mut frames = vec![];
    let mut differences = vec![];
    for frame in [
        15, 0, 63, 54, 57, 60, 237, 240, 243, 429, 432, 450, 15, 63, 240, 0,
    ] {
        let started = std::time::Instant::now();
        let (width, height, rgba) = capture_scene(frame)?;
        let render_ms = started.elapsed().as_secs_f64() * 1000.0;
        anyhow::ensure!(
            (width, height) == (1920, 1080),
            "Unexpected himawari capture dimensions"
        );
        let counts = coverage(&rgba);
        let repeated = images.contains_key(&frame);
        if let Some(previous) = images.get(&frame) {
            if previous != &rgba {
                differences.push(frame);
            }
        } else {
            images.insert(frame, rgba.clone());
        }
        image::save_buffer(
            output.join(format!(
                "{}-himawari-frame-{frame:04}.png",
                if repeated { "repeat" } else { "aviutl" }
            )),
            &rgba,
            width,
            height,
            image::ColorType::Rgba8,
        )?;
        frames.push(serde_json::json!({"frame":frame,"colored_pixels_per_object":counts,"repeated":repeated,"render_ms":render_ms}));
    }
    anyhow::ensure!(
        frames
            .iter()
            .filter(|f| f["frame"] == 15)
            .all(|f| f["colored_pixels_per_object"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c.as_u64().unwrap() > 0)),
        "An active object rendered blank"
    );
    if !replay {
        EDIT_HANDLE.save_project_file(expected)?;
    }
    Ok(
        serde_json::json!({"status":if differences.is_empty(){"passed"}else{"issues_detected"},"project":expected,"replay":replay,
        "objects":["Drum","Kaiwai Phrase","Synth Solo"],"frames":frames,"width":1920,"height":1080,
        "reverse_seek_full_rgba":differences.is_empty(),"repeated_differences":differences}),
    )
}

fn set_freeze(effect: &str, frozen: bool) -> anyhow::Result<()> {
    EDIT_HANDLE.call_edit_section(|section| -> anyhow::Result<()> {
        let object = section
            .find_object_after(0, 0)?
            .ok_or_else(|| anyhow::anyhow!("Probe object disappeared"))?;
        section.set_object_effect_item(
            object,
            effect,
            0,
            "Freeze",
            if frozen { "1" } else { "0" },
        )?;
        Ok(())
    })??;
    Ok(())
}

fn close_own_host() {
    let Some(expected) = std::env::var_os("WEB_RENDER_DEBUG_PROJECT") else {
        return;
    };
    let current = crate::CURRENT_PROJECT_FILE.lock().unwrap().clone();
    aviutl2::tracing::info!("Debug close: current={current:?}, expected={expected:?}");
    let Ok(expected) = std::fs::canonicalize(expected) else {
        return;
    };
    if current.and_then(|path| std::fs::canonicalize(path).ok()) != Some(expected) {
        return;
    }
    // SDK save_project_file performs a backup and does not reset the edited
    // state. Let the host display its normal save confirmation when necessary;
    // reloading a project from this worker can race the host's preview drawing.
    #[link(name = "user32")]
    unsafe extern "system" {
        fn PostMessageW(hwnd: isize, message: u32, wparam: usize, lparam: isize) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, process: *mut u32) -> u32;
    }
    if let Some(window) = EDIT_HANDLE.get_host_app_window_raw() {
        let hwnd = window.hwnd.get();
        let mut pid = 0;
        // SAFETY: the SDK supplies a host HWND; output points to an initialized u32.
        // WM_CLOSE is posted only after verifying ownership by this exact process.
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == std::process::id() {
                let posted = PostMessageW(hwnd, 0x0010, 0, 0);
                aviutl2::tracing::info!("Debug WM_CLOSE hwnd={hwnd}, pid={pid}, posted={posted}");
            }
        }
    }
}
