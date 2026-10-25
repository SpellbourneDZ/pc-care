//! A private execution ledger, not a replay queue or user-selected output path.
use super::preferences::{check_file, directory_guard};
use crate::domain::{
    contracts::{ApiError, PlanId},
    execution::ExecutionRecord,
};
use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Write},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use windows_sys::Win32::Storage::FileSystem::*;

const MAX_BYTES: u64 = 256 * 1024;
const MAX_RECORDS: usize = 64;

pub struct JournalSession {
    root: PathBuf,
    _directories: Vec<File>,
    _lease: File,
}

impl JournalSession {
    pub fn open(root: &Path, create: bool) -> Result<Self, ApiError> {
        let directories = directory_guard(root, create)
            .map_err(|_| ApiError::AdapterError)?
            .ok_or(ApiError::NotFound)?;
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(root.join("execution.lock"))
            .map_err(|_| ApiError::AdapterError)?;
        check_file(&lease).map_err(|_| ApiError::AdapterError)?;
        lease.try_lock().map_err(|_| ApiError::Capacity)?;
        Ok(Self {
            root: root.to_owned(),
            _directories: directories,
            _lease: lease,
        })
    }

    pub fn read(&self, id: PlanId) -> Result<ExecutionRecord, ApiError> {
        let record: ExecutionRecord = serde_json::from_slice(&self.read_bytes(&self.path(id))?)
            .map_err(|_| ApiError::AdapterError)?;
        record.validate()?;
        if record.plan_id != id {
            return Err(ApiError::AdapterError);
        }
        Ok(record)
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, ApiError> {
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
            .map_err(|error| {
                if error.kind() == ErrorKind::NotFound {
                    ApiError::NotFound
                } else {
                    ApiError::AdapterError
                }
            })?;
        check_file(&file).map_err(|_| ApiError::AdapterError)?;
        let mut data = Vec::new();
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut data)
            .map_err(|_| ApiError::AdapterError)?;
        if data.len() as u64 > MAX_BYTES {
            return Err(ApiError::AdapterError);
        }
        Ok(data)
    }

    pub fn list(&self) -> Result<Vec<ExecutionRecord>, ApiError> {
        let mut records = Vec::new();
        for (index, entry) in std::fs::read_dir(&self.root)
            .map_err(|_| ApiError::AdapterError)?
            .enumerate()
        {
            if index >= MAX_RECORDS * 4 {
                return Err(ApiError::Capacity);
            }
            let path = entry.map_err(|_| ApiError::AdapterError)?.path();
            if path.extension().is_some_and(|value| value == "json") {
                let name = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .ok_or(ApiError::AdapterError)?;
                let id = PlanId(Uuid::parse_str(name).map_err(|_| ApiError::AdapterError)?);
                if records.len() == MAX_RECORDS {
                    return Err(ApiError::Capacity);
                }
                records.push(self.read(id)?);
            }
        }
        Ok(records)
    }

    pub fn create(&self, record: &ExecutionRecord) -> Result<(), ApiError> {
        if self.list()?.len() >= MAX_RECORDS {
            return Err(ApiError::Capacity);
        }
        match self.read(record.plan_id) {
            Ok(_) => return Err(ApiError::AlreadyExecuted),
            Err(ApiError::NotFound) => {}
            Err(error) => return Err(error),
        }
        self.write(record, false)
    }

    pub fn replace(&self, record: &ExecutionRecord) -> Result<(), ApiError> {
        let previous = self.read(record.plan_id)?;
        if previous.simulated != record.simulated
            || previous.planned_bytes != record.planned_bytes
            || previous.items.len() != record.items.len()
            || previous
                .items
                .iter()
                .zip(&record.items)
                .any(|(before, after)| {
                    before.candidate_id != after.candidate_id
                        || before.rule_id != after.rule_id
                        || before.planned_bytes != after.planned_bytes
                })
        {
            return Err(ApiError::InvalidRequest);
        }
        self.write(record, true)
    }

    fn path(&self, id: PlanId) -> PathBuf {
        self.root.join(format!("{}.json", id.0))
    }

    fn write(&self, record: &ExecutionRecord, replace: bool) -> Result<(), ApiError> {
        record.validate()?;
        let data = serde_json::to_vec(record).map_err(|_| ApiError::Internal)?;
        self.write_bytes(&self.path(record.plan_id), &data, replace)
    }

    fn write_bytes(&self, destination: &Path, data: &[u8], replace: bool) -> Result<(), ApiError> {
        if data.len() as u64 > MAX_BYTES {
            return Err(ApiError::Capacity);
        }
        let temporary = self.root.join(format!("{}.tmp", Uuid::new_v4()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&temporary)
            .map_err(|_| ApiError::AdapterError)?;
        file.write_all(data)
            .and_then(|_| file.sync_all())
            .map_err(|_| ApiError::AdapterError)?;
        drop(file);
        let wide = |path: &Path| {
            path.as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>()
        };
        let flags = MOVEFILE_WRITE_THROUGH
            | if replace {
                MOVEFILE_REPLACE_EXISTING
            } else {
                0
            };
        if unsafe { MoveFileExW(wide(&temporary).as_ptr(), wide(destination).as_ptr(), flags) } == 0
        {
            return Err(ApiError::AdapterError);
        }
        Ok(())
    }
}

#[path = "execution_recovery.rs"]
mod recovery;

#[cfg(test)]
#[path = "execution_journal_tests.rs"]
mod tests;
