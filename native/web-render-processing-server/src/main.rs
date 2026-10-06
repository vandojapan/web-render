//! Dedicated headless process: all libprocessing calls stay on this main thread.
use bevy::{prelude::*, render::render_resource::Extent3d};
use processing::prelude::*;
use std::{
    collections::HashMap,
    io::{BufRead, Write},
    path::PathBuf,
    time::Instant,
};
use web_render_processing::{Project, Request, Response, Scene, VERSION};

struct Graphics {
    surface: Entity,
    graphics: Entity,
    width: u32,
    height: u32,
}

fn run() -> anyhow::Result<()> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("Expected native project directory"))?,
    );
    let project = Project::load(&root)?;
    init(Config::default())?;
    let image = image_create(
        Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
        vec![255, 0, 0, 255, 0, 0, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
    )?;
    let alpha_image = image_create(
        Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
        vec![255, 255, 255, 64, 255, 255, 255, 192],
        TextureFormat::Rgba8UnormSrgb,
    )?;
    let mut graphics: HashMap<(String, i64), Graphics> = HashMap::new();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    // Readiness is written only after GPU initialization; stdout is exclusively IPC.
    writeln!(
        output,
        "{}",
        serde_json::json!({"ready":VERSION,"project":project.name})
    )?;
    output.flush()?;
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        if line == "shutdown" {
            break;
        }
        anyhow::ensure!(line.len() <= 64 * 1024, "Native request too large");
        let request: Request = serde_json::from_str(&line)?;
        let start = Instant::now();
        let mut created = false;
        let rendered = (|| -> anyhow::Result<Vec<u8>> {
            request.validate()?;
            let object = project
                .objects
                .iter()
                .find(|o| o.id == request.object)
                .ok_or_else(|| anyhow::anyhow!("Unknown native object"))?;
            let key = (object.id.clone(), request.object_id);
            if graphics
                .get(&key)
                .is_some_and(|g| g.width != request.width || g.height != request.height)
            {
                let old = graphics.remove(&key).unwrap();
                graphics_destroy(old.graphics)?;
                surface_destroy(old.surface)?;
            }
            if !graphics.contains_key(&key) {
                // Cap per-worker GPU resources; host may allocate new effect IDs after editing.
                if graphics.len() >= 8 {
                    let victim = graphics.keys().next().unwrap().clone();
                    let old = graphics.remove(&victim).unwrap();
                    graphics_destroy(old.graphics)?;
                    surface_destroy(old.surface)?;
                }
                let surface = surface_create_offscreen(
                    request.width,
                    request.height,
                    1.,
                    TextureFormat::Rgba8UnormSrgb,
                )?;
                let g = graphics_create(
                    surface,
                    request.width,
                    request.height,
                    TextureFormat::Rgba8UnormSrgb,
                )?;
                if object.no_smooth {
                    graphics_no_smooth(g)?;
                }
                graphics.insert(
                    key.clone(),
                    Graphics {
                        surface,
                        graphics: g,
                        width: request.width,
                        height: request.height,
                    },
                );
                created = true;
            }
            let g = graphics[&key].graphics;
            let w = request.width as f32;
            let h = request.height as f32;
            let command = |c| graphics_record_command(g, c);
            graphics_begin_draw(g)?;
            command(DrawCommand::BackgroundColor(
                if object.scene == Scene::Alpha {
                    Color::NONE
                } else {
                    Color::BLACK
                },
            ))?;
            command(DrawCommand::NoStroke)?;
            command(DrawCommand::Fill(Color::WHITE))?;
            let offset = if object.animated {
                (request.time * 41.).rem_euclid(w as f64) as f32
            } else {
                0.
            };
            match object.scene {
                Scene::Shapes => {
                    for i in 0..request.count {
                        let x = (((i.wrapping_mul(37).wrapping_add(object.seed)) % request.width)
                            as f32
                            + offset)
                            .rem_euclid(w)
                            + 0.3;
                        let y = (i.wrapping_mul(71) % request.height) as f32 + 0.3;
                        command(DrawCommand::Triangle {
                            x1: x,
                            y1: y,
                            x2: x + 19.,
                            y2: y + 5.,
                            x3: x + 3.,
                            y3: y + 23.,
                        })?;
                        command(DrawCommand::Ellipse {
                            cx: x + 31.,
                            cy: y + 31.,
                            w: 19.,
                            h: 19.,
                        })?;
                    }
                }
                Scene::Text => {
                    command(DrawCommand::TextSize(24.))?;
                    for i in 0..request.count {
                        command(DrawCommand::Text {
                            content: "Processing 0123 Aa".into(),
                            x: (i % 5) as f32 * w / 5. + offset,
                            y: (i / 5 + 1) as f32 * h / 21.,
                            z: 0.,
                            max_w: None,
                            max_h: None,
                        })?;
                    }
                }
                Scene::Image | Scene::Alpha => command(DrawCommand::Image {
                    entity: if object.scene == Scene::Alpha {
                        alpha_image
                    } else {
                        image
                    },
                    dx: 0.,
                    dy: 0.,
                    d_width: Some(w),
                    d_height: Some(h),
                    sx: None,
                    sy: None,
                    s_width: None,
                    s_height: None,
                })?,
            }
            graphics_flush(g)?;
            let mut raw = graphics_readback_completed_raw(g)?;
            anyhow::ensure!(
                raw.width == request.width
                    && raw.height == request.height
                    && raw.bytes.len() == (request.width as usize) * (request.height as usize) * 4,
                "Invalid native readback"
            );
            web_render_processing::straight_rgba(&mut raw.bytes);
            Ok(raw.bytes)
        })();
        let (bytes, error) = match rendered {
            Ok(b) => (b, None),
            Err(e) => (Vec::new(), Some(format!("{e:#}"))),
        };
        let response = Response {
            version: VERSION,
            nonce: request.nonce,
            width: request.width,
            height: request.height,
            bytes: bytes.len(),
            render_ms: start.elapsed().as_secs_f64() * 1000.,
            created_graphics: created,
            error,
        };
        writeln!(output, "{}", serde_json::to_string(&response)?)?;
        output.write_all(&bytes)?;
        output.flush()?;
    }
    for (_, g) in graphics {
        graphics_destroy(g.graphics)?;
        surface_destroy(g.surface)?;
    }
    Ok(())
}

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter("warn")
        .try_init();
    // Drop the Bevy App explicitly before Windows destroys thread-local storage.
    let result = run();
    if let Err(e) = &result {
        eprintln!("native renderer: {e:#}");
    }
    let _ = processing::exit(if result.is_ok() { 0 } else { 1 });
    if result.is_err() {
        std::process::exit(1);
    }
}
