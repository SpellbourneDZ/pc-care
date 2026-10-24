//! Metadata-only scans of closed backend roots. No delete/write APIs.
use super::{
    files::{FilePolicy, denial_from_io},
    path::LocalPath,
};
use crate::domain::{
    cleanup::RuleId,
    contracts::*,
    engine::{ScanEvent, ScanSink},
    policy::Denial,
};
#[cfg(not(any(test, feature = "qa-fixtures")))]
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::{
    collections::HashSet,
    os::windows::fs::MetadataExt,
    time::{SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;
#[cfg(not(any(test, feature = "qa-fixtures")))]
use windows_sys::Win32::{Foundation::*, System::Diagnostics::ToolHelp::*};

pub struct CleanupScanner {
    policy: FilePolicy,
    #[cfg(any(test, feature = "qa-fixtures"))]
    processes: Result<HashSet<String>, Denial>,
    now: i64,
}
impl CleanupScanner {
    pub fn current(exclusions: &[String]) -> Result<Self, ApiError> {
        #[cfg(not(feature = "qa-fixtures"))]
        let mut policy = FilePolicy::for_current_user().map_err(|_| ApiError::Protected)?;
        #[cfg(feature = "qa-fixtures")]
        let mut policy = FilePolicy::for_fixture(
            &super::preferences::qa_directory()
                .and_then(|path| path.parent().map(|path| path.join("Profile")))
                .ok_or(ApiError::SettingsUnavailable)?,
        )
        .map_err(|_| ApiError::Protected)?;
        for exclusion in exclusions {
            policy
                .exclusions
                .add(exclusion)
                .map_err(|_| ApiError::InvalidRequest)?;
        }
        let ticks = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ApiError::Internal)?
            .as_nanos()
            / 100
            + 116_444_736_000_000_000;
        let now = i64::try_from(ticks).map_err(|_| ApiError::Internal)?;
        Ok(Self {
            policy,
            #[cfg(any(test, feature = "qa-fixtures"))]
            processes: Ok(HashSet::new()),
            now,
        })
    }

    pub fn scan(&self, sink: &mut ScanSink<'_>) -> Result<(), ApiError> {
        for rule in RuleId::ALL {
            sink(ScanEvent::Checkpoint)?;
            if let Some(process) = rule.process_name() {
                #[cfg(not(any(test, feature = "qa-fixtures")))]
                let processes = process_names();
                #[cfg(any(test, feature = "qa-fixtures"))]
                let processes = self.processes.clone();
                match &processes {
                    Err(reason) => {
                        issue(sink, rule, None, *reason)?;
                        continue;
                    }
                    Ok(names) if names.contains(process) => {
                        issue(sink, rule, None, Denial::ApplicationRunning)?;
                        continue;
                    }
                    _ => {}
                }
            }
            match self.policy.root(rule) {
                Ok(root) => self.walk(rule, &root, sink)?,
                Err(reason) => issue(sink, rule, None, reason)?,
            }
        }
        Ok(())
    }

    fn walk(
        &self,
        rule: RuleId,
        directory: &LocalPath,
        sink: &mut ScanSink<'_>,
    ) -> Result<(), ApiError> {
        sink(ScanEvent::Checkpoint)?;
        let entries = match self.policy.directory(rule, directory) {
            Ok(entries) => entries,
            Err(reason) => return issue(sink, rule, Some(directory.as_str()), reason),
        };
        for entry in entries {
            sink(ScanEvent::Visited)?;
            let path = match entry {
                Ok(path) => path,
                Err(reason) => {
                    issue(sink, rule, Some(directory.as_str()), reason)?;
                    continue;
                }
            };
            let local = match path
                .to_str()
                .ok_or(Denial::UnsupportedPath)
                .and_then(LocalPath::parse)
            {
                Ok(local) => local,
                Err(reason) => {
                    issue(sink, rule, None, reason)?;
                    continue;
                }
            };
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0 => {
                    self.walk(rule, &local, sink)?
                }
                Ok(_) => self.file(rule, &local, sink)?,
                Err(error) => issue(sink, rule, Some(local.as_str()), denial_from_io(error))?,
            }
        }
        Ok(())
    }
    fn file(
        &self,
        rule: RuleId,
        path: &LocalPath,
        sink: &mut ScanSink<'_>,
    ) -> Result<(), ApiError> {
        let id = CandidateId::new();
        let snapshot = match self.policy.snapshot(id, rule, path.as_str()) {
            Ok(snapshot) => snapshot,
            Err(reason) => return issue(sink, rule, Some(path.as_str()), reason),
        };
        if let Err(reason) = rule.check_age(snapshot.metadata(), self.now) {
            return issue(sink, rule, Some(path.as_str()), reason);
        }
        if rule == RuleId::UserTemp
            && !path.as_str().rsplit('.').next().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("tmp") || extension.eq_ignore_ascii_case("temp")
            })
        {
            return issue(sink, rule, Some(path.as_str()), Denial::UnverifiedType);
        }
        sink(ScanEvent::Checkpoint)?;
        if let Err(reason) = self.policy.revalidate(id, &snapshot) {
            return issue(sink, rule, Some(path.as_str()), reason);
        }
        let descriptor = rule.descriptor();
        sink(ScanEvent::Candidate(Candidate {
            id,
            source: ScanKind::Cleanup,
            source_key: descriptor.id.clone(),
            rule_id: descriptor.id,
            display_path: snapshot.path().into(),
            safety: descriptor.safety,
            bytes: ByteCount(snapshot.metadata().allocated_size),
            logical_bytes: ByteCount(snapshot.metadata().size),
            identity: serde_json::to_string(snapshot.metadata()).map_err(|_| ApiError::Internal)?,
        }))
    }
}
fn issue(
    sink: &mut ScanSink<'_>,
    rule: RuleId,
    path: Option<&str>,
    reason: Denial,
) -> Result<(), ApiError> {
    sink(ScanEvent::Issue(ScanIssue {
        rule_id: rule.descriptor().id,
        reason,
        count: 1,
        sample_path: path.map(str::to_owned),
    }))
}

#[cfg(not(any(test, feature = "qa-fixtures")))]
fn process_names() -> Result<HashSet<String>, Denial> {
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if raw == INVALID_HANDLE_VALUE {
        return Err(Denial::Unavailable);
    }
    let _handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut names = HashSet::new();
    let mut present = unsafe { Process32FirstW(raw, &mut entry) };
    while present != 0 {
        let end = entry
            .szExeFile
            .iter()
            .position(|c| *c == 0)
            .ok_or(Denial::Unavailable)?;
        names.insert(
            String::from_utf16(&entry.szExeFile[..end])
                .map_err(|_| Denial::Unavailable)?
                .to_ascii_lowercase(),
        );
        present = unsafe { Process32NextW(raw, &mut entry) };
    }
    if unsafe { GetLastError() } != ERROR_NO_MORE_FILES {
        return Err(Denial::Unavailable);
    }
    Ok(names)
}

#[cfg(test)]
#[path = "cleanup_tests.rs"]
mod tests;
