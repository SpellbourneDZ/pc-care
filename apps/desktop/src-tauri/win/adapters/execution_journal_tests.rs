use super::*;
use crate::domain::{contracts::*, execution::*};
use std::fs;

pub(super) fn fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/policy-fixtures")
        .join(Uuid::new_v4().to_string());
    fs::create_dir_all(&root).unwrap();
    root.canonicalize().unwrap()
}
pub(super) fn plan() -> OperationPlan {
    OperationPlan {
        id: PlanId::new(),
        scan_id: ScanId::new(),
        simulated: false,
        planned_bytes: ByteCount(4096),
        expires_in_ms: 30000,
        warnings: vec![],
        targets: vec![Candidate {
            id: CandidateId::new(),
            source: ScanKind::Cleanup,
            source_key: "user-temp-v1".into(),
            rule_id: "user-temp-v1".into(),
            display_path: "C:\\fixture\\artificial.tmp".into(),
            identity: "opaque".into(),
            bytes: ByteCount(4096),
            logical_bytes: ByteCount(1),
            safety: SafetyClass::Safe,
        }],
    }
}
fn record() -> ExecutionRecord {
    ExecutionRecord::new(&plan()).unwrap()
}

#[test]
fn exclusive_lease_atomic_updates_and_restart_recovery_preserve_unknown_outcomes() {
    let root = fixture();
    let session = JournalSession::open(&root, true).unwrap();
    assert!(matches!(
        JournalSession::open(&root, true),
        Err(ApiError::Capacity)
    ));
    let mut record = record();
    session.create(&record).unwrap();
    assert_eq!(session.create(&record), Err(ApiError::AlreadyExecuted));
    record.items[0].status = ExecutionItemStatus::Started;
    session.replace(&record).unwrap();
    drop(session);
    let reopened = JournalSession::open(&root, false).unwrap();
    let mut recovered = reopened.read(record.plan_id).unwrap();
    recovered.recover();
    reopened.replace(&recovered).unwrap();
    assert_eq!(
        reopened.read(record.plan_id).unwrap().items[0].status,
        ExecutionItemStatus::Unknown
    );
    assert_eq!(reopened.list().unwrap().len(), 1);
    let bytes = fs::read_to_string(reopened.path(record.plan_id)).unwrap();
    assert!(!bytes.contains("artificial.tmp"));
    assert!(!bytes.contains("opaque"));
}

#[test]
fn malformed_future_locked_hardlinked_and_redirected_ledgers_fail_closed() {
    let root = fixture();
    let session = JournalSession::open(&root, true).unwrap();
    for content in ["{broken", "{\"schema_version\":99}"] {
        let id = PlanId::new();
        let path = session.path(id);
        fs::write(&path, content).unwrap();
        assert!(session.read(id).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
    }
    let mut future = record();
    future.schema_version = 99;
    let future_bytes = serde_json::to_vec(&future).unwrap();
    fs::write(session.path(future.plan_id), &future_bytes).unwrap();
    assert!(matches!(
        session.read(future.plan_id),
        Err(ApiError::Unsupported)
    ));
    assert_eq!(
        fs::read(session.path(future.plan_id)).unwrap(),
        future_bytes
    );
    let other = fixture();
    let other_session = JournalSession::open(&other, true).unwrap();
    let record = record();
    other_session.create(&record).unwrap();
    let lock = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(other_session.path(record.plan_id))
        .unwrap();
    assert!(other_session.read(record.plan_id).is_err());
    drop(lock);
    let outside = root.join("artificial-sentinel.txt");
    fs::write(&outside, b"must remain").unwrap();
    let id = PlanId::new();
    fs::hard_link(&outside, other_session.path(id)).unwrap();
    assert!(other_session.read(id).is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"must remain");
    drop(other_session);
    let redirected = root.join("redirected");
    crate::win::adapters::files::tests::junction(&redirected, &other);
    assert!(JournalSession::open(&redirected, false).is_err());
}

#[test]
fn failed_replacement_preserves_durable_intent() {
    let root = fixture();
    let session = JournalSession::open(&root, true).unwrap();
    let record = record();
    session.create(&record).unwrap();
    let original = fs::read(session.path(record.plan_id)).unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(session.path(record.plan_id))
        .unwrap();
    let mut changed = record.clone();
    changed.items[0].status = ExecutionItemStatus::Started;
    assert_eq!(session.replace(&changed), Err(ApiError::AdapterError));
    assert_eq!(fs::read(session.path(record.plan_id)).unwrap(), original);
    drop(held);
    assert_eq!(
        session.read(record.plan_id).unwrap().items[0].status,
        ExecutionItemStatus::Pending
    );
}

#[test]
fn journal_child_process() {
    let Some(root) = std::env::var_os("PC_CARE_JOURNAL_CHILD_ROOT") else {
        return;
    };
    let root = PathBuf::from(root).canonicalize().unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/policy-fixtures")
        .canonicalize()
        .unwrap();
    assert_eq!(root.parent(), Some(fixtures.as_path()));
    Uuid::parse_str(root.file_name().unwrap().to_str().unwrap()).unwrap();
    match std::env::var("PC_CARE_JOURNAL_CHILD_MODE")
        .unwrap()
        .as_str()
    {
        "lease" => {
            assert!(matches!(
                JournalSession::open(&root, false),
                Err(ApiError::Capacity)
            ));
            std::process::exit(23);
        }
        "intent" => {
            let id = PlanId(
                Uuid::parse_str(&std::env::var("PC_CARE_JOURNAL_CHILD_PLAN").unwrap()).unwrap(),
            );
            let session = JournalSession::open(&root, false).unwrap();
            let mut record = session.read(id).unwrap();
            record.items[0].status = ExecutionItemStatus::Started;
            session.replace(&record).unwrap();
            // Exit bypasses Rust destructors; the OS must release the journal lease.
            std::process::exit(77);
        }
        _ => panic!("unknown child scenario"),
    }
}

#[test]
fn process_exit_releases_lease_and_durable_intent_is_never_replayed() {
    use std::{os::windows::process::CommandExt, process::Command, thread, time::Duration};
    let root = fixture();
    let plan = plan();
    let session = JournalSession::open(&root, true).unwrap();
    session
        .create(&ExecutionRecord::new(&plan).unwrap())
        .unwrap();
    let child = |mode: &str| {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "win::adapters::execution_journal::tests::journal_child_process",
                "--test-threads=1",
            ])
            .env("PC_CARE_JOURNAL_CHILD_ROOT", &root)
            .env("PC_CARE_JOURNAL_CHILD_MODE", mode)
            .env("PC_CARE_JOURNAL_CHILD_PLAN", plan.id.0.to_string())
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status.code();
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("journal child timed out");
            }
            thread::sleep(Duration::from_millis(10));
        }
    };
    assert_eq!(child("lease"), Some(23));
    drop(session);
    assert_eq!(child("intent"), Some(77));
    let manager = super::super::execution::ExecutionManager::new(Some(root));
    let recovered = manager
        .start(plan, |_, _, _| panic!("replayed effect after process exit"))
        .unwrap();
    assert_eq!(recovered.record.phase, ExecutionPhase::Interrupted);
    assert_eq!(
        recovered.record.items[0].status,
        ExecutionItemStatus::Unknown
    );
}
