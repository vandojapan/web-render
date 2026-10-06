use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use aviutl2::{
    AnyResult, AviUtl2Info, generic::GenericPlugin, module::ScriptModuleFunctions, tracing as log,
};

use crate::Vi5Aux2;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct LuaRenderParams {
    object_name: String,
    effect_id: i32,
    freeze: bool,
    offline: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(tag = "type", content = "value")]
enum LuaParameter {
    Str(String),
    Text(String),
    Number(f64),
    Bool(bool),
    Color(u32),
}
#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct LuaFrameInfo {
    x: f64,
    y: f64,
    z: f64,
    canvas_width: i32,
    canvas_height: i32,
    current_frame: i32,
    current_time: f64,
    total_frames: i32,
    total_time: f64,
    framerate: f64,
    global_frame: i32,
    global_time: f64,
}

#[aviutl2::plugin(ScriptModule)]
pub struct InternalModule;

static TEMPORARY_BUFFER: std::sync::LazyLock<crate::image_buffers::ImageBuffers> =
    std::sync::LazyLock::new(crate::image_buffers::ImageBuffers::default);
static RENDER_CACHE: std::sync::LazyLock<dashmap::DashMap<i32, RenderCachePerEffectEntry>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);
static ADJUSTED_BATCH_SIZE: std::sync::LazyLock<dashmap::DashMap<i32, usize>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);
static IS_FROZEN: std::sync::LazyLock<dashmap::DashMap<i32, bool>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);
static CACHE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Debug, Clone, Default)]
struct RenderCachePerEffectEntry {
    images: HashMap<u64, RenderCacheEntry>,
}
#[derive(Debug, Clone)]
struct RenderCacheEntry {
    image_data: Vec<u8>,
    width: usize,
    height: usize,
}

fn hash_parameter_value(param: &web_render_cef::ParameterValue, hasher: &mut impl Hasher) {
    match param {
        web_render_cef::ParameterValue::Str(value) => {
            0_u8.hash(hasher);
            value.hash(hasher);
        }
        web_render_cef::ParameterValue::Text(value) => {
            1_u8.hash(hasher);
            value.hash(hasher);
        }
        web_render_cef::ParameterValue::Number(value) => {
            2_u8.hash(hasher);
            value.to_bits().hash(hasher);
        }
        web_render_cef::ParameterValue::Bool(value) => {
            3_u8.hash(hasher);
            value.hash(hasher);
        }
        web_render_cef::ParameterValue::Color(value) => {
            4_u8.hash(hasher);
            value.r.hash(hasher);
            value.g.hash(hasher);
            value.b.hash(hasher);
            value.a.hash(hasher);
        }
    }
}

fn compute_cache_key(request: &web_render_cef::RenderRequest) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    crate::native_processing::FINGERPRINT
        .load(std::sync::atomic::Ordering::SeqCst)
        .hash(&mut hasher);
    request.object.hash(&mut hasher);
    request.object_id.hash(&mut hasher);
    request.is_offline.hash(&mut hasher);
    CACHE_GENERATION
        .load(std::sync::atomic::Ordering::SeqCst)
        .hash(&mut hasher);
    for param in &request.parameters {
        param.key.hash(&mut hasher);
        hash_parameter_value(&param.value, &mut hasher);
    }
    request.frame_info.x.to_bits().hash(&mut hasher);
    request.frame_info.y.to_bits().hash(&mut hasher);
    request.frame_info.z.to_bits().hash(&mut hasher);
    request.frame_info.screen_width.hash(&mut hasher);
    request.frame_info.screen_height.hash(&mut hasher);
    request.frame_info.current_frame.hash(&mut hasher);
    request.frame_info.current_time.to_bits().hash(&mut hasher);
    request.frame_info.total_frames.hash(&mut hasher);
    request.frame_info.total_time.to_bits().hash(&mut hasher);
    request.frame_info.framerate.to_bits().hash(&mut hasher);
    request.frame_info.global_frame.hash(&mut hasher);
    request.frame_info.global_time.to_bits().hash(&mut hasher);
    hasher.finish()
}

