use std::path::Path;

pub fn validate(root: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(
        root.is_dir(),
        "Webプロジェクトのフォルダがありません: {}",
        root.display()
    );
    if root.join(web_render_processing::MANIFEST).is_file() {
        web_render_processing::Project::load(root)?;
        return Ok(());
    }
    anyhow::ensure!(
        root.join("package.json").is_file(),
        "package.jsonがありません: {}。プラグインの展開先ではなく、examples/htmlなどWebプロジェクトのフォルダを選択してください。",
        root.display()
    );
    anyhow::ensure!(
        root.join("node_modules/web-render/dist/cli.mjs").is_file(),
        "web-renderの実行ファイルがありません: {}。このWebプロジェクトでnpm installを実行してください。",
        root.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_wrong_folder_and_missing_dependencies_before_starting_cef() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            validate(root.path())
                .unwrap_err()
                .to_string()
                .contains("package.json")
        );
        std::fs::write(root.path().join("package.json"), "{}").unwrap();
        assert!(
            validate(root.path())
                .unwrap_err()
                .to_string()
                .contains("npm install")
        );
        let runtime = root.path().join("node_modules/web-render/dist");
        std::fs::create_dir_all(&runtime).unwrap();
        std::fs::write(runtime.join("cli.mjs"), "").unwrap();
        assert!(validate(root.path()).is_ok());
        assert!(validate(&root.path().join("missing")).is_err());
    }
}
