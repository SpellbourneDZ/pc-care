use super::*;
#[test]
fn unknown_command_capability_and_identity_do_not_depend_on_display_name() {
    let a = blank(AppSource::Hkcu64, "one", "same".into());
    let b = blank(AppSource::Hklm64, "one", "same".into());
    assert_ne!(a.id, b.id);
    assert_eq!(a.uninstall_block, UninstallBlock::MissingMethod);
    assert!(a.estimated_bytes.is_none());
    assert!(a.system_component.is_none());
    let error = io::Error::from(io::ErrorKind::InvalidInput);
    assert_eq!(io_reason(&error), AppIssueReason::UnsafePath);
}
#[cfg(not(feature = "qa-fixtures"))]
#[test]
fn real_current_user_and_registered_sources_are_read_only() {
    let snapshot = collect().unwrap();
    assert!(snapshot.entries.len() <= 4096);
    assert!(
        snapshot
            .entries
            .iter()
            .all(|e| !e.id.is_empty() && !e.name.is_empty())
    );
    assert!(
        snapshot
            .entries
            .iter()
            .filter(|e| e.source == AppSource::Msix)
            .all(|e| e.kind == AppKind::Msix)
    );
}
