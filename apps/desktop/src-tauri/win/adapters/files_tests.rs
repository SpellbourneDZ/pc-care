use super::*;
use std::{
    fs,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use uuid::Uuid;

struct Fixture {
    root: PathBuf,
    temp: PathBuf,
    policy: FilePolicy,
}
impl Fixture {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target/policy-fixtures")
            .join(Uuid::new_v4().to_string());
        let temp = root.join("Profile/AppData/Local/Temp");
        fs::create_dir_all(&temp).unwrap();
        let root = root.canonicalize().unwrap();
        let temp = temp.canonicalize().unwrap();
        let profile = LocalPath::parse(root.join("Profile").to_str().unwrap()).unwrap();
        let policy = FilePolicy {
            profile,
            temp: local(&temp),
            protected: vec![local(&root.join("CustomWindows"))],
            exclusions: Exclusions::default(),
        };
        Self { root, temp, policy }
    }
    fn file(&self, name: &str) -> PathBuf {
        let path = self.temp.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"artificial PC Care fixture").unwrap();
        path
    }
    fn snapshot(&self, path: &Path) -> Result<FileSnapshot, Denial> {
        self.policy.snapshot(
            CandidateId::new(),
            FileSource::UserTemp,
            path.to_str().unwrap(),
        )
    }
}
fn local(path: &Path) -> LocalPath {
    LocalPath::parse(path.to_str().unwrap()).unwrap()
}
fn denied<T>(value: Result<T, Denial>, expected: Denial) {
    match value {
        Err(actual) => assert_eq!(actual, expected),
        Ok(_) => panic!("expected {expected:?}, got success"),
    }
}

#[test]
fn paths_reject_namespaces_traversal_streams_and_ambiguous_names() {
    for path in [
        r"Windows\x",
        r"C:temp\x",
        r"\Windows",
        r"\\server\share\x",
        r"\\?\UNC\server\share\x",
        r"\\.\C:\x",
        r"\\?\GLOBALROOT\Device\x",
        r"C:\a\..\x",
        r"C:\a\.\x",
        r"C:\a\x:stream",
        r"C:\a\x.",
        r"C:\a\x ",
        r"C:\a\NUL.txt",
        r"C:\a\COM1.log",
        r"C:\a\LPT².txt",
        r"C:\a\*",
        "C:\\a\\x\0",
        r"C:\a\\x",
    ] {
        denied(LocalPath::parse(path), Denial::UnsupportedPath);
    }
    let a = LocalPath::parse(r"\\?\c:\папка\ФАЙЛ.txt").unwrap();
    let b = LocalPath::parse("C:/ПАПКА/файл.txt").unwrap();
    assert_eq!(a, b);
    assert!(a.within(&LocalPath::parse(r"C:\Папка").unwrap()));
    assert!(!a.within(&LocalPath::parse(r"C:\Пап").unwrap()));
    denied(
        LocalPath::parse(&format!("C:\\{}", "a\\".repeat(300))),
        Denial::UnsupportedPath,
    );
}

#[test]
fn all_protected_and_unknown_scope_fixtures_are_denied() {
    let f = Fixture::new();
    for path in [
        r"C:\",
        r"D:\",
        r"C:\wInDoWs\Installer\x.msi",
        r"D:\Windows\WinSxS\x",
        r"C:\Program Files\App\x",
        r"D:\Program Files (x86)\App\x",
        r"C:\ProgramData\x",
        r"C:\Recovery\x",
        r"C:\System Volume Information\x",
        r"C:\$Recycle.Bin\x",
        r"C:\Config.Msi\x",
    ] {
        denied(
            f.policy
                .check_path(FileSource::UserTemp, &LocalPath::parse(path).unwrap()),
            Denial::Protected,
        );
    }
    denied(
        f.policy.check_path(
            FileSource::UserTemp,
            &local(&f.root.join("CustomWindows/anything")),
        ),
        Denial::Protected,
    );
    for relative in [
        "OtherUser/AppData/Local/Temp/x",
        "Profile/Downloads/x",
        "Profile/Documents/x",
        "Profile/Desktop/x",
        "Profile/AppData/Local/Browser/User Data/Cookies",
        "Profile/AppData/Roaming/Browser/session",
        "Profile/AppData/Local/Temporary/x",
        "Profile/AppData/Local/Temp",
    ] {
        denied(
            f.policy
                .check_path(FileSource::UserTemp, &local(&f.root.join(relative))),
            Denial::OutsideScope,
        );
    }
}

