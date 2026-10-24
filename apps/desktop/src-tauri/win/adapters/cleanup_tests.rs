use super::*;
use std::{
    fs,
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::{Path, PathBuf},
};
use windows_sys::Win32::Storage::FileSystem::*;

struct Fixture {
    root: PathBuf,
    scanner: CleanupScanner,
}
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target/policy-fixtures")
            .join(uuid::Uuid::new_v4().to_string());
        let profile = root.join("Profile");
        fs::create_dir_all(&profile).unwrap();
        let profile = profile.canonicalize().unwrap();
        let root = root.canonicalize().unwrap();
        let policy = FilePolicy::for_fixture(&profile).unwrap();
        for rule in RuleId::ALL {
            fs::create_dir_all(policy.root(rule).unwrap().as_str()).unwrap();
        }
        let now = 134_000_000_000_000_000;
        Self {
            root,
            scanner: CleanupScanner {
                policy,
                processes: Ok(HashSet::new()),
                now,
            },
        }
    }
    fn file(&self, rule: RuleId, name: &str, old: bool) -> PathBuf {
        let path = PathBuf::from(
            self.scanner
                .policy
                .root(rule)
                .unwrap()
                .child(name)
                .unwrap()
                .as_str(),
        );
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, vec![42; 65_536]).unwrap();
        if old {
            self.age(&path);
        }
        path
    }
    fn age(&self, path: &Path) {
        let file = fs::OpenOptions::new()
            .write(true)
            .access_mode(FILE_WRITE_ATTRIBUTES)
            .open(path)
            .unwrap();
        let old = self.scanner.now - 30 * 24 * 3600 * 10_000_000;
        let info = FILE_BASIC_INFO {
            CreationTime: old,
            LastWriteTime: old,
            ChangeTime: old,
            ..Default::default()
        };
        assert_ne!(
            unsafe {
                SetFileInformationByHandle(
                    file.as_raw_handle(),
                    FileBasicInfo,
                    (&info as *const FILE_BASIC_INFO).cast(),
                    std::mem::size_of::<FILE_BASIC_INFO>() as u32,
                )
            },
            0
        );
    }
    fn scan(&self) -> (Vec<Candidate>, Vec<ScanIssue>) {
        let mut candidates = Vec::new();
        let mut issues = Vec::new();
        self.scanner
            .scan(&mut |event| {
                match event {
                    ScanEvent::Candidate(candidate) => candidates.push(candidate),
                    ScanEvent::Issue(issue) => issues.push(issue),
                    _ => {}
                }
                Ok(())
            })
            .unwrap();
        (candidates, issues)
    }
}

#[test]
fn candidate_publication_holds_reservation_and_releases_it_on_sink_error() {
    for reject in [false, true] {
        let f = Fixture::new();
        let path = f.file(RuleId::UserTemp, "reserved/data.tmp", true);
        let mut published = 0;
        let result = f.scanner.scan(&mut |event| {
            if let ScanEvent::Candidate(_) = event {
                published += 1;
                let error = fs::OpenOptions::new()
                    .access_mode(FILE_WRITE_DATA)
                    .open(&path)
                    .unwrap_err();
                assert_eq!(
                    error.raw_os_error(),
                    Some(windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION as i32)
                );
                if reject {
                    return Err(ApiError::Disconnected);
                }
            }
            Ok(())
        });
        assert_eq!(published, 1);
        assert_eq!(
            result,
            if reject {
                Err(ApiError::Disconnected)
            } else {
                Ok(())
            }
        );
        drop(
            fs::OpenOptions::new()
                .access_mode(FILE_WRITE_DATA)
                .open(&path)
                .unwrap(),
        );
        assert_eq!(fs::read(path).unwrap(), vec![42; 65_536]);
    }
}

#[test]
fn three_allowlisted_sources_report_allocation_and_leave_bytes_unchanged() {
    let f = Fixture::new();
    let paths: Vec<_> = RuleId::ALL
        .into_iter()
        .map(|rule| f.file(rule, "подкаталог/данные.tmp", true))
        .collect();
    let before: Vec<_> = paths.iter().map(|path| fs::read(path).unwrap()).collect();
    let (candidates, issues) = f.scan();
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(candidates.len(), 3);
    for candidate in candidates {
        assert_eq!(candidate.logical_bytes, ByteCount(65_536));
        assert!(candidate.bytes.0 > 0);
        assert!(candidate.identity.contains("allocated_size"));
        assert_eq!(candidate.source, ScanKind::Cleanup);
    }
    for (index, path) in paths.iter().enumerate() {
        assert_eq!(fs::read(path).unwrap(), before[index]);
    }
}

