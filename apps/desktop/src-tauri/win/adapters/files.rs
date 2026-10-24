//! Backend-only read probes; a successful probe is not a deletion capability.
use super::path::LocalPath;
use crate::domain::{
    contracts::CandidateId,
    policy::{Denial, FileIdentity, FileMetadata, check_identity},
};
use std::{
    ffi::c_void,
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr,
};
use windows_sys::{
    Win32::{Foundation::*, Storage::FileSystem::*, System::Com::CoTaskMemFree, UI::Shell::*},
    core::GUID,
};

pub use crate::domain::cleanup::RuleId as FileSource;

pub struct FileSnapshot {
    id: CandidateId,
    source: FileSource,
    path: LocalPath,
    metadata: FileMetadata,
}
impl FileSnapshot {
    pub fn metadata(&self) -> &FileMetadata {
        &self.metadata
    }
    pub fn path(&self) -> &str {
        self.path.as_str()
    }
}

/// Exclusions only restrict the backend scope; they never grant access.
#[derive(Default)]
pub struct Exclusions(Vec<LocalPath>);
impl Exclusions {
    pub fn paths(&self) -> Vec<String> {
        self.0.iter().map(|path| path.as_str().to_owned()).collect()
    }
    pub fn add(&mut self, path: &str) -> Result<(), Denial> {
        let path = LocalPath::parse(path)?;
        if !self.0.contains(&path) {
            self.0.push(path);
        }
        Ok(())
    }
    pub fn remove(&mut self, path: &str) -> Result<(), Denial> {
        let path = LocalPath::parse(path)?;
        self.0.retain(|p| p != &path);
        Ok(())
    }
    fn contains(&self, path: &LocalPath) -> bool {
        self.0.iter().any(|p| path.within(p))
    }
}

pub struct FilePolicy {
    profile: LocalPath,
    temp: LocalPath,
    local_app_data: LocalPath,
    protected: Vec<LocalPath>,
    pub exclusions: Exclusions,
}
impl FilePolicy {
    #[cfg(any(test, feature = "qa-fixtures"))]
    pub(crate) fn for_fixture(profile: &std::path::Path) -> Result<Self, Denial> {
        let profile = LocalPath::parse(profile.to_str().ok_or(Denial::UnsupportedPath)?)?;
        let local_app_data = profile.child(r"AppData\Local")?;
        Ok(Self {
            temp: local_app_data.child("Temp")?,
            local_app_data,
            protected: vec![
                profile.child("Documents")?,
                profile.child("Downloads")?,
                profile.child("Desktop")?,
            ],
            profile,
            exclusions: Exclusions::default(),
        })
    }
    pub fn for_current_user() -> Result<Self, Denial> {
        let profile = known_folder(&FOLDERID_Profile)?;
        let local_app_data = known_folder(&FOLDERID_LocalAppData)?;
        let temp = local_app_data.child("Temp")?;
        if !temp.within(&profile) || temp == profile {
            return Err(Denial::OutsideScope);
        }
        Ok(Self {
            profile,
            temp,
            local_app_data,
            protected: vec![
                known_folder(&FOLDERID_Windows)?,
                known_folder(&FOLDERID_ProgramFiles)?,
                known_folder(&FOLDERID_ProgramFilesX86)?,
                known_folder(&FOLDERID_ProgramData)?,
                known_folder(&FOLDERID_Downloads)?,
                known_folder(&FOLDERID_Documents)?,
                known_folder(&FOLDERID_Desktop)?,
            ],
            exclusions: Exclusions::default(),
        })
    }
    pub fn check_path(&self, source: FileSource, path: &LocalPath) -> Result<(), Denial> {
        self.check_scope(source, path, false)
    }
    pub fn root(&self, source: FileSource) -> Result<LocalPath, Denial> {
        match source.cache_relative() {
            Some(relative) => self.local_app_data.child(relative),
            None => Ok(self.temp.clone()),
        }
    }
    fn check_scope(
        &self,
        source: FileSource,
        path: &LocalPath,
        directory: bool,
    ) -> Result<(), Denial> {
        if path.is_root() || self.protected.iter().any(|p| path.within(p)) {
            return Err(Denial::Protected);
        }
        let root = LocalPath::parse(&format!("{}\\", &path.as_str()[..2]))?;
        for name in [
            "Windows",
            "Program Files",
            "Program Files (x86)",
            "ProgramData",
            "Recovery",
            "System Volume Information",
            "$Recycle.Bin",
            "Config.Msi",
        ] {
            if path.within(&root.child(name)?) {
                return Err(Denial::Protected);
            }
        }
        if self.exclusions.contains(path) {
            return Err(Denial::Excluded);
        }
        let root = self.root(source)?;
        if path.within(&self.profile) && path.within(&root) && (directory || path != &root) {
            if path
                .as_str()
                .split('\\')
                .skip(root.as_str().split('\\').count())
                .any(sensitive_component)
            {
                return Err(Denial::SensitiveData);
            }
            Ok(())
        } else {
            Err(Denial::OutsideScope)
        }
    }
    pub(crate) fn directory(
        &self,
        source: FileSource,
        path: &LocalPath,
    ) -> Result<DirectoryProbe, Denial> {
        self.check_scope(source, path, true)?;
        let handles = open_path(path, true)?;
        deny_browser_profile_ancestors(path, &self.root(source)?, true)?;
        let entries =
            std::fs::read_dir(format!(r"\\?\{}", path.as_str())).map_err(denial_from_io)?;
        Ok(DirectoryProbe {
            _handles: handles,
            entries,
        })
    }
    pub fn snapshot(
        &self,
        id: CandidateId,
        source: FileSource,
        path: &str,
    ) -> Result<FileSnapshot, Denial> {
        let path = LocalPath::parse(path)?;
        self.check_path(source, &path)?;
        let observed = observe(&path, &self.root(source)?)?;
        self.check_path(source, &observed.path)?;
        Ok(FileSnapshot {
            id,
            source,
            path: observed.path,
            metadata: observed.metadata,
        })
    }
    pub fn revalidate(&self, selected: CandidateId, snapshot: &FileSnapshot) -> Result<(), Denial> {
        if selected != snapshot.id {
            return Err(Denial::ForeignId);
        }
        self.check_path(snapshot.source, &snapshot.path)?;
        let current = observe(&snapshot.path, &self.root(snapshot.source)?)?;
        self.check_path(snapshot.source, &current.path)?;
        if snapshot.path != current.path {
            return Err(Denial::TargetChanged);
        }
        check_identity(selected, snapshot.id, &snapshot.metadata, &current.metadata)
        // shortcut: dry-run only; the SPE-63 mutator must close the final check/use race.
    }
}