#[test]
fn parent_exclusions_are_case_insensitive_and_rechecked_after_snapshot() {
    let mut f = Fixture::new();
    let path = f.file("Исключить/nested/file.txt");
    let snapshot = f.snapshot(&path).unwrap();
    let parent = f.temp.join("ИСКЛЮЧИТЬ").to_str().unwrap().to_string();
    f.policy.exclusions.add(&parent).unwrap();
    f.policy.exclusions.add(&parent.to_lowercase()).unwrap();
    assert_eq!(f.policy.exclusions.0.len(), 1);
    denied(
        f.policy.revalidate(snapshot.id, &snapshot),
        Denial::Excluded,
    );
    f.policy.exclusions.remove(&parent.to_lowercase()).unwrap();
    f.policy.revalidate(snapshot.id, &snapshot).unwrap();
    f.policy.exclusions.add(f.root.to_str().unwrap()).unwrap();
    denied(f.snapshot(&path), Denial::Excluded);
    denied(
        f.policy.exclusions.add(r"C:\x\..\y"),
        Denial::UnsupportedPath,
    );
}

#[test]
fn redirected_user_documents_stay_protected_even_inside_temp() {
    let mut f = Fixture::new();
    let path = f.file("redirected-downloads/document.txt");
    f.policy.protected.push(local(path.parent().unwrap()));
    denied(f.snapshot(&path), Denial::Protected);
}

#[test]
fn real_windows_metadata_identity_and_selected_id_are_revalidated() {
    let f = Fixture::new();
    let path = f.file("Юникод/данные.txt");
    let snapshot = f.snapshot(&path).unwrap();
    f.policy.revalidate(snapshot.id, &snapshot).unwrap();
    denied(
        f.policy.revalidate(CandidateId::new(), &snapshot),
        Denial::ForeignId,
    );
    fs::write(&path, b"changed artificial file").unwrap();
    denied(
        f.policy.revalidate(snapshot.id, &snapshot),
        Denial::TargetChanged,
    );
    let snapshot = f.snapshot(&path).unwrap();
    fs::rename(&path, path.with_extension("saved")).unwrap();
    fs::write(&path, b"changed artificial file").unwrap();
    denied(
        f.policy.revalidate(snapshot.id, &snapshot),
        Denial::TargetChanged,
    );
    // Every comparable metadata field participates; same-size replacement cannot pass.
    for field in 0..8 {
        let mut changed = snapshot.metadata.clone();
        match field {
            0 => changed.identity.volume ^= 1,
            1 => changed.identity.id[0] ^= 1,
            2 => changed.size += 1,
            3 => changed.created += 1,
            4 => changed.modified += 1,
            5 => changed.changed += 1,
            6 => changed.attributes ^= 1,
            _ => changed.links += 1,
        }
        denied(
            check_identity(snapshot.id, snapshot.id, &snapshot.metadata, &changed),
            Denial::TargetChanged,
        );
    }
}

#[test]
fn windows_hardlinks_locks_directories_and_missing_files_fail_closed() {
    let f = Fixture::new();
    let path = f.file("file.txt");
    let lock = fs::OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    denied(f.snapshot(&path), Denial::Locked);
    drop(lock);
    fs::hard_link(&path, f.temp.join("second-link.txt")).unwrap();
    denied(f.snapshot(&path), Denial::HardLink);
    let dir = f.temp.join("directory");
    fs::create_dir(&dir).unwrap();
    denied(f.snapshot(&dir), Denial::NotRegularFile);
    denied(f.snapshot(&f.temp.join("absent.txt")), Denial::Unavailable);
}