fn build_render_request(
    object_name: String,
    effect_id: i32,
    params: &HashMap<String, LuaParameter>,
    frame_info: &LuaFrameInfo,
    is_offline: bool,
) -> anyhow::Result<web_render_cef::RenderRequest> {
    web_render_cef::image::rgba_len(
        frame_info.canvas_width as usize,
        frame_info.canvas_height as usize,
    )?;
    if frame_info.current_frame < 0
        || frame_info.total_frames <= 0
        || frame_info.global_frame < 0
        || !frame_info.framerate.is_finite()
        || frame_info.framerate <= 0.0
        || [
            frame_info.x,
            frame_info.y,
            frame_info.z,
            frame_info.current_time,
            frame_info.total_time,
            frame_info.global_time,
        ]
        .iter()
        .any(|v| !v.is_finite())
    {
        anyhow::bail!("Invalid host frame information");
    }
    static NEXT_NONCE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);
    let render_nonce = NEXT_NONCE
        .fetch_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |n| Some(if n == i32::MAX { 1 } else { n + 1 }),
        )
        .unwrap();
    let mut param_keys: Vec<&String> = params.keys().collect();
    param_keys.sort();
    let parameters = param_keys
        .into_iter()
        .map(|key| {
            let param = params
                .get(key)
                .ok_or_else(|| anyhow::anyhow!("Missing parameter: {}", key))?;
            Ok(web_render_cef::Parameter {
                key: key.clone(),
                value: match param {
                    LuaParameter::Str(v) => web_render_cef::ParameterValue::Str(v.clone()),
                    LuaParameter::Text(v) => web_render_cef::ParameterValue::Text(v.clone()),
                    LuaParameter::Number(v) => web_render_cef::ParameterValue::Number(*v),
                    LuaParameter::Bool(v) => web_render_cef::ParameterValue::Bool(*v),
                    LuaParameter::Color(v) => {
                        let color = *v;
                        web_render_cef::ParameterValue::Color(web_render_cef::Color {
                            r: ((color >> 16) & 0xFF) as u8,
                            g: ((color >> 8) & 0xFF) as u8,
                            b: (color & 0xFF) as u8,
                            a: ((color >> 24) & 0xFF) as u8,
                        })
                    }
                },
            })
        })
        .collect::<anyhow::Result<Vec<web_render_cef::Parameter>>>()?;
    Ok(web_render_cef::RenderRequest {
        render_nonce,
        object: object_name,
        object_id: effect_id as i64,
        frame_info: web_render_cef::FrameInfo {
            x: frame_info.x,
            y: frame_info.y,
            z: frame_info.z,
            screen_width: frame_info.canvas_width as _,
            screen_height: frame_info.canvas_height as _,
            current_frame: frame_info.current_frame as _,
            current_time: frame_info.current_time,
            total_frames: frame_info.total_frames as _,
            total_time: frame_info.total_time,
            framerate: frame_info.framerate,
            global_frame: frame_info.global_frame as _,
            global_time: frame_info.global_time,
        },
        parameters,
        is_offline,
    })
}

impl aviutl2::module::ScriptModule for InternalModule {
    fn new(_info: AviUtl2Info) -> AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::module::ScriptModuleTable {
        aviutl2::module::ScriptModuleTable {
            information: "web-render.aux2 Internal Module".to_string(),
            functions: Self::functions(),
        }
    }
}

#[aviutl2::module::functions]
impl InternalModule {
    fn get_batch_size(&self, effect_id: i32) -> usize {
        *ADJUSTED_BATCH_SIZE.entry(effect_id).or_insert(1).value()
    }

