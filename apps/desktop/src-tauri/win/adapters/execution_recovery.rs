//! Private recovery evidence, separate from the pathless result ledger. Never replayed.
use super::*;
use crate::{
    domain::{execution::ExecutionItemStatus, policy::FileMetadata},
    win::adapters::{files::FileReservation, path::LocalPath},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryMapping {
    schema_version: u32,
    plan_id: PlanId,
    candidate_id: crate::domain::contracts::CandidateId,
    rule_id: String,
    original_path: String,
    metadata: FileMetadata,
    stage_id: Uuid,
    resolved: bool,
}

impl JournalSession {
    /// Persists the original identity before a future handle-based rename; creates no staging file.
    pub fn prepare_recovery(
        &self,
        plan_id: PlanId,
        reservation: &FileReservation<'_>,
    ) -> Result<PathBuf, ApiError> {
        let storage = LocalPath::parse(self.root.to_str().ok_or(ApiError::Protected)?)
            .map_err(|_| ApiError::Protected)?;
        if storage.within(reservation.source_root()) || reservation.source_root().within(&storage) {
            return Err(ApiError::Protected);
        }
        if !self.pending_recoveries()?.is_empty() {
            return Err(ApiError::Capacity);
        }
        let snapshot = reservation.snapshot();
        let record = self.read(plan_id)?;
        if record.simulated
            || record.phase != crate::domain::execution::ExecutionPhase::Running
            || !record.items.iter().any(|item| {
                item.candidate_id == snapshot.id()
                    && item.rule_id == snapshot.source().key()
                    && item.planned_bytes.0 == snapshot.metadata().allocated_size
                    && item.status == ExecutionItemStatus::Started
            })
        {
            return Err(ApiError::InvalidRequest);
        }
        let replace = match self.read_recovery(plan_id) {
            Ok(previous) => previous.resolved,
            Err(ApiError::NotFound) => false,
            Err(error) => return Err(error),
        };
        let mapping = RecoveryMapping {
            schema_version: 1,
            plan_id,
            candidate_id: snapshot.id(),
            rule_id: snapshot.source().key().into(),
            original_path: snapshot.path().into(),
            metadata: snapshot.metadata().clone(),
            stage_id: Uuid::new_v4(),
            resolved: false,
        };
        let path = self.root.join(format!("{}.staged", mapping.stage_id));
        let bytes = serde_json::to_vec(&mapping).map_err(|_| ApiError::Internal)?;
        self.write_bytes(&self.recovery_path(plan_id), &bytes, replace)?;
        Ok(path)
    }

    pub(crate) fn recovery_pending(&self, id: PlanId) -> Result<bool, ApiError> {
        match self.read_recovery(id) {
            Ok(mapping) => Ok(!mapping.resolved),
            Err(ApiError::NotFound) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn pending_recoveries(&self) -> Result<Vec<PlanId>, ApiError> {
        let mut pending = Vec::new();
        let mut mappings = 0;
        for (index, entry) in std::fs::read_dir(&self.root)
            .map_err(|_| ApiError::AdapterError)?
            .enumerate()
        {
            if index >= MAX_RECORDS * 4 {
                return Err(ApiError::Capacity);
            }
            let path = entry.map_err(|_| ApiError::AdapterError)?.path();
            if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("recovery"))
            {
                mappings += 1;
                if mappings > MAX_RECORDS {
                    return Err(ApiError::Capacity);
                }
                let id = PlanId(
                    Uuid::parse_str(
                        path.file_stem()
                            .and_then(|s| s.to_str())
                            .ok_or(ApiError::AdapterError)?,
                    )
                    .map_err(|_| ApiError::AdapterError)?,
                );
                let mapping = self.read_recovery(id).map_err(|error| match error {
                    ApiError::NotFound => ApiError::AdapterError,
                    other => other,
                })?;
                if !mapping.resolved {
                    pending.push(id);
                }
            }
        }
        Ok(pending)
    }

    /// Only a previously durable successful receipt can settle the mapping, without file effects.
    pub(crate) fn settle_recovery(&self, id: PlanId) -> Result<(), ApiError> {
        let mut mapping = match self.read_recovery(id) {
            Ok(mapping) => mapping,
            Err(ApiError::NotFound) => return Ok(()),
            Err(error) => return Err(error),
        };
        if mapping.resolved {
            return Ok(());
        }
        let record = self.read(id)?;
        if record.items.iter().any(|item| {
            item.candidate_id == mapping.candidate_id && item.status == ExecutionItemStatus::Success
        }) {
            mapping.resolved = true;
            let bytes = serde_json::to_vec(&mapping).map_err(|_| ApiError::Internal)?;
            self.write_bytes(&self.recovery_path(id), &bytes, true)?;
        }
        Ok(())
    }

    fn read_recovery(&self, id: PlanId) -> Result<RecoveryMapping, ApiError> {
        let mapping: RecoveryMapping =
            serde_json::from_slice(&self.read_bytes(&self.recovery_path(id))?)
                .map_err(|_| ApiError::AdapterError)?;
        if mapping.schema_version != 1 {
            return Err(ApiError::Unsupported);
        }
        let path = LocalPath::parse(&mapping.original_path).map_err(|_| ApiError::AdapterError)?;
        let record = self.read(id).map_err(|error| match error {
            ApiError::NotFound => ApiError::AdapterError,
            other => other,
        })?;
        if mapping.plan_id != id
            || record.simulated
            || path.is_root()
            || mapping.metadata.links != 1
            || mapping.stage_id.get_version_num() != 4
            || !record.items.iter().any(|item| {
                item.candidate_id == mapping.candidate_id
                    && item.rule_id == mapping.rule_id
                    && item.planned_bytes.0 == mapping.metadata.allocated_size
                    && item.status != ExecutionItemStatus::Pending
                    && (!mapping.resolved || item.status == ExecutionItemStatus::Success)
            })
        {
            return Err(ApiError::AdapterError);
        }
        Ok(mapping)
    }

    fn recovery_path(&self, id: PlanId) -> PathBuf {
        self.root.join(format!("{}.recovery", id.0))
    }
}

#[cfg(test)]
#[path = "execution_recovery_tests.rs"]
mod tests;
