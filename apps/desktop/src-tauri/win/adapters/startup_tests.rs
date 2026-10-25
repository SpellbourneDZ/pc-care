use super::*;
use std::{fs, path::PathBuf};

#[cfg(not(feature = "qa-fixtures"))]
#[test]
fn actual_closed_registry_views_are_queried_read_only_without_claiming_activation() {
    let mut snapshot = StartupSnapshot::new(false).unwrap();
    for source in [
        StartupSource::HkcuRun64,
        StartupSource::HkcuRun32,
        StartupSource::HklmRun64,
        StartupSource::HklmRun32,
    ] {
        registry_entries(&mut snapshot, source)
    }
    assert!(
        snapshot
            .entries
            .iter()
            .all(|entry| entry.enabled.is_none() && !entry.can_toggle && !entry.can_reveal)
    );
    assert!(snapshot.catalog.total <= MAX_ENTRIES as u32);
}
#[test]
fn registry_fixture_decodes_strings_and_reports_unsupported_or_malformed_values() {
    let mut snapshot = StartupSnapshot::new(true).unwrap();
    let source = StartupSource::HkcuRun32;
    let text = r#""C:\fixture\test.exe" --literal "a b""#;
    let data: Vec<u8> = text
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect();
    assert!(registry_value(
        &mut snapshot,
        source,
        "Сохранить кавычки".into(),
        REG_EXPAND_SZ,
        &data
    ));
    assert_eq!(snapshot.entries[0].command.as_deref(), Some(text));
    assert!(!snapshot.entries[0].can_toggle);
    registry_value(
        &mut snapshot,
        source,
        "wrong".into(),
        REG_DWORD,
        &[1, 2, 3, 4],
    );
    registry_value(&mut snapshot, source, "invalid".into(), REG_SZ, &[1]);
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.catalog.issues.len(), 2);
}
#[test]
fn startup_folder_is_metadata_only_rejects_hardlinks_and_respects_exclusions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/startup-fixtures")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let file = root.join("Искусственный ярлык.lnk");
    let data = b"not a real shortcut; never resolve or execute";
    fs::write(&file, data).unwrap();
    fs::write(root.join("desktop.ini"), b"artificial layout metadata").unwrap();
    fs::write(root.join("linked-a.lnk"), b"hardlinked fixture").unwrap();
    fs::hard_link(root.join("linked-a.lnk"), root.join("linked-b.lnk")).unwrap();
    let local = LocalPath::parse(root.to_str().unwrap()).unwrap();
    let mut snapshot = StartupSnapshot::new(true).unwrap();
    folder_entries(
        &mut snapshot,
        StartupSource::UserFolder,
        &local,
        &Exclusions::default(),
    );
    assert_eq!(snapshot.entries.len(), 1);
    let entry = &snapshot.entries[0];
    assert!(entry.can_reveal);
    assert!(entry.command.is_none());
    assert!(entry.enabled.is_none());
    assert!(entry.location.is_some());
    assert!(
        snapshot
            .catalog
            .issues
            .iter()
            .any(|i| i.reason == StartupIssueReason::UnsafePath && i.count == 2)
    );
    assert_eq!(fs::read(&file).unwrap(), data);
    let mut excluded = Exclusions::default();
    excluded.add(local.as_str()).unwrap();
    let mut empty = StartupSnapshot::new(true).unwrap();
    folder_entries(&mut empty, StartupSource::UserFolder, &local, &excluded);
    assert!(empty.entries.is_empty());
    assert_eq!(fs::read(file).unwrap(), data);
}
