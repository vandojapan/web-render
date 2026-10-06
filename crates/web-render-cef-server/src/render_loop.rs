use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use cef::{ImplBrowser, ImplFrame};
use prost::Message;
use tokio::sync::broadcast;
use web_render_cef::image::{PaintFormat, PaintFrame};

type PaintCallback = dyn FnMut(&PaintFrame<'_>) -> std::ops::ControlFlow<()> + Send + Sync;
static PAINT_CALLBACKS: std::sync::LazyLock<dashmap::DashMap<u32, Box<PaintCallback>>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);
const NOTIFICATION_NONCE: u32 = 1;

fn maybe_temporary_save_buffer(frame: &PaintFrame<'_>, nonce: u32) {
    if std::env::var("VI5_SAVE_PAINT_BUFFERS").is_ok() {
        std::fs::create_dir_all("paint_buffer").expect("Failed to create paint_buffer directory");
        let current_nano = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let filename = format!("paint_buffer/paint_buffer_{current_nano}_nonce_{nonce}.png");
        let (width, height) = frame.dimensions();
        let rgba = frame
            .crop_rgba(0, 0, width as i32, height as i32)
            .expect("Validated paint dimensions");
        let png = image::RgbaImage::from_raw(width as u32, height as u32, rgba)
            .expect("Validated paint length");
        png.save(&filename)
            .expect("Failed to save paint buffer as PNG");
        tracing::info!("Saved paint buffer to {}", filename);
    }
}

fn on_paint(frame: PaintFrame<'_>) {
    let nonce = match frame.metadata_nonce() {
        Ok(nonce) => nonce,
        Err(_) => {
            maybe_temporary_save_buffer(&frame, 0);
            return;
        }
    };
    // Inspect nonce first. No full-surface conversion or packing is needed;
    // callbacks decode metadata and copy only their validated image rectangles.
    if let Some(mut callback) = PAINT_CALLBACKS.get_mut(&nonce) {
        match callback(&frame) {
            std::ops::ControlFlow::Break(()) => {
                tracing::debug!("Paint callback for nonce {} completed and removed", nonce);
                drop(callback);
                PAINT_CALLBACKS.remove(&nonce);
            }
            std::ops::ControlFlow::Continue(()) => {
                tracing::debug!(
                    "Paint callback for nonce {} processed frame, waiting for more frames",
                    nonce
                );
            }
        }
    } else {
        tracing::warn!("No paint callback found for nonce {}", nonce);
        maybe_temporary_save_buffer(&frame, nonce);
    }
}

pub fn on_software_paint(buffer: &[u8], width: usize, height: usize) {
    tracing::debug!("Software paint received: {}x{}", width, height);
    match PaintFrame::new(
        buffer,
        width,
        height,
        width * 4,
        PaintFormat::PremultipliedBgra,
    ) {
        Ok(frame) => on_paint(frame),
        Err(error) => tracing::error!("Invalid software paint: {error}"),
    }
}
pub fn on_accelerated_paint(
    buffer: &wgpu::BufferView,
    width: usize,
    height: usize,
    bytes_per_row: usize,
) {
    tracing::debug!("Accelerated paint received: {}x{}", width, height);
    match PaintFrame::new(buffer, width, height, bytes_per_row, PaintFormat::Rgba) {
        Ok(frame) => on_paint(frame),
        Err(error) => tracing::error!("Invalid accelerated paint: {error}"),
    }
}

pub struct RenderLoop {
    browser: cef::Browser,
    initialized:
        Arc<std::sync::Mutex<Option<anyhow::Result<crate::protocol::serverjs::InitializeInfo>>>>,
    notification_tx: broadcast::Sender<crate::protocol::libserver::Notification>,
    notification_history: Arc<std::sync::Mutex<Vec<crate::protocol::libserver::Notification>>>,
}

