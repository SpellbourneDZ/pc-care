use super::contracts::{ApiReply, ApiRequest, ScanSnapshot, TaskProgress};
use std::path::Path;
use ts_rs::{Config, TS};

pub fn export(directory: impl AsRef<Path>) -> Result<(), ts_rs::ExportError> {
    let config = Config::default().with_out_dir(directory.as_ref());
    ApiRequest::export_all(&config)?;
    ApiReply::export_all(&config)?;
    ScanSnapshot::export_all(&config)?;
    TaskProgress::export_all(&config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, fs, path::PathBuf};

    fn files(directory: &Path) -> BTreeMap<String, String> {
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                (
                    path.file_name().unwrap().to_str().unwrap().to_owned(),
                    fs::read_to_string(path).unwrap().replace("\r\n", "\n"),
                )
            })
            .collect()
    }

    #[test]
    fn typescript_bindings_match_rust_contracts() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let generated = root
            .join("../../../target/bindings-check")
            .join(uuid::Uuid::new_v4().to_string());
        export(&generated).unwrap();
        assert_eq!(
            files(&generated),
            files(&root.join("../src/generated")),
            "Run pnpm types:generate after editing Rust contracts"
        );
    }
}
