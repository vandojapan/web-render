//! Compare host captures with the previously verified, independent GPU reference.
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(
        args.len() == 2,
        "validate_native_host <host-proof> <reference-proof>"
    );
    let proof = std::path::PathBuf::from(&args[0]);
    let baseline = std::path::PathBuf::from(&args[1]);
    let mut comparisons = Vec::new();
    for (mode, suffix) in [("default", "default"), ("no-smooth", "no-smooth")] {
        for scene in ["shapes", "text", "image"] {
            let actual = image::open(proof.join(format!("{scene}-{suffix}-00.png")))?.into_rgba8();
            let mut expected = std::fs::read(baseline.join(format!("{mode}-1920-{scene}.rgba")))?;
            web_render_processing::straight_rgba(&mut expected);
            let changed = actual
                .as_raw()
                .iter()
                .zip(&expected)
                .filter(|(a, b)| a != b)
                .count();
            anyhow::ensure!(
                actual.as_raw().len() == expected.len() && changed == 0,
                "{scene} {mode}: {changed} bytes differ"
            );
            comparisons.push(serde_json::json!({"scene":scene,"mode":mode,"different_bytes":changed,"bytes":expected.len()}));
        }
    }
    let report = serde_json::json!({"status":"passed","host_matches_independent_gpu":comparisons});
    std::fs::write(
        proof.join("pixel-reference.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("All six host captures exactly match independent GPU references");
    Ok(())
}