#[test]
fn cloud_reparse_and_unsupported_attributes_never_pass() {
    for attr in [
        FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY,
    ] {
        denied(check_attributes(attr, false), Denial::ReparsePoint);
    }
    for attr in [
        FILE_ATTRIBUTE_OFFLINE,
        FILE_ATTRIBUTE_RECALL_ON_OPEN,
        FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
    ] {
        denied(check_attributes(attr, true), Denial::CloudPlaceholder);
    }
    for attr in [
        FILE_ATTRIBUTE_SYSTEM,
        FILE_ATTRIBUTE_READONLY,
        FILE_ATTRIBUTE_ENCRYPTED,
        0x8000_0000,
    ] {
        denied(check_attributes(attr, true), Denial::Protected);
    }
    check_attributes(FILE_ATTRIBUTE_ARCHIVE, true).unwrap();
}

#[test]
fn current_user_scope_comes_from_known_folders_without_reading_user_files() {
    let policy = FilePolicy::for_current_user().unwrap();
    assert!(policy.temp.within(&policy.profile));
    denied(
        policy.check_path(
            FileSource::UserTemp,
            &policy.profile.child("Downloads/fixture.txt").unwrap(),
        ),
        Denial::Protected,
    );
    policy
        .check_path(
            FileSource::UserTemp,
            &policy.temp.child("fixture.txt").unwrap(),
        )
        .unwrap();
}

#[test]
fn windows_acl_denial_is_visible_and_never_elevated() {
    use windows_sys::Win32::Security::{Authorization::*, *};
    let f = Fixture::new();
    let path = f.file("acl-denied/acl-fixture.txt");
    let snapshot = f.snapshot(&path).unwrap();
    fn deny_acl(path: &Path, rights: u32) -> impl Drop {
        let mut wide = local(path).wide();
        let mut original_sd = ptr::null_mut();
        let mut original_dacl = ptr::null_mut();
        assert_eq!(
            unsafe {
                GetNamedSecurityInfoW(
                    wide.as_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    &mut original_dacl,
                    ptr::null_mut(),
                    &mut original_sd,
                )
            },
            ERROR_SUCCESS
        );
        struct Restore {
            path: Vec<u16>,
            sd: PSECURITY_DESCRIPTOR,
            dacl: *mut ACL,
        }
        impl Drop for Restore {
            fn drop(&mut self) {
                let result = unsafe {
                    SetNamedSecurityInfoW(
                        self.path.as_ptr(),
                        SE_FILE_OBJECT,
                        DACL_SECURITY_INFORMATION | UNPROTECTED_DACL_SECURITY_INFORMATION,
                        ptr::null_mut(),
                        ptr::null_mut(),
                        self.dacl,
                        ptr::null(),
                    )
                };
                unsafe { LocalFree(self.sd) };
                assert_eq!(result, ERROR_SUCCESS, "restore artificial fixture ACL");
            }
        }
        let restore = Restore {
            path: wide.clone(),
            sd: original_sd,
            dacl: original_dacl,
        };
        let sddl: Vec<u16> = format!("D:(D;;0x{rights:x};;;WD)(A;;FA;;;WD)")
            .encode_utf16()
            .chain([0])
            .collect();
        let mut denied_sd = ptr::null_mut();
        assert_ne!(
            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    1,
                    &mut denied_sd,
                    ptr::null_mut(),
                )
            },
            0
        );
        let mut present = 0;
        let mut defaulted = 0;
        let mut denied_dacl = ptr::null_mut();
        assert_ne!(
            unsafe {
                GetSecurityDescriptorDacl(denied_sd, &mut present, &mut denied_dacl, &mut defaulted)
            },
            0
        );
        let result = unsafe {
            SetNamedSecurityInfoW(
                wide.as_mut_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                denied_dacl,
                ptr::null(),
            )
        };
        unsafe { LocalFree(denied_sd) };
        assert_eq!(result, ERROR_SUCCESS);
        restore
    }
    let parent_restore = deny_acl(path.parent().unwrap(), FILE_DELETE_CHILD);
    let file_restore = deny_acl(&path, DELETE);
    denied(
        f.policy.revalidate(snapshot.id, &snapshot),
        Denial::AccessDenied,
    );
    drop(file_restore);
    drop(parent_restore);
    // ACL restoration changes ChangeTime, so a fresh snapshot is required.
    f.snapshot(&path).unwrap();
}

