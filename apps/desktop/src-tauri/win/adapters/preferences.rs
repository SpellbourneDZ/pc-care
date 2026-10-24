//! Fixed app-owned configuration, never a frontend-selected storage path.
use super::{files::Exclusions, path::LocalPath};
use crate::domain::contracts::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Read, Write},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::PathBuf,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
use windows_sys::Win32::Storage::FileSystem::*;

const MAX_BYTES: u64 = 256 * 1024;
const MAX_EXCLUSIONS: usize = 128;

#[derive(Clone)]
pub struct SettingsStore {
    root: Option<PathBuf>,
    gate: Arc<Mutex<()>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    schema_version: u32,
    revision: SettingsRevision,
    preferences: Preferences,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Legacy {
    schema_version: u32,
    theme: Theme,
    #[serde(default)]
    exclusions: Vec<String>,
}

fn defaults() -> SettingsSnapshot {
    SettingsSnapshot {
        schema_version: 1,
        revision: None,
        preferences: Preferences::default(),
    }
}

fn validate(mut preferences: Preferences) -> Result<Preferences, ApiError> {
    if preferences.exclusions.len() > MAX_EXCLUSIONS {
        return Err(ApiError::InvalidRequest);
    }
    let mut exclusions = Exclusions::default();
    for path in &preferences.exclusions {
        exclusions.add(path).map_err(|_| ApiError::InvalidRequest)?;
    }
    preferences.exclusions = exclusions.paths();
    Ok(preferences)
}

impl SettingsStore {
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            root,
            gate: Arc::new(Mutex::new(())),
        }
    }
    pub fn load(&self) -> Result<SettingsSnapshot, ApiError> {
        let _gate = self.gate.lock().map_err(|_| ApiError::Internal)?;
        let root = self.root.as_ref().ok_or(ApiError::SettingsUnavailable)?;
        let Some(_directories) = directory_guard(root, false)? else {
            return Ok(defaults());
        };
        read_settings(root)
    }
    pub fn save(
        &self,
        expected: Option<SettingsRevision>,
        preferences: Preferences,
    ) -> Result<SettingsSnapshot, ApiError> {
        let preferences = validate(preferences)?;
        let _gate = self.gate.lock().map_err(|_| ApiError::Internal)?;
        let root = self.root.as_ref().ok_or(ApiError::SettingsUnavailable)?;
        let _directories = directory_guard(root, true)?.ok_or(ApiError::SettingsUnavailable)?;
        let _lock = lock_file(root)?;
        let current = read_settings(root)?;
        if current.revision != expected {
            return Err(ApiError::SettingsConflict);
        }
        let revision = SettingsRevision::new();
        let data = serde_json::to_vec_pretty(&Stored {
            schema_version: 1,
            revision,
            preferences: preferences.clone(),
        })
        .map_err(|_| ApiError::Internal)?;
        if data.len() as u64 > MAX_BYTES {
            return Err(ApiError::InvalidRequest);
        }
        let temporary = root.join(format!("settings.{}.tmp", Uuid::new_v4()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&temporary)
            .map_err(|_| ApiError::SettingsUnavailable)?;
        file.write_all(&data)
            .and_then(|_| file.sync_all())
            .map_err(|_| ApiError::SettingsUnavailable)?;
        drop(file);
        // Same-directory replacement keeps the old JSON intact on failed/partial writes.
        fs::rename(&temporary, root.join("settings.json"))
            .map_err(|_| ApiError::SettingsUnavailable)?;
        Ok(SettingsSnapshot {
            schema_version: 1,
            revision: Some(revision),
            preferences,
        })
    }
}

