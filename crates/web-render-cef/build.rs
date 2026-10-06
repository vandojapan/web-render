fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../../protocol/lib-server.proto");
    println!("cargo:rerun-if-changed=../../protocol/common.proto");
    let mut config = prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_with_config(
            config,
            &[
                "../../protocol/lib-server.proto",
                "../../protocol/common.proto",
            ],
            &["../../protocol"],
        )?;
    Ok(())
}
