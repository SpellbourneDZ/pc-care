//! Immutable metadata-only disk results; none of these IDs grant mutation rights.
use super::{contracts::*, policy::Denial};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_NODES: usize = 100_000;
pub const MAX_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct VolumeInfo {
    pub path: String,
    pub filesystem: Option<String>,
    pub total_bytes: Option<ByteCount>,
    pub free_bytes: Option<ByteCount>,
    pub available_bytes: Option<ByteCount>,
    pub error: Option<ApiError>,
    pub last_scan_path: Option<String>,
    #[ts(type = "number | null")]
    pub last_scan_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StorageNode {
    pub id: u32,
    pub parent_id: Option<u32>,
    pub name: String,
    pub path: String,
    pub directory: bool,
    pub bytes: ByteCount,
    pub logical_bytes: ByteCount,
    #[ts(type = "number | null")]
    pub modified_ms: Option<u64>,
    pub skipped: Option<Denial>,
    #[serde(skip)]
    #[ts(skip)]
    pub(crate) identity: Option<(u64, [u8; 16])>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum StorageView {
    Children,
    Files,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum StorageSort {
    Size,
    Name,
    Modified,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StorageQuery {
    pub parent_id: u32,
    pub view: StorageView,
    pub search: String,
    pub extension: String,
    pub min_bytes: ByteCount,
    #[ts(type = "number | null")]
    pub modified_before_ms: Option<u64>,
    pub sort: StorageSort,
    pub offset: u32,
    pub limit: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StoragePage {
    pub scan_id: ScanId,
    pub root: StorageNode,
    pub current: StorageNode,
    pub ancestors: Vec<StorageNode>,
    pub rows: Vec<StorageNode>,
    pub total: u32,
    pub node_count: u32,
    pub issues: Vec<ScanIssue>,
    pub truncated: bool,
    #[ts(type = "number")]
    pub completed_ms: u64,
}

pub fn aggregate(nodes: &mut [StorageNode]) -> Result<(), ApiError> {
    if !nodes.first().is_some_and(|n| n.directory) {
        return Err(ApiError::Internal);
    }
    for index in 0..nodes.len() {
        let node = &nodes[index];
        if node.id as usize != index
            || (index == 0 && node.parent_id.is_some())
            || (index != 0
                && !node
                    .parent_id
                    .is_some_and(|p| (p as usize) < index && nodes[p as usize].directory))
        {
            return Err(ApiError::Internal);
        }
    }
    for index in (1..nodes.len()).rev() {
        let parent = nodes[index].parent_id.ok_or(ApiError::Internal)? as usize;
        nodes[parent].bytes.0 = nodes[parent]
            .bytes
            .0
            .checked_add(nodes[index].bytes.0)
            .ok_or(ApiError::Capacity)?;
        nodes[parent].logical_bytes.0 = nodes[parent]
            .logical_bytes
            .0
            .checked_add(nodes[index].logical_bytes.0)
            .ok_or(ApiError::Capacity)?;
    }
    Ok(())
}

pub fn page(
    scan: &ScanSnapshot,
    nodes: &[StorageNode],
    completed_ms: u64,
    query: StorageQuery,
) -> Result<StoragePage, ApiError> {
    if !(1..=500).contains(&query.limit)
        || query.search.len() > 1024
        || query.extension.len() > 128
        || query
            .modified_before_ms
            .is_some_and(|t| t > 8_640_000_000_000_000)
    {
        return Err(ApiError::InvalidRequest);
    }
    let current = nodes
        .get(query.parent_id as usize)
        .filter(|n| n.directory)
        .ok_or(ApiError::NotFound)?;
    let search = query.search.to_lowercase();
    let extension = query.extension.trim_start_matches('.').to_lowercase();
    let mut inside = vec![false; nodes.len()];
    inside[query.parent_id as usize] = true;
    let mut rows = Vec::new();
    for node in nodes {
        if node.id == query.parent_id {
            continue;
        }
        let descendant = node.parent_id.is_some_and(|p| inside[p as usize]);
        inside[node.id as usize] = descendant;
        let scope = match query.view {
            StorageView::Children => node.parent_id == Some(query.parent_id),
            StorageView::Files => descendant && !node.directory,
        };
        if scope
            && node.name.to_lowercase().contains(&search)
            && (extension.is_empty()
                || (!node.directory
                    && node
                        .name
                        .rsplit_once('.')
                        .is_some_and(|(_, e)| e.to_lowercase() == extension)))
            && node.bytes.0 >= query.min_bytes.0
            && query
                .modified_before_ms
                .is_none_or(|limit| node.modified_ms.is_some_and(|date| date < limit))
        {
            rows.push(node);
        }
    }
    rows.sort_unstable_by(|a, b| {
        (match query.sort {
            StorageSort::Size => b.bytes.0.cmp(&a.bytes.0),
            StorageSort::Name => a.name.cmp(&b.name),
            StorageSort::Modified => b.modified_ms.cmp(&a.modified_ms),
        })
        .then(a.id.cmp(&b.id))
    });
    let total = rows.len() as u32;
    let rows = rows
        .into_iter()
        .skip(query.offset as usize)
        .take(query.limit as usize)
        .cloned()
        .collect();
    let mut ancestors = vec![current.clone()];
    while let Some(parent) = ancestors.last().and_then(|n| n.parent_id) {
        ancestors.push(nodes[parent as usize].clone());
    }
    ancestors.reverse();
    Ok(StoragePage {
        scan_id: scan.id,
        root: nodes.first().ok_or(ApiError::Internal)?.clone(),
        current: current.clone(),
        ancestors,
        rows,
        total,
        node_count: nodes.len() as u32,
        issues: scan.issues.clone(),
        truncated: scan.truncated,
        completed_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: u32, parent_id: Option<u32>, directory: bool, bytes: u64) -> StorageNode {
        StorageNode {
            id,
            parent_id,
            directory,
            name: format!("файл-{id}.bin"),
            path: format!(r"C:\fixture\{id}"),
            bytes: ByteCount(bytes),
            logical_bytes: ByteCount(bytes),
            modified_ms: Some(10),
            skipped: None,
            identity: None,
        }
    }
    fn query() -> StorageQuery {
        StorageQuery {
            parent_id: 1,
            view: StorageView::Files,
            search: String::new(),
            extension: "bin".into(),
            min_bytes: ByteCount(0),
            modified_before_ms: Some(11),
            sort: StorageSort::Size,
            offset: 0,
            limit: 100,
        }
    }
    fn snapshot() -> ScanSnapshot {
        ScanSnapshot {
            id: ScanId::new(),
            task_id: ScanTaskId::new(),
            source: ScanKind::Storage,
            candidates: vec![],
            issues: vec![],
            truncated: false,
        }
    }
    #[test]
    fn hundred_thousand_records_keep_exact_aggregates_and_bounded_pages_after_filters() {
        let mut nodes = vec![node(0, None, true, 0), node(1, Some(0), true, 0)];
        nodes
            .extend((2..100_000).map(|id| node(id, Some(1), false, 9_007_199_254_740 + id as u64)));
        let expected: u64 = nodes.iter().map(|n| n.bytes.0).sum();
        aggregate(&mut nodes).unwrap();
        assert_eq!(nodes[0].bytes.0, expected);
        let scan = snapshot();
        let first = page(&scan, &nodes, 12, query()).unwrap();
        assert_eq!(first.rows.len(), 100);
        assert_eq!(first.total, 99_998);
        assert_eq!(first.rows[0].id, 99_999);
        assert_eq!(first.ancestors.len(), 2);
        let mut filtered = query();
        filtered.search = "файл-99".into();
        let filtered = page(&scan, &nodes, 12, filtered).unwrap();
        assert!(filtered.total < first.total);
        assert_eq!(filtered.root.bytes, first.root.bytes);
        assert_eq!(filtered.current.bytes, first.current.bytes);
        assert!(filtered.rows.iter().all(|n| n.name.contains("файл-99")));
        let mut empty = query();
        empty.offset = 100_000;
        assert!(page(&scan, &nodes, 12, empty).unwrap().rows.is_empty());
    }
    #[test]
    fn malformed_hierarchy_overflow_and_hostile_query_are_rejected() {
        assert!(aggregate(&mut []).is_err());
        assert!(aggregate(&mut [node(0, None, true, 0), node(1, Some(2), true, 0)]).is_err());
        assert!(
            aggregate(&mut [node(0, None, true, u64::MAX), node(1, Some(0), false, 1)]).is_err()
        );
        let mut q = query();
        q.limit = 501;
        assert!(matches!(
            page(&snapshot(), &[node(0, None, true, 0)], 0, q),
            Err(ApiError::InvalidRequest)
        ));
    }
}
