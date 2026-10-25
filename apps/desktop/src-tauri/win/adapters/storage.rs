//! Read-only handle metadata and bounded depth-first enumeration. Never opens file contents.
use super::{
    files::{self, Exclusions},
    path::LocalPath,
};
use crate::domain::{
    contracts::*,
    engine::{ScanEvent, ScanSink},
    policy::Denial,
};
use std::{
    collections::HashSet,
    fs::{self, ReadDir},
    os::windows::io::{FromRawHandle, OwnedHandle},
    ptr,
};
use windows_sys::Win32::{Foundation::*, Storage::FileSystem::*};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn volume(path: String) -> VolumeInfo {
    let mut result = VolumeInfo {
        path,
        filesystem: None,
        total_bytes: None,
        free_bytes: None,
        available_bytes: None,
        error: Some(ApiError::Unsupported),
        last_scan_path: None,
        last_scan_ms: None,
    };
    let root = wide(&result.path);
    if unsafe { GetDriveTypeW(root.as_ptr()) } != 3 {
        return result;
    }
    let mut filesystem = [0u16; 64];
    if unsafe {
        GetVolumeInformationW(
            root.as_ptr(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    } == 0
    {
        result.error = Some(ApiError::AdapterError);
        return result;
    }
    let name = String::from_utf16_lossy(
        &filesystem[..filesystem
            .iter()
            .position(|v| *v == 0)
            .unwrap_or(filesystem.len())],
    );
    result.filesystem = Some(name.clone());
    let (mut available, mut total, mut free) = (0, 0, 0);
    if unsafe { GetDiskFreeSpaceExW(root.as_ptr(), &mut available, &mut total, &mut free) } != 0 {
        result.total_bytes = Some(ByteCount(total));
        result.available_bytes = Some(ByteCount(available));
        result.free_bytes = Some(ByteCount(free));
        result.error = if name == "NTFS" {
            None
        } else {
            Some(ApiError::Unsupported)
        };
    } else {
        result.error = Some(ApiError::AdapterError);
    }
    result
}

pub fn volumes() -> Result<Vec<VolumeInfo>, ApiError> {
    let bits = unsafe { GetLogicalDrives() };
    if bits == 0 {
        return Err(ApiError::AdapterError);
    }
    Ok((0..26)
        .filter(|index| bits & (1 << index) != 0)
        .map(|index| volume(format!("{}:\\", (b'A' + index) as char)))
        .collect())
}

pub struct StorageScanner {
    root: LocalPath,
    exclusions: Exclusions,
}
impl StorageScanner {
    pub fn current(path: &str, exclusions: &[String]) -> Result<Self, ApiError> {
        let root = LocalPath::parse(path).map_err(|_| ApiError::InvalidRequest)?;
        #[cfg(all(feature = "qa-fixtures", not(test)))]
        {
            let allowed = super::preferences::qa_directory()
                .and_then(|p| p.parent().map(|p| p.join("Profile")))
                .ok_or(ApiError::SettingsUnavailable)?;
            let allowed = LocalPath::parse(allowed.to_str().ok_or(ApiError::Protected)?)
                .map_err(|_| ApiError::Protected)?;
            if !root.within(&allowed) {
                return Err(ApiError::Protected);
            }
        }
        let v = volume(format!("{}\\", &root.as_str()[..2]));
        if let Some(error) = v.error {
            return Err(error);
        }
        let mut exclusions_set = Exclusions::default();
        for path in exclusions {
            exclusions_set
                .add(path)
                .map_err(|_| ApiError::SettingsInvalid)?;
        }
        Ok(Self {
            root,
            exclusions: exclusions_set,
        })
    }

    pub fn scan(&self, sink: &mut ScanSink<'_>) -> Result<(), ApiError> {
        sink(ScanEvent::Checkpoint)?;
        if self.exclusions.contains(&self.root) {
            return Err(ApiError::Protected);
        }
        let mut root_guards = Vec::new();
        for prefix in self.root.prefixes() {
            if self.exclusions.contains(&prefix) {
                return Err(ApiError::Protected);
            }
            root_guards.push(probe(&prefix, true).map_err(|_| ApiError::Protected)?);
        }
        let root = root_guards.last().ok_or(ApiError::Internal)?;
        let volume_id = root.identity.0;
        let mut identities = HashSet::from([root.identity]);
        let mut next_id = 1;
        let mut root_node = node(0, None, &self.root, true);
        root_node.modified_ms = root.modified_ms;
        root_node.identity = Some(root.identity);
        sink(ScanEvent::StorageNode(root_node))?;
        let entries = fs::read_dir(self.root.wide_path()).map_err(|_| ApiError::AdapterError)?;
        let mut stack = vec![Frame {
            id: 0,
            path: self.root.clone(),
            entries,
            _guard: None,
        }];
        while !stack.is_empty() {
            sink(ScanEvent::Checkpoint)?;
            let frame = stack.last_mut().ok_or(ApiError::Internal)?;
            let entry = match frame.entries.next() {
                Some(entry) => entry,
                None => {
                    stack.pop();
                    continue;
                }
            };
            sink(ScanEvent::Visited)?;
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    issue(sink, files::denial_from_io(e))?;
                    continue;
                }
            };
            let path = match entry
                .file_name()
                .to_str()
                .and_then(|name| frame.path.child(name).ok())
            {
                Some(path) => path,
                None => {
                    issue(sink, Denial::UnsupportedPath)?;
                    continue;
                }
            };
            let directory = match entry.file_type() {
                Ok(kind) => kind.is_dir(),
                Err(e) => {
                    issue(sink, files::denial_from_io(e))?;
                    continue;
                }
            };
            let mut item = node(next_id, Some(frame.id), &path, directory);
            next_id += 1;
            let result = if self.exclusions.contains(&path) {
                Err(Denial::Excluded)
            } else {
                probe(&path, directory)
            };
            match result {
                Ok(guard) if guard.identity.0 != volume_id => {
                    item.skipped = Some(Denial::OutsideScope);
                }
                Ok(guard) if !identities.insert(guard.identity) => {
                    item.skipped = Some(Denial::HardLink);
                }
                Ok(guard) => {
                    item.identity = Some(guard.identity);
                    item.modified_ms = guard.modified_ms;
                    item.bytes = ByteCount(guard.allocated);
                    item.logical_bytes = ByteCount(guard.logical);
                    if directory {
                        match fs::read_dir(path.wide_path()) {
                            Ok(entries) => {
                                let id = item.id;
                                sink(ScanEvent::StorageNode(item))?;
                                stack.push(Frame {
                                    id,
                                    path,
                                    entries,
                                    _guard: Some(guard),
                                });
                                continue;
                            }
                            Err(e) => {
                                item.skipped = Some(files::denial_from_io(e));
                            }
                        }
                    }
                }
                Err(reason) => {
                    item.skipped = Some(reason);
                }
            }
            if let Some(reason) = item.skipped {
                issue(sink, reason)?;
            }
            sink(ScanEvent::StorageNode(item))?;
        }
        Ok(())
    }
}

