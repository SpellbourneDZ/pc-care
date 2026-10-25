use super::*;
use crate::domain::{
    engine::Engine,
    storage::{StorageQuery, StorageSort, StorageView},
};
use std::{path::PathBuf, sync::mpsc};

fn query() -> StorageQuery {
    StorageQuery {
        parent_id: 0,
        view: StorageView::Files,
        search: String::new(),
        extension: String::new(),
        min_bytes: ByteCount(0),
        modified_before_ms: None,
        sort: StorageSort::Size,
        offset: 0,
        limit: 500,
    }
}
fn fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/storage-fixtures")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&root).unwrap();
    root.canonicalize().unwrap()
}
fn scan(root: &std::path::Path, exclusions: &[String]) -> StoragePage {
    let scanner = StorageScanner::current(root.to_str().unwrap(), exclusions).unwrap();
    let engine = Engine::new(false);
    let (tx, rx) = mpsc::channel();
    engine
        .start_storage_scan(
            30_000,
            move |progress| tx.send(progress).is_ok(),
            move |sink| scanner.scan(sink),
        )
        .unwrap();
    let terminal = rx.iter().last().unwrap();
    assert_eq!(
        terminal.status,
        TaskStatus::Completed,
        "{:?}",
        terminal.error
    );
    engine
        .get_storage_page(terminal.scan_id.unwrap(), query())
        .unwrap()
}
#[test]
fn iterative_metadata_scan_preserves_fixtures_and_skips_hardlinks_junctions_and_exclusions() {
    let root = fixture();
    fs::create_dir_all(root.join("папка/внутри")).unwrap();
    fs::write(
        root.join("папка/внутри/данные.bin"),
        b"synthetic storage fixture",
    )
    .unwrap();
    fs::write(root.join("ignored.txt"), b"excluded").unwrap();
    fs::write(root.join("link-a.txt"), b"hardlinked fixture").unwrap();
    fs::hard_link(root.join("link-a.txt"), root.join("link-b.txt")).unwrap();
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(root.join("loop"))
        .arg(&root)
        .output()
        .unwrap();
    assert!(status.status.success());
    let page = scan(&root, &[root.join("ignored.txt").to_str().unwrap().into()]);
    assert!(!page.truncated);
    assert_eq!(
        page.root.logical_bytes.0,
        b"synthetic storage fixture".len() as u64
    );
    assert!(page.root.bytes.0 > 0);
    assert_eq!(
        page.issues
            .iter()
            .find(|i| i.reason == Denial::HardLink)
            .unwrap()
            .count,
        2
    );
    assert!(page.issues.iter().any(|i| i.reason == Denial::ReparsePoint));
    assert!(page.issues.iter().any(|i| i.reason == Denial::Excluded));
    assert_eq!(
        fs::read(root.join("папка/внутри/данные.bin")).unwrap(),
        b"synthetic storage fixture"
    );
    assert_eq!(
        page.rows
            .iter()
            .find(|n| n.name == "данные.bin")
            .unwrap()
            .logical_bytes
            .0,
        25
    );
}
#[test]
fn root_exclusion_and_nonlocal_input_never_start_a_report() {
    let root = fixture();
    assert!(StorageScanner::current(r"\\server\share", &[]).is_err());
    let scanner =
        StorageScanner::current(root.to_str().unwrap(), &[root.to_str().unwrap().into()]).unwrap();
    assert_eq!(scanner.scan(&mut |_| Ok(())), Err(ApiError::Protected));
}

#[test]
fn explorer_location_uses_private_identity_and_fresh_exclusions_without_opening_shell() {
    let root = fixture();
    let path = root.join("selected.bin");
    fs::write(&path, b"synthetic location fixture").unwrap();
    let page = scan(&root, &[]);
    let node = page.rows.iter().find(|n| n.name == "selected.bin").unwrap();
    assert!(location_guards(node, &[]).is_ok());
    assert!(
        serde_json::to_value(node)
            .unwrap()
            .get("identity")
            .is_none()
    );
    assert!(matches!(
        location_guards(node, &[root.to_str().unwrap().into()]),
        Err(ApiError::Protected)
    ));
    fs::rename(&path, root.join("previous-artificial.bin")).unwrap();
    fs::write(&path, b"synthetic replacement").unwrap();
    assert!(matches!(
        location_guards(node, &[]),
        Err(ApiError::TargetChanged)
    ));
    assert!(serde_json::from_value::<ApiRequest>(serde_json::json!({"type":"open_storage_location","data":{"scan_id":ScanId::new(),"node_id":0,"path":"C:/Windows"}})).is_err());
}

