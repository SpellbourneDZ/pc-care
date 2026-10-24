use super::*;
use std::cell::RefCell;

fn plan(simulated: bool) -> OperationPlan {
    OperationPlan {
        id: PlanId::new(),
        scan_id: ScanId::new(),
        simulated,
        targets: (0..3)
            .map(|index| Candidate {
                id: CandidateId::new(),
                source: ScanKind::Cleanup,
                source_key: "user-temp-v1".into(),
                rule_id: "user-temp-v1".into(),
                display_path: format!("C:\\fixture\\{index}.tmp"),
                identity: "opaque".into(),
                bytes: ByteCount(4096),
                logical_bytes: ByteCount(1),
                safety: SafetyClass::Safe,
            })
            .collect(),
        planned_bytes: ByteCount(3 * 4096),
        expires_in_ms: 30000,
        warnings: vec![],
    }
}

#[test]
fn intent_precedes_each_effect_and_cancel_preserves_acknowledged_results() {
    let plan = plan(false);
    let cancelled = AtomicBool::new(false);
    let saved = RefCell::new(Vec::new());
    let result = run(
        &plan,
        &cancelled,
        |record| {
            saved.borrow_mut().push(record.clone());
            Ok(())
        },
        |candidate, cancel| {
            let stored = saved.borrow();
            let item = stored
                .last()
                .unwrap()
                .items
                .iter()
                .find(|item| item.candidate_id == candidate.id)
                .unwrap();
            assert_eq!(item.status, ExecutionItemStatus::Started);
            cancel.store(true, Ordering::Release);
            Ok(RecycleReceipt)
        },
    )
    .unwrap();
    assert_eq!(result.phase, ExecutionPhase::Cancelled);
    assert!(result.items[0].recycled);
    assert_eq!(result.items[0].actual_reclaimed_bytes, None);
    assert!(
        result.items[1..]
            .iter()
            .all(|item| item.reason == Some(ExecutionFailure::Cancelled))
    );
}

#[test]
fn failed_journal_write_stops_effects_and_recovery_never_replays_started_items() {
    for fail_at in [0, 1, 2] {
        let plan = plan(false);
        let mut writes = 0;
        let mut effects = 0;
        let mut durable = None;
        assert!(
            run(
                &plan,
                &AtomicBool::new(false),
                |record| {
                    if writes == fail_at {
                        return Err(ApiError::AdapterError);
                    }
                    writes += 1;
                    durable = Some(record.clone());
                    Ok(())
                },
                |_, _| {
                    effects += 1;
                    Ok(RecycleReceipt)
                }
            )
            .is_err()
        );
        assert_eq!(effects, usize::from(fail_at == 2));
        if let Some(mut record) = durable {
            record.recover();
            assert_eq!(record.phase, ExecutionPhase::Interrupted);
            assert_eq!(
                record.items[0].status,
                if effects > 0 {
                    ExecutionItemStatus::Unknown
                } else {
                    ExecutionItemStatus::Skipped
                }
            );
            assert!(
                record
                    .items
                    .iter()
                    .all(|item| item.status != ExecutionItemStatus::Pending
                        && item.status != ExecutionItemStatus::Started)
            );
        }
    }
}

#[test]
fn dry_run_never_calls_effects_and_per_item_failures_remain_distinct() {
    let dry = run(
        &plan(true),
        &AtomicBool::new(false),
        |_| Ok(()),
        |_, _| panic!("dry-run called effect"),
    )
    .unwrap();
    assert!(
        dry.items
            .iter()
            .all(|item| !item.recycled && item.reason == Some(ExecutionFailure::DryRun))
    );
    let failures = [
        ExecutionFailure::Policy(Denial::TargetChanged),
        ExecutionFailure::AdapterError,
        ExecutionFailure::OutcomeUnknown,
    ];
    let mut index = 0;
    let result = run(
        &plan(false),
        &AtomicBool::new(false),
        |_| Ok(()),
        |_, _| {
            let failure = failures[index];
            index += 1;
            Err(failure)
        },
    )
    .unwrap();
    assert_eq!(
        result
            .items
            .iter()
            .map(|item| item.status)
            .collect::<Vec<_>>(),
        vec![
            ExecutionItemStatus::Skipped,
            ExecutionItemStatus::Error,
            ExecutionItemStatus::Unknown
        ]
    );
    let wire = serde_json::to_string(&result).unwrap();
    assert!(!wire.contains("C:\\"));
    assert!(!wire.contains("opaque"));
}

#[test]
fn unknown_outcome_stops_the_batch_and_invalid_targets_never_reach_effects() {
    let mut calls = 0;
    let result = run(
        &plan(false),
        &AtomicBool::new(false),
        |_| Ok(()),
        |_, _| {
            calls += 1;
            Err(ExecutionFailure::OutcomeUnknown)
        },
    )
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(result.phase, ExecutionPhase::Interrupted);
    assert!(
        result.items[1..]
            .iter()
            .all(|item| item.reason == Some(ExecutionFailure::Interrupted))
    );
    for invalid in 0..4 {
        let mut plan = plan(false);
        match invalid {
            0 => plan.targets[0].safety = SafetyClass::Protected,
            1 => plan.targets[0].source_key = "foreign".into(),
            2 => plan.planned_bytes = ByteCount(0),
            _ => plan.targets[1].id = plan.targets[0].id,
        }
        assert!(
            run(
                &plan,
                &AtomicBool::new(false),
                |_| panic!("invalid intent"),
                |_, _| panic!("invalid effect")
            )
            .is_err()
        );
    }
}

#[test]
fn contradictory_durable_outcomes_are_rejected() {
    let completed = run(
        &plan(false),
        &AtomicBool::new(false),
        |_| Ok(()),
        |_, _| Ok(RecycleReceipt),
    )
    .unwrap();
    completed.validate().unwrap();
    for invalid in 0..4 {
        let mut record = completed.clone();
        let item = &mut record.items[0];
        match invalid {
            0 => item.actual_reclaimed_bytes = Some(ByteCount(4096)),
            1 => {
                item.status = ExecutionItemStatus::Skipped;
                item.recycled = false;
                item.reason = Some(ExecutionFailure::OutcomeUnknown);
            }
            2 => {
                item.status = ExecutionItemStatus::Error;
                item.recycled = false;
                item.reason = Some(ExecutionFailure::Cancelled);
            }
            _ => {
                item.status = ExecutionItemStatus::Skipped;
                item.recycled = false;
                item.reason = Some(ExecutionFailure::DryRun);
                item.actual_reclaimed_bytes = Some(ByteCount(0));
            }
        }
        assert_eq!(record.validate(), Err(ApiError::InvalidRequest));
    }
}
