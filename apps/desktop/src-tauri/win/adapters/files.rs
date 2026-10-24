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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSource {
    UserTemp,
}

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
    protected: Vec<LocalPath>,
    pub exclusions: Exclusions,
}
impl FilePolicy {
    pub fn for_current_user() -> Result<Self, Denial> {
        let profile = known_folder(&FOLDERID_Profile)?;
        let temp = known_folder(&FOLDERID_LocalAppData)?.child("Temp")?;
        if !temp.within(&profile) || temp == profile {
            return Err(Denial::OutsideScope);
        }
        Ok(Self {
            profile,
            temp,
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
        // No broad profile/AppData permission: browser state, documents and other users stay denied.
        match source {
            FileSource::UserTemp
                if path.within(&self.profile) && path.within(&self.temp) && path != &self.temp =>
            {
                Ok(())
            }
            _ => Err(Denial::OutsideScope),
        }
    }
    pub fn snapshot(
        &self,
        id: CandidateId,
        source: FileSource,
        path: &str,
    ) -> Result<FileSnapshot, Denial> {
        let path = LocalPath::parse(path)?;
        self.check_path(source, &path)?;
        let observed = observe(&path)?;
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
        let current = observe(&snapshot.path)?;
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

fn observe(path: &LocalPath) -> Result<Observed, Denial> {
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
                FILE_READ_ATTRIBUTES | if leaf { DELETE } else { 0 },
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
        check_attributes(basic.FileAttributes, leaf)?;
        let canonical = final_path(&handle)?;
        // Reject short-name aliases, SUBST and unexpected resolution, even inside an allowed scope.
        if canonical != *prefix {
            return Err(Denial::TargetChanged);
        }
        if !leaf {
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
    let handle = handles.last().ok_or(Denial::Unavailable)?;
    let basic: FILE_BASIC_INFO = info(handle, FileBasicInfo)?;
    let standard: FILE_STANDARD_INFO = info(handle, FileStandardInfo)?;
    let identity: FILE_ID_INFO = info(handle, FileIdInfo)?;
    if standard.Directory || standard.DeletePending || standard.EndOfFile < 0 {
        return Err(Denial::NotRegularFile);
    }
    if standard.NumberOfLinks != 1 {
        return Err(Denial::HardLink);
    }
    Ok(Observed {
        path: final_path(handle)?,
        metadata: FileMetadata {
            identity: FileIdentity {
                volume: identity.VolumeSerialNumber,
                id: identity.FileId.Identifier,
            },
            size: standard.EndOfFile as u64,
            created: basic.CreationTime,
            modified: basic.LastWriteTime,
            changed: basic.ChangeTime,
            attributes: basic.FileAttributes,
            links: standard.NumberOfLinks,
        },
    })
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
    match unsafe { GetLastError() } {
        ERROR_ACCESS_DENIED | ERROR_PRIVILEGE_NOT_HELD => Denial::AccessDenied,
        ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION => Denial::Locked,
        _ => Denial::Unavailable,
    }
}

#[cfg(test)]
#[path = "files_tests.rs"]
pub(crate) mod tests;
