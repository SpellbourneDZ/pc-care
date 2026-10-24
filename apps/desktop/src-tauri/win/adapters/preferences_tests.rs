use super::*;

struct Fixture {
    root: PathBuf,
    config: PathBuf,
    store: SettingsStore,
}
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target/policy-fixtures")
            .join(Uuid::new_v4().to_string());
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let config = root.join("config");
        Self {
            root,
            store: SettingsStore::new(Some(config.clone())),
            config,
        }
    }
    fn write(&self, bytes: &[u8]) {
        fs::create_dir_all(&self.config).unwrap();
        fs::write(self.config.join("settings.json"), bytes).unwrap();
    }
}

#[test]
fn defaults_read_without_creating_config_and_preferences_survive_restart() {
    let f = Fixture::new();
    assert_eq!(f.store.load().unwrap(), defaults());
    assert!(!f.config.exists());
    let wanted = Preferences {
        theme: Theme::Dark,
        language: Language::Ru,
        exclusions: vec![r"C:\пример\folder".into(), r"c:\ПРИМЕР\folder".into()],
    };
    let saved = f.store.save(None, wanted).unwrap();
    assert!(saved.revision.is_some());
    assert_eq!(saved.preferences.exclusions.len(), 1);
    let restarted = SettingsStore::new(Some(f.config));
    assert_eq!(restarted.load().unwrap(), saved);
    let mut edited = saved.preferences.clone();
    edited.theme = Theme::Light;
    let next = restarted.save(saved.revision, edited).unwrap();
    assert_ne!(saved.revision, next.revision);
    assert_eq!(restarted.load().unwrap(), next);
}

#[test]
fn legacy_migrates_in_memory_and_only_explicit_save_replaces_it() {
    let f = Fixture::new();
    let original = br#"{"schema_version":0,"theme":"dark","exclusions":["C:\\example"]}"#;
    f.write(original);
    let loaded = f.store.load().unwrap();
    assert_eq!(loaded.preferences.theme, Theme::Dark);
    assert_eq!(loaded.preferences.language, Language::Ru);
    assert_eq!(fs::read(f.config.join("settings.json")).unwrap(), original);
    assert_eq!(loaded.revision, Some(SettingsRevision(Uuid::nil())));
    let saved = f.store.save(loaded.revision, loaded.preferences).unwrap();
    assert_ne!(saved.revision, loaded.revision);
    assert_eq!(f.store.load().unwrap(), saved);
}

#[test]
fn malformed_or_future_settings_are_not_reset_or_overwritten() {
    for (bytes, error) in [
        (b"{invalid".as_slice(), ApiError::SettingsInvalid),
        (
            br#"{"schema_version":2,"future":"data"}"#.as_slice(),
            ApiError::Unsupported,
        ),
        (
            br#"{"schema_version":0,"theme":"dark","unknown":"data"}"#.as_slice(),
            ApiError::SettingsInvalid,
        ),
        (
            br#"{"schema_version":0,"theme":"dark","exclusions":["C:\\a\\..\\Windows"]}"#
                .as_slice(),
            ApiError::SettingsInvalid,
        ),
    ] {
        let f = Fixture::new();
        f.write(bytes);
        assert_eq!(f.store.load(), Err(error));
        assert_eq!(f.store.save(None, Preferences::default()), Err(error));
        assert_eq!(fs::read(f.config.join("settings.json")).unwrap(), bytes);
    }
}

#[test]
fn invalid_exclusions_and_oversized_files_fail_closed() {
    let f = Fixture::new();
    for path in [r"C:\x\..\Windows", r"\\server\share", "", r"C:\x:stream"] {
        assert_eq!(
            f.store.save(
                None,
                Preferences {
                    exclusions: vec![path.into()],
                    ..Preferences::default()
                }
            ),
            Err(ApiError::InvalidRequest)
        );
    }
    assert_eq!(
        f.store.save(
            None,
            Preferences {
                exclusions: vec![r"C:\x".into(); MAX_EXCLUSIONS + 1],
                ..Preferences::default()
            }
        ),
        Err(ApiError::InvalidRequest)
    );
    assert!(!f.config.exists());
    f.write(&vec![b' '; MAX_BYTES as usize + 1]);
    assert_eq!(f.store.load(), Err(ApiError::SettingsInvalid));
    assert_eq!(
        f.store.save(None, Preferences::default()),
        Err(ApiError::SettingsInvalid)
    );
    assert_eq!(
        fs::metadata(f.config.join("settings.json")).unwrap().len(),
        MAX_BYTES + 1
    );
}

