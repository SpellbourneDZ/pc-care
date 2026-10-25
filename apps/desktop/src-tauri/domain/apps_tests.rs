use super::*;
use crate::domain::contracts::ApiRequest;
fn test_record(source: AppSource, key: &str, name: &str) -> InstalledApp {
    InstalledApp {
        id: format!("{}:{key}", source.tag()),
        name: name.into(),
        publisher: None,
        version: None,
        kind: AppKind::Win32,
        source,
        system_component: None,
        estimated_bytes: None,
        registered_date: None,
        install_location: None,
        registration: key.into(),
        uninstall_block: UninstallBlock::MissingMethod,
    }
}
fn query() -> AppQuery {
    AppQuery {
        search: String::new(),
        kind: None,
        source: None,
        sort: AppSort::Name,
        offset: 0,
        limit: 100,
    }
}
#[test]
fn identities_preserve_duplicate_names_and_pages_never_offer_uninstall() {
    let mut s = AppSnapshot::new().unwrap();
    let a = test_record(AppSource::Hkcu64, "a", "Same");
    let b = test_record(AppSource::Hklm64, "b", "Same");
    s.push(a.clone());
    s.push(b);
    s.push(a);
    assert_eq!(s.catalog.total, 2);
    let mut q = query();
    q.source = Some(AppSource::Hklm64);
    assert_eq!(s.page(q).unwrap().total, 1);
    let mut q = query();
    q.limit = 501;
    assert!(s.page(q).is_err());
    for text in [
        r#"{"type":"launch_uninstall","data":{"app_id":"a"}}"#,
        r#"{"type":"refresh_apps","data":{"key":"HKLM"}}"#,
    ] {
        assert!(serde_json::from_str::<ApiRequest>(text).is_err())
    }
}
#[test]
fn cap_and_unknown_size_are_explicit() {
    let mut s = AppSnapshot::new().unwrap();
    for i in 0..4097 {
        s.push(test_record(AppSource::Hkcu64, &i.to_string(), "Fixture"));
    }
    assert_eq!(s.catalog.total, 4096);
    assert!(s.catalog.truncated);
    assert!(s.entries[0].estimated_bytes.is_none());
}
