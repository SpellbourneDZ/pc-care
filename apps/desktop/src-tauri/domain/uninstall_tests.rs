use super::*;
use crate::domain::{
    contracts::{ApiError, ApiRequest},
    engine::Engine,
};
fn app() -> InstalledApp {
    InstalledApp {
        id: "hkcu64:fixture".into(),
        name: "Artificial MSI".into(),
        publisher: Some("Fixture Publisher".into()),
        version: Some("1.2.3".into()),
        kind: AppKind::Msi,
        source: AppSource::Hkcu64,
        system_component: Some(false),
        estimated_bytes: None,
        registered_date: None,
        install_location: None,
        registration: "{62e06e1f-50af-4c56-a5c8-2c8e2fbea775}".into(),
        uninstall_block: UninstallBlock::NotValidated,
    }
}
fn product() -> MsiProduct {
    MsiProduct {
        context: MsiContext::UserUnmanaged,
        state: "5".into(),
        name: "Artificial MSI".into(),
        publisher: "Fixture Publisher".into(),
        version: "1.2.3".into(),
    }
}
#[test]
fn only_an_exact_nonnil_product_guid_can_become_a_controlled_argument() {
    let mut a = app();
    let preview = msi_preview(&a, &[product()]).unwrap();
    assert_eq!(
        preview.arguments,
        vec!["/x", "{62E06E1F-50AF-4C56-A5C8-2C8E2FBEA775}", "/norestart"]
    );
    for input in [
        "62e06e1f-50af-4c56-a5c8-2c8e2fbea775",
        "{00000000-0000-0000-0000-000000000000}",
        "{62e06e1f-50af-4c56-a5c8-2c8e2fbea775} /quiet",
        "{62e06e1f50af4c56a5c82c8e2fbea775}",
        "C:\\fixture.msi",
        "{🦄🦄🦄🦄🦄🦄🦄🦄🦄}",
    ] {
        a.registration = input.into();
        assert_eq!(
            product_code(&a).unwrap_err(),
            UninstallCheckStatus::InvalidIdentity
        );
    }
    let mut a = app();
    a.system_component = None;
    assert_eq!(
        product_code(&a).unwrap_err(),
        UninstallCheckStatus::Protected
    );
    let mut a = app();
    a.kind = AppKind::Win32;
    assert_eq!(
        product_code(&a).unwrap_err(),
        UninstallCheckStatus::Unsupported
    );
}
#[test]
fn installed_context_and_all_identity_metadata_must_match_without_ambiguity() {
    let a = app();
    assert_eq!(
        msi_preview(&a, &[]).unwrap_err(),
        UninstallCheckStatus::NotInstalled
    );
    assert_eq!(
        msi_preview(&a, &[product(), product()]).unwrap_err(),
        UninstallCheckStatus::AmbiguousContext
    );
    let mut p = product();
    p.context = MsiContext::Machine;
    assert_eq!(
        msi_preview(&a, &[p]).unwrap_err(),
        UninstallCheckStatus::AmbiguousContext
    );
    let mut p = product();
    p.state = "1".into();
    assert_eq!(
        msi_preview(&a, &[p]).unwrap_err(),
        UninstallCheckStatus::NotInstalled
    );
    for field in [0, 1, 2] {
        let mut p = product();
        match field {
            0 => p.name = "Same name, different product".into(),
            1 => p.publisher = String::new(),
            _ => p.version = "2".into(),
        };
        assert_eq!(
            msi_preview(&a, &[p]).unwrap_err(),
            UninstallCheckStatus::MetadataMismatch
        )
    }
}
#[test]
fn checks_own_the_snapshot_and_ipc_never_accepts_a_path_or_launch_args() {
    let engine = Engine::new(false);
    let mut s = AppSnapshot::new().unwrap();
    let a = app();
    s.push(a.clone());
    let catalog = engine.publish_apps(s).unwrap();
    assert_eq!(engine.app(catalog.id, &a.id).unwrap(), a);
    assert!(matches!(
        engine.app(catalog.id, "foreign"),
        Err(ApiError::NotFound)
    ));
    engine.publish_apps(AppSnapshot::new().unwrap()).unwrap();
    assert!(matches!(
        engine.app(catalog.id, &a.id),
        Err(ApiError::NotFound)
    ));
    for text in [
        r#"{"type":"check_uninstall","data":{"snapshot_id":"00000000-0000-0000-0000-000000000000","app_id":"x","arguments":["/quiet"]}}"#,
        r#"{"type":"launch_uninstall","data":{"app_id":"x"}}"#,
    ] {
        assert!(serde_json::from_str::<ApiRequest>(text).is_err())
    }
}