#[test]
fn stale_revision_and_concurrent_saves_do_not_lose_updates() {
    let f = Fixture::new();
    let first = f.store.save(None, Preferences::default()).unwrap();
    assert_eq!(
        f.store.save(None, Preferences::default()),
        Err(ApiError::SettingsConflict)
    );
    let other = f.store.clone();
    let revision = first.revision;
    let remote = std::thread::spawn(move || {
        other.save(
            revision,
            Preferences {
                theme: Theme::Dark,
                ..Preferences::default()
            },
        )
    });
    let local = f.store.save(
        revision,
        Preferences {
            theme: Theme::Light,
            ..Preferences::default()
        },
    );
    let remote = remote.join().unwrap();
    assert_eq!(usize::from(local.is_ok()) + usize::from(remote.is_ok()), 1);
    assert!(
        matches!(local, Err(ApiError::SettingsConflict))
            || matches!(remote, Err(ApiError::SettingsConflict))
    );
}

#[test]
fn locked_store_and_unavailable_storage_are_visible() {
    let f = Fixture::new();
    fs::create_dir(&f.config).unwrap();
    let lock = lock_file(&f.config).unwrap();
    assert_eq!(
        f.store.save(None, Preferences::default()),
        Err(ApiError::SettingsUnavailable)
    );
    assert!(!f.config.join("settings.json").exists());
    drop(lock);
    f.store.save(None, Preferences::default()).unwrap();
    let unavailable = SettingsStore::new(None);
    assert_eq!(unavailable.load(), Err(ApiError::SettingsUnavailable));
    assert_eq!(
        unavailable.save(None, Preferences::default()),
        Err(ApiError::SettingsUnavailable)
    );
}

#[test]
fn config_reparse_and_hardlink_targets_cannot_redirect_storage() {
    let f = Fixture::new();
    fs::create_dir(&f.config).unwrap();
    let outside = f.root.join("outside");
    fs::create_dir(&outside).unwrap();
    let link = f.root.join("junction");
    super::super::files::tests::junction(&link, &outside);
    let redirected = SettingsStore::new(Some(link));
    assert_eq!(redirected.load(), Err(ApiError::SettingsUnavailable));
    assert_eq!(
        redirected.save(None, Preferences::default()),
        Err(ApiError::SettingsUnavailable)
    );
    assert!(!outside.join("settings.json").exists());
    let target = f.root.join("artificial-file.txt");
    fs::write(&target, b"not configuration").unwrap();
    fs::hard_link(&target, f.config.join("settings.json")).unwrap();
    assert_eq!(f.store.load(), Err(ApiError::SettingsUnavailable));
    assert_eq!(
        f.store.save(None, Preferences::default()),
        Err(ApiError::SettingsUnavailable)
    );
    assert_eq!(fs::read(target).unwrap(), b"not configuration");
}

#[test]
fn failed_atomic_replace_preserves_the_previous_settings() {
    let f = Fixture::new();
    let original = f.store.save(None, Preferences::default()).unwrap();
    let bytes = fs::read(f.config.join("settings.json")).unwrap();
    let reader = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(f.config.join("settings.json"))
        .unwrap();
    assert_eq!(
        f.store.save(
            original.revision,
            Preferences {
                theme: Theme::Dark,
                ..Preferences::default()
            }
        ),
        Err(ApiError::SettingsUnavailable)
    );
    drop(reader);
    assert_eq!(fs::read(f.config.join("settings.json")).unwrap(), bytes);
    assert_eq!(f.store.load().unwrap(), original);
}

#[test]
fn unknown_preferences_ipc_fields_and_language_are_rejected() {
    for raw in [
        r#"{"type":"save_preferences","data":{"expected_revision":null,"preferences":{"theme":"dark","language":"ru","exclusions":[]},"config_path":"C:\\Windows"}}"#,
        r#"{"type":"save_preferences","data":{"expected_revision":null,"preferences":{"theme":"dark","language":"unknown","exclusions":[]}}}"#,
    ] {
        assert!(serde_json::from_str::<ApiRequest>(raw).is_err());
    }
}