impl RenderLoop {
    pub fn new(browser: cef::Browser) -> Self {
        let (notification_tx, _) = broadcast::channel(128);
        Self {
            browser,
            initialized: Arc::new(std::sync::Mutex::new(None)),
            notification_tx,
            notification_history: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn subscribe_notifications(
        &self,
    ) -> (
        Vec<crate::protocol::libserver::Notification>,
        broadcast::Receiver<crate::protocol::libserver::Notification>,
    ) {
        let history_guard = self
            .notification_history
            .lock()
            .expect("Failed to lock notification history");
        let rx = self.notification_tx.subscribe();
        let history = history_guard.clone();
        (history, rx)
    }

    pub async fn assert_initialized(&self) -> anyhow::Result<()> {
        let initialized = self
            .initialized
            .lock()
            .expect("Failed to lock initialization state");
        match initialized.as_ref() {
            Some(Ok(_)) => Ok(()),
            Some(Err(e)) => Err(anyhow::anyhow!("RenderLoop initialization failed: {}", e)),
            None => anyhow::bail!("RenderLoop is not initialized"),
        }
    }

    pub async fn wait_for_initialization(&self) -> anyhow::Result<()> {
        let start_time = std::time::Instant::now();
        loop {
            if self
                .initialized
                .lock()
                .expect("Failed to lock initialization state")
                .is_some()
            {
                return Ok(());
            }
            cef::do_message_loop_work();
            tokio::time::sleep(Duration::from_millis(10)).await;
            if start_time.elapsed() > Duration::from_secs(30) {
                anyhow::bail!("Timeout waiting for initialization");
            }
        }
    }

    pub async fn initialize(
        &self,
        url: &str,
    ) -> anyhow::Result<crate::protocol::serverjs::InitializeInfo> {
        {
            let mut initialized = self
                .initialized
                .lock()
                .expect("Failed to lock initialization state");
            *initialized = None;
        }
        self.notification_history
            .lock()
            .expect("Failed to lock notification history")
            .clear();
        PAINT_CALLBACKS.clear();
        PAINT_CALLBACKS.insert(
            NOTIFICATION_NONCE,
            Box::new({
                let notification_tx = self.notification_tx.clone();
                let notification_history = self.notification_history.clone();
                move |frame| {
                    match read_message_from_image::<crate::protocol::serverjs::Notifications>(
                        frame,
                    ) {
                        Ok(payload) => {
                            for notification in payload.entries {
                                match notification.entry {
                                    Some(crate::protocol::serverjs::notification_entry::Entry::Log(log)) => {
                                        let log_notification =
                                            crate::protocol::libserver::Notification {
                                                notification: Some(
                                                    crate::protocol::libserver::notification::Notification::LogNotification(
                                                        crate::protocol::libserver::LogNotification {
                                                            level: log.level,
                                                            message: log.message,
                                                        },
                                                    ),
                                                ),
                                            };
                                        publish_notification(
                                            &notification_tx,
                                            &notification_history,
                                            log_notification,
                                        );
                                    }
                                    Some(crate::protocol::serverjs::notification_entry::Entry::ObjectListUpdate(object_list)) => {
                                        let object_infos_notification =
                                            crate::protocol::libserver::Notification {
                                                notification: Some(
                                                    crate::protocol::libserver::notification::Notification::ObjectInfoNotification(
                                                        crate::protocol::libserver::ObjectInfosNotification {
                                                            object_infos: object_list.object_infos
                                                        }
                                                    ),
                                                )
                                            };
                                        publish_notification(
                                            &notification_tx,
                                            &notification_history,
                                            object_infos_notification,
                                        );
                                    }
                                    None => {
                                        tracing::warn!("Received notification with unparsable entry");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to decode notification payload: {}", e);
                        }
                    }
                    std::ops::ControlFlow::Continue(())
                }
            }),
        );
        PAINT_CALLBACKS.insert(
            0,
            Box::new({
                let initialized = self.initialized.clone();
                let browser = self.browser.clone();
                move |paint| {
                    let result = match read_message_from_image::<
                        crate::protocol::serverjs::InitializeInfo,
                    >(paint)
                    {
                        Ok(info) => {
                            tracing::info!("Page initialization complete");
                            Ok(info)
                        }
                        Err(e) => {
                            tracing::error!("Failed to decode InitializationComplete: {}", e);
                            Err(anyhow::anyhow!(
                                "Failed to decode InitializationComplete: {}",
                                e
                            ))
                        }
                    };
                    let mut initialized = initialized
                        .lock()
                        .expect("Failed to lock initialization state");
                    if initialized.is_none() {
                        *initialized = Some(result);
                    }
                    if let Some(frame) = browser.main_frame() {
                        frame.execute_java_script(
                            Some(&cef::CefString::from("window.__vi5__.acknowledge(0)")),
                            None,
                            0,
                        );
                    }
                    std::ops::ControlFlow::Break(())
                }
            }),
        );
        tracing::info!("Loading URL for initialization: {}", url);
        self.browser
            .main_frame()
            .unwrap()
            .load_url(Some(&cef::CefString::from(url)));
        self.wait_for_initialization().await?;
        let initialized = self
            .initialized
            .lock()
            .expect("Failed to lock initialization state");
        match initialized.as_ref().expect("Initialization state missing") {
            Ok(info) => Ok(info.clone()),
            Err(e) => Err(anyhow::anyhow!("Initialization failed: {}", e)),
        }
    }

    pub async fn batch_render(
        &self,
        request: crate::protocol::common::BatchRenderRequest,
    ) -> anyhow::Result<crate::protocol::libserver::BatchRenderResponse> {
        self.assert_initialized().await?;
        if request.render_requests.is_empty() {
            return Ok(crate::protocol::libserver::BatchRenderResponse {
                render_responses: vec![],
            });
        }
        tracing::debug!(
            "Starting batch render with {} requests",
            request.render_requests.len()
        );
        let start_time = std::time::Instant::now();
        let nonce = loop {
            let nonce = rand::random::<u32>();
            // 1024までは予約しておく
            if !PAINT_CALLBACKS.contains_key(&nonce) && nonce > 1024 {
                break nonce;
            }
        };
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut maybe_tx = Some(tx);
        let expected_ids: std::collections::HashSet<_> = request
            .render_requests
            .iter()
            .map(|r| r.render_nonce)
            .collect();
        if expected_ids.len() != request.render_requests.len()
            || expected_ids.iter().any(|n| *n <= 0)
        {
            anyhow::bail!("Request IDs must be unique and positive");
        }
        let mut remaining_ids = expected_ids.clone();
        let callback = Box::new(move |paint: &PaintFrame<'_>| {
            let Some(tx) = &maybe_tx else {
                return std::ops::ControlFlow::Break(());
            };
            let response = match read_message_from_image::<
                crate::protocol::serverjs::RootRenderResponse,
            >(paint)
            {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::error!("Failed to decode BatchRenderResponse: {}", e);
                    let _ = tx.send(Err(anyhow::anyhow!("Malformed render metadata: {e}")));
                    return std::ops::ControlFlow::Break(());
                }
            };

            let response = match response.response {
                Some(crate::protocol::serverjs::root_render_response::Response::Success(
                    batch_response,
                )) => batch_response,
                Some(crate::protocol::serverjs::root_render_response::Response::ErrorMessage(
                    err,
                )) => {
                    tracing::error!("Batch render error: {}", err);
                    for render_nonce in &remaining_ids {
                        let _ = tx.send(anyhow::Ok(crate::protocol::libserver::RenderResponse {
                            render_nonce: *render_nonce,
                            response: Some(
                                crate::protocol::libserver::render_response::Response::ErrorMessage(
                                    err.clone(),
                                ),
                            ),
                        }));
                    }
                    return std::ops::ControlFlow::Break(());
                }
                _ => {
                    tracing::error!("Invalid RootRenderResponse: missing BatchRenderResponse");
                    let _ = tx.send(Err(anyhow::anyhow!("Missing root render response")));
                    return std::ops::ControlFlow::Break(());
                }
            };

            for single_render_response in response.render_responses {
                if !remaining_ids.remove(&single_render_response.nonce) {
                    let _ = tx.send(Err(anyhow::anyhow!(
                        "Unexpected or duplicate response ID {}",
                        single_render_response.nonce
                    )));
                    return std::ops::ControlFlow::Break(());
                }
                let Some(single_response) = single_render_response.response else {
                    let _ = tx.send(Err(anyhow::anyhow!(
                        "Missing response for ID {}",
                        single_render_response.nonce
                    )));
                    return std::ops::ControlFlow::Break(());
                };
                match single_response {
                    crate::protocol::serverjs::single_render_response::Response::RendereredObjectInfo(
                        renderered_object_info,
                    ) => {
                        let info = &renderered_object_info;
                        let image_data = if info.y < web_render_cef::image::METADATA_ROWS as i32 {
                            Err(web_render_cef::image::ImageError("crop overlaps metadata rows"))
                        } else {
                            paint.crop_rgba(info.x, info.y, info.width, info.height)
                        };
                        let image_data = match image_data {
                            Ok(image) => image,
                            Err(error) => {
                                let _ = tx.send(Err(anyhow::anyhow!("Invalid crop for ID {}: {error}", single_render_response.nonce)));
                                return std::ops::ControlFlow::Break(());
                            }
                        };
                        let _ = tx.send(anyhow::Ok(crate::protocol::libserver::RenderResponse {
                            render_nonce: single_render_response.nonce,
                            response: Some(
                                crate::protocol::libserver::render_response::Response::Success(
                                    crate::protocol::libserver::SuccessRenderResponse {
                                        width: renderered_object_info.width,
                                        height: renderered_object_info.height,
                                        image_data,
                                    },
                                ),
                            ),
                        }));
                    }
                    crate::protocol::serverjs::single_render_response::Response::ErrorMessage(
                        err,
                    ) => {
                        let _ = tx.send(anyhow::Ok(crate::protocol::libserver::RenderResponse {
                            render_nonce: single_render_response.nonce,
                            response: Some(
                                crate::protocol::libserver::render_response::Response::ErrorMessage(
                                    err,
                                ),
                            ),
                        }));
                    }
                }
            }

            if !response.is_incomplete {
                if !remaining_ids.is_empty() {
                    let _ = tx.send(Err(anyhow::anyhow!(
                        "Missing response IDs: {remaining_ids:?}"
                    )));
                }
                drop(maybe_tx.take());
                tracing::debug!(
                    "All render responses received for nonce {}, took {:?}",
                    nonce,
                    start_time.elapsed()
                );
                return std::ops::ControlFlow::Break(());
            }
            std::ops::ControlFlow::Continue(())
        });
        PAINT_CALLBACKS.insert(nonce, callback);
        // Removes the callback on success, error, timeout, and cancellation of this future.
        struct CallbackGuard {
            nonce: u32,
            browser: cef::Browser,
            completed: bool,
        }
        impl Drop for CallbackGuard {
            fn drop(&mut self) {
                PAINT_CALLBACKS.remove(&self.nonce);
                if !self.completed
                    && let Some(frame) = self.browser.main_frame()
                {
                    let js = format!("window.__vi5__?.cancelCapture({});", self.nonce);
                    frame.execute_java_script(Some(&cef::CefString::from(js.as_str())), None, 1);
                }
            }
        }
        let mut callback_guard = CallbackGuard {
            nonce,
            browser: self.browser.clone(),
            completed: false,
        };
        let request = base64::engine::general_purpose::STANDARD.encode(request.encode_to_vec());
        let js = format!("window.__vi5__.render({nonce}, '{request}');");
        tracing::debug!(
            "Executing JS to request frame with nonce {}: {}",
            nonce,
            &js
        );
        tracing::debug!("Requesting frame with nonce {}", nonce);
        self.browser.main_frame().unwrap().execute_java_script(
            Some(&cef::CefString::from(js.as_str())),
            None,
            1,
        );
        let mut render_responses = vec![];
        // main_server pumps CEF continuously on this same thread. Wake as soon as
        // OnPaint sends a response instead of adding a second 5 ms polling loop.
        let deadline = tokio::time::sleep(Duration::from_secs(30));
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                response = rx.recv() => match response {
                    Some(response) => render_responses.push(response?),
                    None => break,
                },
                _ = &mut deadline => {
                    anyhow::bail!("Timeout waiting for render responses");
                }
            }
        }

        self.browser
            .main_frame()
            .ok_or_else(|| anyhow::anyhow!("CEF main frame unavailable"))?
            .execute_java_script(
                Some(&cef::CefString::from(
                    format!("window.__vi5__.acknowledge({nonce});").as_str(),
                )),
                None,
                1,
            );
        callback_guard.completed = true;
        tracing::debug!(
            nonce,
            elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0,
            "Batch response ready after ACK"
        );
        Ok(crate::protocol::libserver::BatchRenderResponse { render_responses })
    }

    pub async fn purge_cache(&self) -> anyhow::Result<()> {
        self.assert_initialized().await?;
        let mut keys_to_remove = vec![];
        for callback in PAINT_CALLBACKS.iter() {
            if *callback.key() > 1024 {
                keys_to_remove.push(*callback.key());
            }
        }
        for key in keys_to_remove {
            PAINT_CALLBACKS.remove(&key);
        }
        let js = "window.__vi5__.purgeCache();";
        self.browser.main_frame().unwrap().execute_java_script(
            Some(&cef::CefString::from(js)),
            None,
            1,
        );
        Ok(())
    }
}

fn publish_notification(
    notification_tx: &broadcast::Sender<crate::protocol::libserver::Notification>,
    notification_history: &Arc<std::sync::Mutex<Vec<crate::protocol::libserver::Notification>>>,
    notification: crate::protocol::libserver::Notification,
) {
    const MAX_NOTIFICATION_HISTORY: usize = 256;

    {
        let mut history = notification_history
            .lock()
            .expect("Failed to lock notification history");
        history.push(notification.clone());
        if history.len() > MAX_NOTIFICATION_HISTORY {
            let drain_count = history.len() - MAX_NOTIFICATION_HISTORY;
            history.drain(0..drain_count);
        }
    }

    let _ = notification_tx.send(notification);
}

fn read_message_from_image<T: Message + Default>(frame: &PaintFrame<'_>) -> anyhow::Result<T> {
    // H1 H2 H3 A_ N1 N2 N3 A_ N4 L1 L2 A_ L3 L4 M1 A_ M2 M3 M4 A_ M5 ...
    // H: header bytes
    // N: nonce bytes
    // A: alpha byte (ignored)
    // L: length bytes (little-endian u32)
    // M: message bytes
    let message_buffer = frame.read_metadata()?;
    let message = T::decode(&message_buffer[..])?;
    Ok(message)
}