#[test]
fn windows_long_paths_remain_local_and_revalidate() {
    let f = Fixture::new();
    let path = f.file(&format!(
        "{}/{}/{}/данные.txt",
        "a".repeat(100),
        "b".repeat(100),
        "c".repeat(100)
    ));
    assert!(path.to_str().unwrap().len() > 260);
    let snapshot = f.snapshot(&path).unwrap();
    f.policy.revalidate(snapshot.id, &snapshot).unwrap();
}

// Junctions need no Developer Mode/admin. Only a newly created fixture directory is modified.
pub(crate) fn junction(link: &Path, target: &Path) {
    use windows_sys::Win32::System::{IO::DeviceIoControl, Ioctl::FSCTL_SET_REPARSE_POINT};
    assert!(
        link.starts_with(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../target/policy-fixtures")
                .canonicalize()
                .unwrap()
        )
    );
    fs::create_dir(link).unwrap();
    let target = local(target).as_str().to_string();
    let substitute: Vec<u16> = format!(r"\??\{target}").encode_utf16().collect();
    let print: Vec<u16> = target.encode_utf16().collect();
    let mut data = Vec::new();
    let path_bytes = (substitute.len() + print.len() + 2) * 2;
    data.extend(0xA0000003u32.to_le_bytes()); // IO_REPARSE_TAG_MOUNT_POINT
    data.extend(((8 + path_bytes) as u16).to_le_bytes());
    data.extend(0u16.to_le_bytes());
    for value in [
        0,
        substitute.len() * 2,
        (substitute.len() + 1) * 2,
        print.len() * 2,
    ] {
        data.extend((value as u16).to_le_bytes());
    }
    for unit in substitute.into_iter().chain([0]).chain(print).chain([0]) {
        data.extend(unit.to_le_bytes());
    }
    let wide = local(link).wide();
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_WRITE,
            0,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            ptr::null_mut(),
        )
    };
    assert_ne!(raw, INVALID_HANDLE_VALUE);
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    let mut returned = 0;
    assert_ne!(
        unsafe {
            DeviceIoControl(
                handle.as_raw_handle(),
                FSCTL_SET_REPARSE_POINT,
                data.as_ptr().cast(),
                data.len() as u32,
                ptr::null_mut(),
                0,
                &mut returned,
                ptr::null_mut(),
            )
        },
        0,
        "junction fixture: {:?}",
        std::io::Error::last_os_error()
    );
}

#[test]
fn actual_junctions_and_loops_are_rejected_before_following_children() {
    let f = Fixture::new();
    let external = f.root.join("outside");
    fs::create_dir(&external).unwrap();
    fs::write(external.join("file.txt"), b"outside fixture").unwrap();
    let link = f.temp.join("junction");
    junction(&link, &external);
    denied(f.snapshot(&link.join("file.txt")), Denial::ReparsePoint);
    let loop_path = f.temp.join("loop");
    junction(&loop_path, &f.temp);
    denied(
        f.snapshot(&loop_path.join("loop/loop/file.txt")),
        Denial::ReparsePoint,
    );
    // Keep all synthetic fixtures as test artifacts; no recursive deletion across junctions.
}