fn directory_guard(root: &std::path::Path, create: bool) -> Result<Option<Vec<File>>, ApiError> {
    let path = LocalPath::parse(root.to_str().ok_or(ApiError::SettingsUnavailable)?)
        .map_err(|_| ApiError::SettingsUnavailable)?;
    if path.is_root() {
        return Err(ApiError::SettingsUnavailable);
    }
    let mut handles = Vec::new();
    for prefix in path.prefixes() {
        let target = PathBuf::from(format!(r"\\?\{}", prefix.as_str()));
        match fs::symlink_metadata(&target) {
            Err(error) if error.kind() == ErrorKind::NotFound => {
                if !create {
                    return Ok(None);
                }
                match fs::create_dir(&target) {
                    Ok(()) => {}
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                    Err(_) => return Err(ApiError::SettingsUnavailable),
                }
            }
            Err(_) => return Err(ApiError::SettingsUnavailable),
            Ok(_) => {}
        }
        let file = OpenOptions::new()
            .read(true)
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
            .open(&target)
            .map_err(|_| ApiError::SettingsUnavailable)?;
        let metadata = file.metadata().map_err(|_| ApiError::SettingsUnavailable)?;
        if !metadata.is_dir()
            || metadata.file_attributes()
                & (FILE_ATTRIBUTE_REPARSE_POINT
                    | FILE_ATTRIBUTE_OFFLINE
                    | FILE_ATTRIBUTE_RECALL_ON_OPEN
                    | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
                != 0
        {
            return Err(ApiError::SettingsUnavailable);
        }
        handles.push(file);
    }
    Ok(Some(handles))
}

fn check_file(file: &File) -> Result<(), ApiError> {
    let metadata = file.metadata().map_err(|_| ApiError::SettingsUnavailable)?;
    if !metadata.is_file()
        || metadata.file_attributes()
            & (FILE_ATTRIBUTE_REPARSE_POINT
                | FILE_ATTRIBUTE_OFFLINE
                | FILE_ATTRIBUTE_RECALL_ON_OPEN
                | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            != 0
    {
        return Err(ApiError::SettingsUnavailable);
    }
    let mut info = FILE_STANDARD_INFO::default();
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileStandardInfo,
            (&mut info as *mut FILE_STANDARD_INFO).cast(),
            std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
        )
    } == 0
        || info.NumberOfLinks != 1
        || info.DeletePending
    {
        return Err(ApiError::SettingsUnavailable);
    }
    if metadata.len() > MAX_BYTES {
        return Err(ApiError::SettingsInvalid);
    }
    Ok(())
}
fn lock_file(root: &std::path::Path) -> Result<File, ApiError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(root.join("settings.lock"))
        .map_err(|_| ApiError::SettingsUnavailable)?;
    check_file(&file)?;
    file.try_lock().map_err(|_| ApiError::SettingsUnavailable)?;
    Ok(file)
}
fn read_settings(root: &std::path::Path) -> Result<SettingsSnapshot, ApiError> {
    let mut file = match OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(root.join("settings.json"))
    {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(defaults()),
        Err(_) => return Err(ApiError::SettingsUnavailable),
    };
    check_file(&file)?;
    let mut data = Vec::new();
    (&mut file)
        .take(MAX_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|_| ApiError::SettingsUnavailable)?;
    if data.len() as u64 > MAX_BYTES {
        return Err(ApiError::SettingsInvalid);
    }
    let value: serde_json::Value =
        serde_json::from_slice(&data).map_err(|_| ApiError::SettingsInvalid)?;
    match value.get("schema_version").and_then(|v| v.as_u64()) {
        Some(1) => {
            let stored: Stored =
                serde_json::from_value(value).map_err(|_| ApiError::SettingsInvalid)?;
            Ok(SettingsSnapshot {
                schema_version: 1,
                revision: Some(stored.revision),
                preferences: validate(stored.preferences).map_err(|_| ApiError::SettingsInvalid)?,
            })
        }
        Some(0) => {
            let old: Legacy =
                serde_json::from_value(value).map_err(|_| ApiError::SettingsInvalid)?;
            if old.schema_version != 0 {
                return Err(ApiError::SettingsInvalid);
            }
            Ok(SettingsSnapshot {
                schema_version: 1,
                revision: Some(SettingsRevision(Uuid::nil())),
                preferences: validate(Preferences {
                    theme: old.theme,
                    language: Language::Ru,
                    exclusions: old.exclusions,
                })
                .map_err(|_| ApiError::SettingsInvalid)?,
            })
        }
        Some(_) => Err(ApiError::Unsupported),
        None => Err(ApiError::SettingsInvalid),
    }
}

#[cfg(feature = "qa-fixtures")]
pub fn qa_directory() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var_os("PC_CARE_FIXTURE_ROOT")?);
    let allowed = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/native-fixtures")
        .canonicalize()
        .ok()?;
    let actual = root.canonicalize().ok()?;
    if actual.parent()? != allowed || Uuid::parse_str(actual.file_name()?.to_str()?).is_err() {
        return None;
    }
    Some(actual.join("config"))
}

#[cfg(test)]
#[path = "preferences_tests.rs"]
mod tests;