fn issue(sink: &mut ScanSink<'_>, reason: Denial) -> Result<(), ApiError> {
    sink(ScanEvent::Issue(ScanIssue {
        rule_id: "storage".into(),
        reason,
        count: 1,
        sample_path: None,
    }))
}
fn node(id: u32, parent_id: Option<u32>, path: &LocalPath, directory: bool) -> StorageNode {
    StorageNode {
        id,
        parent_id,
        name: path
            .as_str()
            .rsplit('\\')
            .find(|part| !part.is_empty())
            .unwrap_or(path.as_str())
            .into(),
        path: path.as_str().into(),
        directory,
        bytes: ByteCount(0),
        logical_bytes: ByteCount(0),
        modified_ms: None,
        skipped: None,
        identity: None,
    }
}
struct Frame {
    id: u32,
    path: LocalPath,
    entries: ReadDir,
    _guard: Option<Probe>,
}
struct Probe {
    _handle: OwnedHandle,
    identity: (u64, [u8; 16]),
    allocated: u64,
    logical: u64,
    modified_ms: Option<u64>,
}
fn probe(path: &LocalPath, directory: bool) -> Result<Probe, Denial> {
    if unsafe { GetDriveTypeW(wide(&format!("{}\\", &path.as_str()[..2])).as_ptr()) } != 3 {
        return Err(Denial::UnsupportedPath);
    }
    let raw = unsafe {
        CreateFileW(
            path.wide().as_ptr(),
            FILE_READ_ATTRIBUTES | if directory { FILE_LIST_DIRECTORY } else { 0 },
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            ptr::null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(files::denial_from_io(std::io::Error::last_os_error()));
    }
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    let basic: FILE_BASIC_INFO = files::info(&handle, FileBasicInfo)?;
    files::check_attributes(basic.FileAttributes, false)?;
    let standard: FILE_STANDARD_INFO = files::info(&handle, FileStandardInfo)?;
    if standard.Directory != directory
        || standard.DeletePending
        || standard.EndOfFile < 0
        || standard.AllocationSize < 0
    {
        return Err(Denial::TargetChanged);
    }
    if !directory && standard.NumberOfLinks != 1 {
        return Err(Denial::HardLink);
    }
    if files::final_path(&handle)? != *path {
        return Err(Denial::TargetChanged);
    }
    if directory {
        let sensitive: FILE_CASE_SENSITIVE_INFO = files::info(&handle, FileCaseSensitiveInfo)?;
        if sensitive.Flags != 0 {
            return Err(Denial::UnsupportedPath);
        }
    }
    let identity: FILE_ID_INFO = files::info(&handle, FileIdInfo)?;
    let allocated = if directory {
        0
    } else if basic.FileAttributes & (FILE_ATTRIBUTE_COMPRESSED | FILE_ATTRIBUTE_SPARSE_FILE) != 0 {
        let compression: FILE_COMPRESSION_INFO = files::info(&handle, FileCompressionInfo)?;
        u64::try_from(compression.CompressedFileSize).map_err(|_| Denial::Unavailable)?
    } else {
        standard.AllocationSize as u64
    };
    Ok(Probe {
        _handle: handle,
        identity: (identity.VolumeSerialNumber, identity.FileId.Identifier),
        allocated,
        logical: if directory {
            0
        } else {
            standard.EndOfFile as u64
        },
        modified_ms: u64::try_from(basic.LastWriteTime)
            .ok()
            .and_then(|time| time.checked_sub(116_444_736_000_000_000))
            .map(|time| time / 10_000),
    })
}

fn location_guards(node: &StorageNode, exclusions: &[String]) -> Result<Vec<Probe>, ApiError> {
    if node.skipped.is_some() || node.identity.is_none() {
        return Err(ApiError::Protected);
    }
    let path = LocalPath::parse(&node.path).map_err(|_| ApiError::Protected)?;
    let mut excluded = Exclusions::default();
    for text in exclusions {
        excluded.add(text).map_err(|_| ApiError::SettingsInvalid)?;
    }
    if excluded.contains(&path) {
        return Err(ApiError::Protected);
    }
    let prefixes = path.prefixes();
    let mut guards = Vec::new();
    for (index, prefix) in prefixes.iter().enumerate() {
        guards.push(
            probe(prefix, index + 1 != prefixes.len() || node.directory)
                .map_err(|_| ApiError::TargetChanged)?,
        );
    }
    let current = guards.last().ok_or(ApiError::TargetChanged)?;
    if Some(current.identity) != node.identity
        || current.modified_ms != node.modified_ms
        || (!node.directory && current.logical != node.logical_bytes.0)
    {
        return Err(ApiError::TargetChanged);
    }
    Ok(guards)
}

pub fn show_location(node: &StorageNode, exclusions: &[String]) -> Result<(), ApiError> {
    use windows_sys::Win32::{System::Com::*, UI::Shell::*};
    let _guards = location_guards(node, exclusions)?;
    // A PIDL selects a backend-owned filesystem object; no shell command or file execution.
    let initialized = unsafe { CoInitializeEx(ptr::null(), COINIT_APARTMENTTHREADED as u32) };
    if initialized < 0 {
        return Err(ApiError::AdapterError);
    }
    let pidl = unsafe { ILCreateFromPathW(wide(&node.path).as_ptr()) };
    let result = if pidl.is_null() {
        Err(ApiError::Unsupported)
    } else {
        let hr = unsafe { SHOpenFolderAndSelectItems(pidl, 0, ptr::null(), 0) };
        unsafe { ILFree(pidl) };
        if hr < 0 {
            Err(ApiError::AdapterError)
        } else {
            Ok(())
        }
    };
    unsafe { CoUninitialize() };
    result
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
