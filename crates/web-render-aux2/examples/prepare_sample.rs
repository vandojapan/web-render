//! Prepare a COPY of an empty AviUtl2 sample with the pinned SDK's serialization format.
use base64::Engine;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(
        args.len() == 3,
        "prepare_sample <source.aup2> <copy.aup2> <web-project>"
    );
    let input = std::fs::canonicalize(PathBuf::from(&args[0]))?;
    let output = std::path::absolute(PathBuf::from(&args[1]))?;
    anyhow::ensure!(input != output, "Refusing to overwrite the source project");
    anyhow::ensure!(
        !output.exists(),
        "Output already exists; choose a new copy path"
    );
    let project = std::fs::canonicalize(PathBuf::from(&args[2]))?;
    let project_text = project.to_string_lossy();
    let project = if let Some(unc) = project_text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        project_text.trim_start_matches(r"\\?\").to_string()
    };
    let bytes = rmp_serde::to_vec_named(&project)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let source = std::fs::read_to_string(&input)?;
    anyhow::ensure!(
        !source.lines().any(|line| line
            .strip_prefix('[')
            .and_then(|name| name.strip_suffix(']'))
            .is_some_and(|name| name
                .split('.')
                .next()
                .unwrap_or("")
                .parse::<usize>()
                .is_ok())),
        "Use an empty sample; existing objects must be preserved"
    );
    let mut result = String::new();
    let mut sections: Vec<Vec<&str>> = Vec::new();
    for line in source.lines() {
        if line.starts_with('[') || sections.is_empty() {
            sections.push(Vec::new());
        }
        sections.last_mut().unwrap().push(line);
    }
    let mut plugin_index = 0;
    let mut replaced_index = None;
    for section in sections {
        let header = section.first().copied().unwrap_or("");
        if let Some(index) = header
            .strip_prefix("[plugin.")
            .and_then(|v| v.strip_suffix(']'))
        {
            plugin_index = plugin_index.max(index.parse::<usize>()? + 1);
            if section.contains(&"plugin.name=web-render") {
                replaced_index = Some(index.parse::<usize>()?);
                continue;
            }
        }
        for line in section {
            if header == "[project]" && line.starts_with("file=") {
                result.push_str(&format!("file={}\n", output.display()));
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }
    }
    let plugin_index = replaced_index.unwrap_or(plugin_index);
    result.push_str(&format!(
        "[plugin.{plugin_index}]\nplugin.name=web-render\nproject_dir=--aviutl2-rs:serde-rmp-base64-v1:{}\n--aviutl2-rs:serde-base64-chunk:project_dir:0={encoded}\n",
        bytes.len()
    ));
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output, result)?;
    println!("Sample copy: {}\nWeb project: {project}", output.display());
    Ok(())
}