#[test]
fn recent_excluded_running_and_browser_state_are_not_candidates() {
    let mut f = Fixture::new();
    f.file(RuleId::UserTemp, "recent.bin", false);
    let excluded = f.file(RuleId::UserTemp, "excluded/old.bin", true);
    f.scanner
        .policy
        .exclusions
        .add(excluded.parent().unwrap().to_str().unwrap())
        .unwrap();
    f.file(RuleId::ChromeHttpCache, "old.bin", true);
    f.scanner.processes = Ok(HashSet::from(["chrome.exe".into()]));
    let (candidates, issues) = f.scan();
    assert!(candidates.is_empty());
    for reason in [
        Denial::TooRecent,
        Denial::Excluded,
        Denial::ApplicationRunning,
    ] {
        assert!(issues.iter().any(|issue| issue.reason == reason));
    }
    for source in [RuleId::ChromeHttpCache, RuleId::EdgeHttpCache] {
        let root = f.scanner.policy.root(source).unwrap();
        let profile = root.as_str().strip_suffix(r"\Cache").unwrap();
        for protected in [
            "Cookies",
            "Login Data",
            "Sessions",
            "Network",
            "Local Storage",
            "IndexedDB",
        ] {
            let path = LocalPath::parse(&format!(r"{profile}\{protected}\file")).unwrap();
            assert_eq!(
                f.scanner.policy.check_path(source, &path),
                Err(Denial::OutsideScope)
            );
        }
    }
}

#[test]
fn locked_acl_hardlink_and_actual_junctions_are_visible_without_following_them() {
    let f = Fixture::new();
    let locked = f.file(RuleId::UserTemp, "locked.bin", true);
    let _lock = fs::OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(&locked)
        .unwrap();
    let hard = f.file(RuleId::UserTemp, "hard.bin", true);
    fs::hard_link(&hard, hard.with_extension("link")).unwrap();
    let denied = f.file(RuleId::UserTemp, "denied/file.bin", true);
    let _parent_acl =
        super::super::files::tests::deny_acl(denied.parent().unwrap(), FILE_DELETE_CHILD);
    let _file_acl = super::super::files::tests::deny_acl(&denied, DELETE);
    let outside = f.root.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("untouched.bin"), b"artificial outside data").unwrap();
    let link = f.root.join("Profile/AppData/Local/Temp/junction");
    super::super::files::tests::junction(&link, &outside);
    let (candidates, issues) = f.scan();
    assert!(candidates.is_empty());
    for reason in [
        Denial::Locked,
        Denial::AccessDenied,
        Denial::HardLink,
        Denial::ReparsePoint,
    ] {
        assert!(
            issues.iter().any(|issue| issue.reason == reason),
            "{issues:?}"
        );
    }
    assert_eq!(
        fs::read(outside.join("untouched.bin")).unwrap(),
        b"artificial outside data"
    );
}