fn known_folder(id: &GUID) -> Result<LocalPath, Denial> {
    let mut raw = ptr::null_mut();
    // Shell allocates a NUL-terminated string; always release it with the matching allocator.
    let hr =
        unsafe { SHGetKnownFolderPath(id, KF_FLAG_DONT_VERIFY as u32, ptr::null_mut(), &mut raw) };
    if hr < 0 {
        unsafe { CoTaskMemFree(raw.cast()) };
        return Err(Denial::Unavailable);
    }
    if raw.is_null() {
        return Err(Denial::Unavailable);
    }
    let result = (|| {
        let mut len = 0;
        while len <= 32_000 && unsafe { *raw.add(len) } != 0 {
            len += 1;
        }
        if len > 32_000 {
            return Err(Denial::UnsupportedPath);
        }
        let text = String::from_utf16(unsafe { std::slice::from_raw_parts(raw, len) })
            .map_err(|_| Denial::UnsupportedPath)?;
        LocalPath::parse(&text)
    })();
    unsafe { CoTaskMemFree(raw.cast()) };
    result
}

struct Observed {
    path: LocalPath,
    metadata: FileMetadata,
}

pub(crate) struct DirectoryProbe {
    _handles: Vec<OwnedHandle>,
    entries: std::fs::ReadDir,
}
impl Iterator for DirectoryProbe {
    type Item = Result<std::path::PathBuf, Denial>;
    fn next(&mut self) -> Option<Self::Item> {
        self.entries
            .next()
            .map(|entry| entry.map(|entry| entry.path()).map_err(denial_from_io))
    }
}

