use super::contracts::*;

pub const COUNT: u32 = 3;

pub fn candidates() -> Vec<Candidate> {
    [
        ("temp", 1024, SafetyClass::Safe),
        ("cache", 2048, SafetyClass::Review),
        ("protected", 4096, SafetyClass::Protected),
    ]
    .into_iter()
    .map(|(key, bytes, safety)| Candidate {
        id: CandidateId::new(),
        source: ScanKind::Mock,
        source_key: key.into(),
        display_path: format!("mock://fixtures/{key}"),
        identity: format!("fixture:{key}:v1"),
        rule_id: "mock-fixture".into(),
        bytes: ByteCount(bytes),
        logical_bytes: ByteCount(bytes),
        safety,
    })
    .collect()
}

pub fn revalidate(candidate: &Candidate) -> bool {
    candidates().iter().any(|current| {
        current.source_key == candidate.source_key
            && current.identity == candidate.identity
            && current.display_path == candidate.display_path
            && current.rule_id == candidate.rule_id
            && current.logical_bytes == candidate.logical_bytes
            && current.bytes == candidate.bytes
            && current.safety == candidate.safety
            && current.source == candidate.source
    })
}