#[test]
fn changed_target_before_publication_is_skipped() {
    let f = Fixture::new();
    let path = f.file(RuleId::UserTemp, "changed.tmp", true);
    let mut issues = Vec::new();
    f.scanner
        .file(
            RuleId::UserTemp,
            &LocalPath::parse(path.to_str().unwrap()).unwrap(),
            &mut |event| {
                match event {
                    ScanEvent::Checkpoint => {
                        fs::write(&path, b"changed artificial fixture").unwrap()
                    }
                    ScanEvent::Candidate(_) => panic!("changed target published"),
                    ScanEvent::Issue(issue) => issues.push(issue),
                    _ => {}
                }
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(issues[0].reason, Denial::TargetChanged);
}

#[test]
fn compressed_allocation_is_distinct_from_logical_size() {
    use windows_sys::Win32::System::{IO::DeviceIoControl, Ioctl::FSCTL_SET_COMPRESSION};
    let f = Fixture::new();
    let path = f.file(RuleId::UserTemp, "compressed.tmp", false);
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let format: u16 = 1; // COMPRESSION_FORMAT_DEFAULT; only an artificial NTFS fixture.
    let mut returned = 0;
    assert_ne!(
        unsafe {
            DeviceIoControl(
                file.as_raw_handle(),
                FSCTL_SET_COMPRESSION,
                (&format as *const u16).cast(),
                2,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
            )
        },
        0,
        "NTFS compression fixture: {:?}",
        std::io::Error::last_os_error()
    );
    drop(file);
    f.age(&path);
    let (candidates, issues) = f.scan();
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(candidates.len(), 1);
    assert!(candidates[0].bytes.0 < candidates[0].logical_bytes.0);
    assert_eq!(fs::read(path).unwrap(), vec![42; 65_536]);
}

#[test]
fn directory_probe_pins_ancestors_without_blocking_child_file_creation() {
    let f = Fixture::new();
    let root = f.scanner.policy.root(RuleId::UserTemp).unwrap();
    let guard = f.scanner.policy.directory(RuleId::UserTemp, &root).unwrap();
    let open_writer = || {
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(root.as_str())
    };
    assert!(open_writer().is_err());
    let child = PathBuf::from(root.child("new-fixture.bin").unwrap().as_str());
    fs::write(&child, b"artificial child").unwrap();
    drop(guard);
    open_writer().unwrap();
}

#[test]
fn portable_browser_profiles_sensitive_sidecars_and_unverified_temp_types_stay_protected() {
    let f = Fixture::new();
    let payload = f.file(RuleId::UserTemp, "portable/Default/payload.tmp", true);
    fs::write(
        payload.parent().unwrap().join("Preferences"),
        b"artificial browser marker",
    )
    .unwrap();
    let sidecar = f.file(RuleId::UserTemp, "COOKIES.SQLITE-WAL.TMP", true);
    for name in [
        "key4.db.tmp",
        "logins.json.temp",
        "formhistory.sqlite-wal.tmp",
        "sessionstore.jsonlz4.bak.tmp",
    ] {
        f.file(RuleId::UserTemp, name, true);
    }
    let unknown = f.file(RuleId::UserTemp, "personal-document.pdf", true);
    let cache = f.file(RuleId::ChromeHttpCache, "entry", true);
    let cache_root = PathBuf::from(
        f.scanner
            .policy
            .root(RuleId::ChromeHttpCache)
            .unwrap()
            .as_str(),
    );
    fs::write(
        cache_root.parent().unwrap().join("Preferences"),
        b"artificial parent outside cache scope",
    )
    .unwrap();
    let (candidates, issues) = f.scan();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].display_path, cache.to_str().unwrap());
    assert!(
        issues
            .iter()
            .any(|issue| issue.reason == Denial::SensitiveData)
    );
    assert!(
        issues
            .iter()
            .any(|issue| issue.reason == Denial::UnverifiedType)
    );
    for path in [payload, sidecar, unknown] {
        assert_eq!(fs::read(path).unwrap(), vec![42; 65_536]);
    }
}

#[test]
fn browser_marker_added_after_snapshot_revokes_scope() {
    let f = Fixture::new();
    let path = f.file(RuleId::UserTemp, "new-profile/data.tmp", true);
    let id = CandidateId::new();
    let snapshot = f
        .scanner
        .policy
        .snapshot(id, RuleId::UserTemp, path.to_str().unwrap())
        .unwrap();
    fs::write(
        path.parent().unwrap().join("prefs.js"),
        b"artificial browser marker",
    )
    .unwrap();
    assert_eq!(
        f.scanner.policy.revalidate(id, &snapshot),
        Err(Denial::SensitiveData)
    );
}

#[test]
fn failed_process_probe_closes_both_caches_without_losing_temp_results() {
    let mut f = Fixture::new();
    f.file(RuleId::UserTemp, "eligible.tmp", true);
    f.file(RuleId::ChromeHttpCache, "entry", true);
    f.scanner.processes = Err(Denial::Unavailable);
    let (candidates, issues) = f.scan();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].rule_id, "user-temp-v1");
    assert_eq!(
        issues
            .iter()
            .filter(|issue| issue.reason == Denial::Unavailable)
            .count(),
        2
    );
}
