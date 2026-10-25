use super::*;
use crate::domain::{contracts::ApiRequest, engine::Engine};
fn record(source: StartupSource, name: &str) -> StartupEntry {
    StartupEntry {
        id: 0,
        name: name.into(),
        source,
        scope: source.scope(),
        path: r"C:\fixture".into(),
        command: Some("untrusted command --never-executed".into()),
        publisher: None,
        enabled: None,
        can_toggle: false,
        can_reveal: false,
        location: None,
    }
}
fn query() -> StartupQuery {
    StartupQuery {
        search: String::new(),
        source: None,
        scope: None,
        offset: 0,
        limit: 100,
    }
}
#[test]
fn registry_text_preserves_raw_quotes_arguments_unicode_and_environment_without_execution() {
    let text = r#""%APPDATA%\Искусственная программа.exe" --name "two words" & echo never"#;
    let bytes: Vec<u8> = text
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(registry_string(&bytes).unwrap(), text);
    assert_eq!(registry_string(&bytes[..bytes.len() - 2]).unwrap(), text);
    for bytes in [vec![1], vec![0, 0, 65, 0], vec![0x00, 0xd8], vec![1; 65538]] {
        assert!(registry_string(&bytes).is_err())
    }
}
#[test]
fn immutable_inventory_filters_pages_and_rejects_mutation_and_foreign_ids() {
    let mut snapshot = StartupSnapshot::new(true).unwrap();
    snapshot.push(record(StartupSource::HkcuRun64, "Личная программа"));
    snapshot.push(record(StartupSource::HklmRun32, "Общая программа"));
    let mut q = query();
    q.scope = Some(StartupScope::Machine);
    let page = snapshot.page(q).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.entries[0].name, "Общая программа");
    assert_eq!(snapshot.catalog.total, 2);
    let mut q = query();
    q.search = "НИКОГДА".into();
    assert!(snapshot.page(q).unwrap().entries.is_empty());
    let mut q = query();
    q.search = "never-executed".into();
    assert_eq!(snapshot.page(q).unwrap().total, 2);
    let mut q = query();
    q.limit = 501;
    assert!(snapshot.page(q).is_err());
    let engine = Engine::new(false);
    let first = engine.publish_startup(snapshot).unwrap();
    assert!(engine.startup_page(first.id, query()).is_ok());
    assert!(matches!(
        engine.startup_location(first.id, 0),
        Err(ApiError::Unsupported)
    ));
    engine
        .publish_startup(StartupSnapshot::new(true).unwrap())
        .unwrap();
    assert!(matches!(
        engine.startup_page(first.id, query()),
        Err(ApiError::NotFound)
    ));
    for text in [
        r#"{"type":"set_startup_state","data":{"id":0,"enabled":false}}"#,
        r#"{"type":"refresh_startup","data":{"key":"HKLM/anything"}}"#,
    ] {
        assert!(serde_json::from_str::<ApiRequest>(text).is_err());
    }
}
#[test]
fn inventory_caps_are_visible_and_private_location_is_never_exported() {
    let mut snapshot = StartupSnapshot::new(true).unwrap();
    for index in 0..=MAX_ENTRIES {
        snapshot.push(record(
            StartupSource::HkcuRun64,
            &format!("fixture-{index}"),
        ));
    }
    assert_eq!(snapshot.catalog.total, MAX_ENTRIES as u32);
    assert!(snapshot.catalog.truncated);
    assert!(
        snapshot
            .catalog
            .issues
            .iter()
            .any(|i| i.reason == StartupIssueReason::LimitReached)
    );
    let wire = serde_json::to_value(&snapshot.entries[0]).unwrap();
    assert!(wire.get("location").is_none());
    assert_eq!(wire["enabled"], serde_json::Value::Null);
    assert_eq!(wire["can_toggle"], false);
}
