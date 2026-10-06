fn main() -> anyhow::Result<()> {
    if let Ok(runtime) = std::env::var("DEP_CEF_DLL_WRAPPER_CEF_DIR") {
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
        let profile_dir = out
            .ancestors()
            .nth(3)
            .ok_or_else(|| anyhow::anyhow!("Invalid OUT_DIR"))?;
        std::fs::write(profile_dir.join("cef-runtime-path.txt"), runtime)?;
    }
    println!("cargo:rerun-if-changed=../../protocol/lib-server.proto");
    println!("cargo:rerun-if-changed=../../protocol/common.proto");
    println!("cargo:rerun-if-changed=../../protocol/server-js.proto");
    let mut config = prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    tonic_prost_build::configure()
        .build_server(true)
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap())
                .join("libserver_descriptor.bin"),
        )
        .compile_with_config(
            config,
            &[
                "../../protocol/lib-server.proto",
                "../../protocol/common.proto",
            ],
            &["../../protocol"],
        )?;
    let mut config = prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    config.compile_protos(
        &[
            "../../protocol/server-js.proto",
            "../../protocol/common.proto",
        ],
        &["../../protocol"],
    )?;
    Ok(())
}
