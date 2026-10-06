mod cef_app;
mod compression;
mod gpu_capture;
mod handlers;
mod parent_process;
mod protocol;
mod render_loop;
mod server;
mod types;

use std::sync::Arc;

use clap::Parser;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::cef_app::{
    build_render_options, build_settings, create_browser, initialize_cef, prepare_process,
};
use crate::gpu_capture::GpuCapture;
use crate::handlers::create_client;
use crate::render_loop::RenderLoop;

#[derive(clap::Parser, Debug)]
pub struct Args {
    /// Enable hardware acceleration
    #[clap(long)]
    hardware_acceleration: bool,

    /// Launch devtools
    #[clap(long)]
    devtools: bool,

    /// Port to listen on
    #[clap(long, default_value = "50051")]
    port: u16,

    /// Parent process (will exit if parent process exits)
    #[clap(long)]
    parent_process: Option<u32>,
}

fn main() -> anyhow::Result<()> {
    // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
    //     .target(env_logger::Target::Stderr)
    //     .init();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_thread_ids(true)
                .with_thread_names(true),
        )
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    color_backtrace::install();

    let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);

    let args = cef::args::Args::new();
    let options = build_render_options();
    let is_browser_process = prepare_process(&args)?;
    if !is_browser_process {
        tracing::info!("Initialized as a secondary process, exiting main.");
        return Ok(());
    }

    let cli_args = Args::parse();
    let mut settings = build_settings();
    // CEF's singleton uses root_cache_path even when cache_path is empty.
    // Each server owns an isolated directory, removed after the shutdown guard.
    let profile_dir = tempfile::Builder::new()
        .prefix("web-render-cef-")
        .tempdir()?;
    settings.root_cache_path = cef::CefString::from(profile_dir.path().to_string_lossy().as_ref());
    settings.remote_debugging_port = if cli_args.devtools { 5151 } else { 0 };
    let _shutdown_guard = initialize_cef(&args, &settings)?;

    let gpu = if cli_args.hardware_acceleration {
        Some(Arc::new(GpuCapture::new()?))
    } else {
        None
    };

    let mut client = create_client(
        &options,
        gpu,
        crate::render_loop::on_software_paint,
        crate::render_loop::on_accelerated_paint,
    );
    let browser = create_browser(&mut client, cli_args.hardware_acceleration)?;
    let render_loop = RenderLoop::new(browser);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(main_server(
            render_loop,
            cli_args.port,
            cli_args.parent_process,
        ))?;
    Ok(())
}

pub async fn main_server(
    render_loop: RenderLoop,
    port: u16,
    parent_pid: Option<u32>,
) -> anyhow::Result<()> {
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::unbounded_channel();
    let shutdown_tx = Arc::new(shutdown_tx);
    if let Some(ppid) = parent_pid {
        let shutdown_tx_clone = shutdown_tx.clone();
        tokio::spawn(async move {
            parent_process::watch(ppid, shutdown_tx_clone).await;
        });
    }
    let server = server::MainServer::new(render_loop, shutdown_tx);
    let addr = format!("[::1]:{}", port).parse().unwrap();
    tracing::info!("Starting gRPC server on {}", addr);
    let service = tonic::transport::Server::builder()
        .max_frame_size((1 << 24) - 1)
        .add_service(
            crate::protocol::libserver::lib_server_server::LibServerServer::new(server)
                .send_compressed(tonic::codec::CompressionEncoding::Gzip)
                .max_decoding_message_size(usize::MAX),
        )
        .add_service(
            tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(
                    crate::protocol::libserver::FILE_DESCRIPTOR_SET,
                )
                .build_v1()
                .unwrap(),
        )
        .serve_with_shutdown(addr, async {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Shutting down gRPC server");
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Received shutdown request");
                }
            }
        });
    tokio::pin!(service);
    // The current-thread Tokio runtime stays on the thread that initialized CEF.
    // Pump continuously, including idle periods and notification-only updates.
    let mut pump = tokio::time::interval(std::time::Duration::from_millis(5));
    loop {
        tokio::select! {
            result = &mut service => { result?; break; }
            _ = pump.tick() => cef::do_message_loop_work(),
        }
    }

    Ok(())
}