fn observe(path: &LocalPath, root: &LocalPath) -> Result<Observed, Denial> {
    let handles = open_path(path, false)?;
    deny_browser_profile_ancestors(path, root, false)?;
    let handle = handles.last().ok_or(Denial::Unavailable)?;
    let basic: FILE_BASIC_INFO = info(handle, FileBasicInfo)?;
    let standard: FILE_STANDARD_INFO = info(handle, FileStandardInfo)?;
    let identity: FILE_ID_INFO = info(handle, FileIdInfo)?;
    if standard.Directory
        || standard.DeletePending
        || standard.EndOfFile < 0
        || standard.AllocationSize < 0
    {
        return Err(Denial::NotRegularFile);
    }
    if standard.NumberOfLinks != 1 {
        return Err(Denial::HardLink);
    }
    let allocated_size = if basic.FileAttributes & FILE_ATTRIBUTE_COMPRESSED != 0 {
        // Ancestors and leaf stay pinned without write/delete sharing for this path-based query.
        let mut high = 0;
        unsafe { SetLastError(ERROR_SUCCESS) };
        let low = unsafe { GetCompressedFileSizeW(path.wide().as_ptr(), &mut high) };
        if low == u32::MAX && unsafe { GetLastError() } != ERROR_SUCCESS {
            return Err(last_error());
        }
        (u64::from(high) << 32) | u64::from(low)
    } else {
        standard.AllocationSize as u64
    };
    Ok(Observed {
        path: final_path(handle)?,
        metadata: FileMetadata {
            identity: FileIdentity {
                volume: identity.VolumeSerialNumber,
                id: identity.FileId.Identifier,
            },
            size: standard.EndOfFile as u64,
            allocated_size,
            created: basic.CreationTime,
            modified: basic.LastWriteTime,
            changed: basic.ChangeTime,
            attributes: basic.FileAttributes,
            links: standard.NumberOfLinks,
        },
    })
}

fn sensitive_component(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let mut base = lower.as_str();
    while let Some(shorter) = [
        "-wal", "-shm", "-journal", ".bak", ".old", ".backup", ".tmp", ".temp",
    ]
    .into_iter()
    .find_map(|suffix| base.strip_suffix(suffix))
    {
        base = shorter;
    }
    matches!(
        base,
        "cookies"
            | "login data"
            | "web data"
            | "network"
            | "sessions"
            | "session storage"
            | "local storage"
            | "indexeddb"
            | "service worker"
            | "webstorage"
            | "storage"
            | "cookies.sqlite"
            | "key3.db"
            | "key4.db"
            | "cert8.db"
            | "cert9.db"
            | "logins.json"
            | "sessionstore.jsonlz4"
            | "sessionstore.js"
            | "sessionstore-backups"
            | "places.sqlite"
            | "formhistory.sqlite"
            | "webappsstore.sqlite"
            | "chromeappsstore.sqlite"
            | "permissions.sqlite"
            | "content-prefs.sqlite"
            | "favicons.sqlite"
            | "bookmarkbackups"
            | "pkcs11.txt"
            | "handlers.json"
            | "xulstore.json"
            | "containers.json"
            | "persdict.dat"
            | "search.json.mozlz4"
            | "user.js"
            | "extensions"
            | "prefs.js"
            | "local state"
            | "preferences"
            | "secure preferences"
            | "bookmarks"
            | "current session"
            | "current tabs"
            | "last session"
            | "last tabs"
    )
}

fn deny_browser_profile_ancestors(
    path: &LocalPath,
    root: &LocalPath,
    directory: bool,
) -> Result<(), Denial> {
    // This only denies scope. Marker metadata is queried while the caller pins all ancestors.
    let prefixes = path.prefixes();
    let count = prefixes.len() - usize::from(!directory);
    for parent in prefixes
        .into_iter()
        .take(count)
        .filter(|parent| parent.within(root))
    {
        for marker in [
            "Local State",
            "Preferences",
            "Secure Preferences",
            "Login Data",
            "prefs.js",
            "cookies.sqlite",
        ] {
            let target = parent.child(marker)?;
            match std::fs::symlink_metadata(format!(r"\\?\{}", target.as_str())) {
                Ok(_) => return Err(Denial::SensitiveData),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(denial_from_io(error)),
            }
        }
    }
    Ok(())
}