    fn call_object(
        &self,
        render_params: String,
        params_json: String,
        frame_info_json: String,
    ) -> aviutl2::AnyResult<(*const u8, usize, usize)> {
        let render_params: LuaRenderParams = serde_json::from_str(&render_params)?;
        let image_reservation = TEMPORARY_BUFFER.reserve(render_params.effect_id)?;
        let batch_params: Vec<HashMap<String, LuaParameter>> = serde_json::from_str(&params_json)?;
        let batch_frame_info: Vec<LuaFrameInfo> = serde_json::from_str(&frame_info_json)?;
        if batch_params.is_empty() || batch_params.len() > 50 {
            anyhow::bail!("Render batch must contain between 1 and 50 requests");
        }
        let batch_render_request = if batch_params.len() != batch_frame_info.len() {
            anyhow::bail!(
                "Mismatched batch sizes: params {}, frame_info {}",
                batch_params.len(),
                batch_frame_info.len()
            );
        } else {
            batch_params
                .into_iter()
                .zip(batch_frame_info)
                .map(|(params, frame_info)| {
                    build_render_request(
                        render_params.object_name.clone(),
                        render_params.effect_id,
                        &params,
                        &frame_info,
                        render_params.offline,
                    )
                })
                .collect::<anyhow::Result<Vec<web_render_cef::RenderRequest>>>()?
        };

        let mut current_freeze_state = IS_FROZEN
            .entry(render_params.effect_id)
            .or_insert(render_params.freeze);
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Failed to get cache directory"))?
            .join("web_render_aux2_cache")
            .join(format!(
                "{}-{}",
                std::process::id(),
                CACHE_GENERATION.load(std::sync::atomic::Ordering::SeqCst)
            ))
            .join(render_params.effect_id.to_string());
        match (render_params.freeze, *current_freeze_state) {
            (true, false) => {
                log::info!("Freezing cache for effect_id {}", render_params.effect_id);
                *current_freeze_state = true;
            }
            (false, true) => {
                log::info!("Unfreezing cache for effect_id {}", render_params.effect_id);
                std::fs::remove_dir_all(&cache_dir).ok();
                *current_freeze_state = false;
            }
            _ => {}
        }

        let batch_cache_keys: Vec<u64> =
            batch_render_request.iter().map(compute_cache_key).collect();

        if *current_freeze_state {
            log::debug!(
                "Cache is frozen for effect_id {}, skipping rendering and using existing cache if available",
                render_params.effect_id
            );
            let cache_path = cache_dir.join(format!("{}.webp", batch_cache_keys[0]));
            if let Ok(opened) = image::open(&cache_path).map(|img| img.into_rgba8()) {
                log::debug!(
                    "Loaded cached image from {:?} for effect_id {}",
                    cache_path,
                    render_params.effect_id
                );
                let (width, height) = opened.dimensions();
                let image_data = opened.into_raw();
                web_render_cef::image::validate_rgba(
                    width as usize,
                    height as usize,
                    image_data.len(),
                )?;
                return Ok((
                    image_reservation.commit(image_data)?,
                    width as usize,
                    height as usize,
                ));
            }
        }

        let mut cached_entries = RENDER_CACHE.entry(render_params.effect_id).or_default();

        let should_render_now = !cached_entries.images.contains_key(&batch_cache_keys[0]);

        if should_render_now {
            let (uncached_keys, uncached_requests) = batch_render_request
                .into_iter()
                .enumerate()
                .filter_map(|(i, req)| {
                    if !cached_entries.images.contains_key(&batch_cache_keys[i]) {
                        Some(((req.render_nonce, batch_cache_keys[i]), req))
                    } else {
                        None
                    }
                })
                .unzip::<_, _, Vec<_>, Vec<_>>();
            log::debug!(
                "Rendering {} uncached requests for effect_id {}",
                uncached_requests.len(),
                render_params.effect_id
            );
            let rendered = Vi5Aux2::with_instance({
                move |instance| {
                    instance
                        .runtime
                        .read()
                        .map_err(|e| anyhow::anyhow!("Failed to acquire runtime read lock: {}", e))?
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("tokio runtime is not initialized"))?
                        .block_on(instance.render_batch(uncached_requests))
                }
            })?;
            // batch_cache_keys に存在しないキャッシュを削除
            cached_entries
                .images
                .retain(|key, _| batch_cache_keys.contains(key));

            let mut response_keys: HashMap<i32, u64> = uncached_keys.into_iter().collect();
            for response in rendered {
                let cache_key = response_keys
                    .remove(&response.render_nonce)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Unexpected or duplicate response ID {}",
                            response.render_nonce
                        )
                    })?;
                match response.response {
                    web_render_cef::RenderResponseData::Success {
                        width,
                        height,
                        image_data,
                    } => {
                        web_render_cef::image::validate_rgba(
                            width as usize,
                            height as usize,
                            image_data.len(),
                        )?;
                        cached_entries.images.insert(
                            cache_key,
                            RenderCacheEntry {
                                image_data,
                                width: width as usize,
                                height: height as usize,
                            },
                        );

                        if *current_freeze_state {
                            let cache_path = cache_dir.join(format!("{}.webp", cache_key));
                            if let Err(e) = std::fs::create_dir_all(&cache_dir)
                                .map_err(anyhow::Error::from)
                                .and_then(|_| {
                                    image::RgbaImage::from_raw(
                                        width as _,
                                        height as _,
                                        cached_entries
                                            .images
                                            .get(&cache_key)
                                            .unwrap()
                                            .image_data
                                            .clone(),
                                    )
                                    .ok_or_else(|| {
                                        anyhow::anyhow!("Failed to create image from raw data")
                                    })
                                    .and_then(|img| {
                                        img.save_with_format(&cache_path, image::ImageFormat::WebP)
                                            .map_err(|e| {
                                                anyhow::anyhow!(
                                                    "Failed to save cached image to {:?}: {}",
                                                    cache_path,
                                                    e
                                                )
                                            })
                                    })
                                })
                            {
                                log::error!(
                                    "Failed to save cached image for cache_key {}: {}",
                                    cache_key,
                                    e
                                );
                            } else {
                                log::debug!(
                                    "Saved cached image to {:?} for cache_key {}",
                                    cache_path,
                                    cache_key
                                );
                            }
                        }
                    }
                    web_render_cef::RenderResponseData::Error(err) => {
                        if cache_key == batch_cache_keys[0] {
                            anyhow::bail!("JS returned error: {}", err);
                        }
                        continue;
                    }
                }
            }
            if !response_keys.is_empty() {
                anyhow::bail!("Missing rendered responses: {:?}", response_keys.keys());
            }

            // CEF captures one request at a time; automatic prefetch uses one frame.
            ADJUSTED_BATCH_SIZE.insert(render_params.effect_id, 1);
        }

        let current_image = cached_entries
            .images
            .get(&batch_cache_keys[0])
            .ok_or_else(|| anyhow::anyhow!("Unreachable: first image not cached"))?;
        let current_image_data = current_image.image_data.clone();
        let current_image_ptr = image_reservation.commit(current_image_data)?;
        Ok((current_image_ptr, current_image.width, current_image.height))
    }
    fn free_image(&self, id: i32) {
        if TEMPORARY_BUFFER.free(id) {
            log::debug!("Disposed image buffer for id {}", id);
        } else {
            log::warn!("No image buffer found for id {}", id);
        }
    }
}

pub fn clear_render_cache() {
    CACHE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    RENDER_CACHE.clear();
    // Images already returned to Lua remain owned until free_image.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_and_mismatched_lua_batches_before_rendering() {
        let module = InternalModule;
        let options = r#"{"object_name":"test","effect_id":-999,"freeze":false,"offline":true}"#;
        assert!(
            module
                .call_object(options.into(), "[]".into(), "[]".into())
                .is_err()
        );
        assert!(
            module
                .call_object(options.into(), "[{}]".into(), "[]".into())
                .is_err()
        );
    }

    #[test]
    fn cache_clear_preserves_a_returned_image() {
        let id = -998;
        let pointer = TEMPORARY_BUFFER
            .reserve(id)
            .unwrap()
            .commit(vec![1, 2, 3, 255])
            .unwrap();
        clear_render_cache();
        // SAFETY: cache clear does not release the four-byte image lease.
        assert_eq!(
            unsafe { std::slice::from_raw_parts(pointer, 4) },
            &[1, 2, 3, 255]
        );
        assert!(TEMPORARY_BUFFER.free(id));
    }
}