#[test]
fn compressed_allocation_and_open_file_metadata_are_reported_without_reading_contents() {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::{IO::DeviceIoControl, Ioctl::FSCTL_SET_COMPRESSION};
    let root = fixture();
    let compressed = root.join("compressed.bin");
    fs::write(&compressed, vec![42; 65536]).unwrap();
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&compressed)
        .unwrap();
    let format: u16 = 1;
    let mut returned = 0;
    assert_ne!(
        unsafe {
            DeviceIoControl(
                file.as_raw_handle(),
                FSCTL_SET_COMPRESSION,
                (&format as *const u16).cast(),
                2,
                ptr::null_mut(),
                0,
                &mut returned,
                ptr::null_mut(),
            )
        },
        0
    );
    drop(file);
    let locked = root.join("locked.bin");
    fs::write(&locked, b"locked artificial fixture").unwrap();
    let guard = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&locked)
        .unwrap();
    let page = scan(&root, &[]);
    // READ_ATTRIBUTES can inspect metadata even when file contents deny all sharing.
    assert!(
        page.rows
            .iter()
            .any(|n| n.name == "locked.bin" && n.skipped.is_none())
    );
    let file = page
        .rows
        .iter()
        .find(|n| n.name == "compressed.bin")
        .unwrap();
    assert_eq!(file.logical_bytes.0, 65536);
    assert!(file.bytes.0 < file.logical_bytes.0);
    drop(guard);
    assert_eq!(fs::read(compressed).unwrap(), vec![42; 65536]);
}

#[test]
fn storage_cancel_has_no_report_and_rescan_expires_the_previous_snapshot() {
    let root = fixture();
    fs::write(root.join("unchanged.bin"), b"artificial").unwrap();
    let engine = Engine::new(false);
    let other = engine.clone();
    let (tx, rx) = mpsc::channel();
    let scanner = StorageScanner::current(root.to_str().unwrap(), &[]).unwrap();
    engine
        .start_storage_scan(
            30_000,
            move |p| {
                if p.status == TaskStatus::Running {
                    other.cancel_task(p.task_id).unwrap();
                }
                tx.send(p).is_ok()
            },
            move |sink| scanner.scan(sink),
        )
        .unwrap();
    let terminal = rx.iter().last().unwrap();
    assert_eq!(terminal.status, TaskStatus::Cancelled);
    assert!(terminal.scan_id.is_none());
    let run = || {
        for _ in 0..100 {
            let scanner = StorageScanner::current(root.to_str().unwrap(), &[]).unwrap();
            let (tx, rx) = mpsc::channel();
            match engine.start_storage_scan(
                30_000,
                move |p| tx.send(p).is_ok(),
                move |sink| scanner.scan(sink),
            ) {
                Ok(_) => return rx.iter().last().unwrap().scan_id.unwrap(),
                Err(ApiError::Capacity) => std::thread::sleep(std::time::Duration::from_millis(1)),
                Err(error) => panic!("unexpected start error: {error:?}"),
            }
        }
        panic!("previous read-only worker did not exit")
    };
    let first = run();
    assert!(engine.get_storage_page(first, query()).is_ok());
    let second = run();
    assert!(matches!(
        engine.get_storage_page(first, query()),
        Err(ApiError::NotFound)
    ));
    assert!(engine.get_storage_page(second, query()).is_ok());
    let history = engine
        .storage_last(&format!(
            "{}\\",
            &LocalPath::parse(root.to_str().unwrap()).unwrap().as_str()[..2]
        ))
        .unwrap()
        .unwrap();
    assert!(history.1 > 0);
    assert_eq!(
        history.0,
        LocalPath::parse(root.to_str().unwrap()).unwrap().as_str()
    );
    assert_eq!(fs::read(root.join("unchanged.bin")).unwrap(), b"artificial");
}

#[test]
fn storage_node_cap_publishes_only_a_bounded_partial_report() {
    let engine = Engine::new(false);
    let (tx, rx) = mpsc::channel();
    engine
        .start_storage_scan(
            30_000,
            move |p| tx.send(p).is_ok(),
            |sink| {
                for id in 0..=crate::domain::storage::MAX_NODES as u32 {
                    let path = LocalPath::parse(&format!(r"C:\fixture\{id}")).unwrap();
                    let mut item = node(id, if id == 0 { None } else { Some(0) }, &path, id == 0);
                    item.bytes = ByteCount(u64::from(id != 0));
                    item.logical_bytes = item.bytes;
                    sink(ScanEvent::StorageNode(item))?;
                }
                Ok(())
            },
        )
        .unwrap();
    let terminal = rx.iter().last().unwrap();
    assert_eq!(terminal.status, TaskStatus::Completed);
    let page = engine
        .get_storage_page(terminal.scan_id.unwrap(), query())
        .unwrap();
    assert!(page.truncated);
    assert_eq!(page.node_count, 100000);
    assert_eq!(page.rows.len(), 500);
    assert_eq!(page.root.bytes.0, 99999);
    assert!(page.issues.iter().any(|i| i.reason == Denial::LimitReached));
}