fn open_path(path: &LocalPath, directory: bool) -> Result<Vec<OwnedHandle>, Denial> {
    let prefixes = path.prefixes();
    let root: Vec<u16> = prefixes[0].as_str().encode_utf16().chain([0]).collect();
    if unsafe { GetDriveTypeW(root.as_ptr()) } != 3 {
        return Err(Denial::UnsupportedPath);
    } // DRIVE_FIXED
    let mut handles = Vec::new();
    for (index, prefix) in prefixes.iter().enumerate() {
        let leaf = index + 1 == prefixes.len();
        let wide = prefix.wide();
        // OPEN_EXISTING never creates/truncates. DELETE requests an ACL check, not a mutation.
        let raw = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_READ_ATTRIBUTES
                    | if leaf && !directory {
                        DELETE
                    } else if leaf {
                        FILE_LIST_DIRECTORY
                    } else {
                        0
                    },
                FILE_SHARE_READ,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
                ptr::null_mut(),
            )
        };
        if raw == INVALID_HANDLE_VALUE {
            return Err(last_error());
        }
        let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
        let basic: FILE_BASIC_INFO = info(&handle, FileBasicInfo)?;
        check_attributes(basic.FileAttributes, leaf && !directory)?;
        if leaf
            && directory
            && basic.FileAttributes
                & (FILE_ATTRIBUTE_HIDDEN
                    | FILE_ATTRIBUTE_SYSTEM
                    | FILE_ATTRIBUTE_READONLY
                    | FILE_ATTRIBUTE_ENCRYPTED)
                != 0
        {
            return Err(Denial::Protected);
        }
        let canonical = final_path(&handle)?;
        // Reject short-name aliases, SUBST and unexpected resolution, even inside an allowed scope.
        if canonical != *prefix {
            return Err(Denial::TargetChanged);
        }
        if !leaf || directory {
            if basic.FileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0 {
                return Err(Denial::NotRegularFile);
            }
            let sensitive: FILE_CASE_SENSITIVE_INFO = info(&handle, FileCaseSensitiveInfo)?;
            if sensitive.Flags != 0 {
                return Err(Denial::UnsupportedPath);
            }
        }
        handles.push(handle);
    }
    Ok(handles)
}

fn check_attributes(attributes: u32, leaf: bool) -> Result<(), Denial> {
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(Denial::ReparsePoint);
    }
    if attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
    {
        return Err(Denial::CloudPlaceholder);
    }
    if leaf {
        if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            return Err(Denial::NotRegularFile);
        }
        let allowed = FILE_ATTRIBUTE_NORMAL
            | FILE_ATTRIBUTE_ARCHIVE
            | FILE_ATTRIBUTE_TEMPORARY
            | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED
            | FILE_ATTRIBUTE_COMPRESSED;
        if attributes & !allowed != 0 {
            return Err(Denial::Protected);
        }
    }
    Ok(())
}

fn info<T: Default>(handle: &OwnedHandle, class: FILE_INFO_BY_HANDLE_CLASS) -> Result<T, Denial> {
    let mut value = T::default();
    // T and its information class are paired at the private call sites above.
    if unsafe {
        GetFileInformationByHandleEx(
            handle.as_raw_handle(),
            class,
            (&mut value as *mut T).cast::<c_void>(),
            size_of::<T>() as u32,
        )
    } == 0
    {
        return Err(last_error());
    }
    Ok(value)
}
fn final_path(handle: &OwnedHandle) -> Result<LocalPath, Denial> {
    let mut buffer = vec![0u16; 32_768];
    let len = unsafe {
        GetFinalPathNameByHandleW(
            handle.as_raw_handle(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            0,
        )
    } as usize;
    if len == 0 {
        return Err(last_error());
    }
    if len >= buffer.len() {
        return Err(Denial::UnsupportedPath);
    }
    LocalPath::parse(&String::from_utf16(&buffer[..len]).map_err(|_| Denial::UnsupportedPath)?)
}
fn last_error() -> Denial {
    error_code(unsafe { GetLastError() })
}
pub(crate) fn denial_from_io(error: std::io::Error) -> Denial {
    error_code(error.raw_os_error().unwrap_or(0) as u32)
}
fn error_code(code: u32) -> Denial {
    match code {
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Denial::NotPresent,
        ERROR_ACCESS_DENIED | ERROR_PRIVILEGE_NOT_HELD => Denial::AccessDenied,
        ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION => Denial::Locked,
        _ => Denial::Unavailable,
    }
}

#[cfg(test)]
#[path = "files_tests.rs"]
pub(crate) mod tests;
