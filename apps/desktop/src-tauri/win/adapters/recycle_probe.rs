//! Disposable VM experiment only; not registered in IPC and never takes a caller path.
#![allow(non_snake_case)]
use std::{
    cell::RefCell,
    fs::{File, OpenOptions},
    mem::{offset_of, size_of},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
    path::{Path, PathBuf},
    rc::Rc,
};
use windows::{
    Win32::{
        Foundation::E_ABORT,
        System::{Com::*, SystemInformation::*},
        UI::Shell::*,
    },
    core::{HRESULT, PCWSTR, Ref, implement},
};
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_READ_ATTRIBUTES, FILE_RENAME_INFO,
    FILE_SHARE_READ, FileIdInfo, FileRenameInfo, GetFileInformationByHandleEx,
    SetFileInformationByHandle,
};

#[derive(Default)]
struct Receipt {
    pre: bool,
    post: Option<(i32, bool)>,
}
#[implement(IFileOperationProgressSink)]
struct Sink {
    receipt: Rc<RefCell<Receipt>>,
}
impl IFileOperationProgressSink_Impl for Sink_Impl {
    fn StartOperations(&self) -> windows::core::Result<()> {
        Ok(())
    }
    fn FinishOperations(&self, _: HRESULT) -> windows::core::Result<()> {
        Ok(())
    }
    fn PreDeleteItem(&self, flags: u32, _: Ref<IShellItem>) -> windows::core::Result<()> {
        if flags & TSF_DELETE_RECYCLE_IF_POSSIBLE.0 as u32 == 0 {
            return Err(E_ABORT.into());
        }
        self.receipt.borrow_mut().pre = true;
        Ok(())
    }
    fn PostDeleteItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        result: HRESULT,
        recycled: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        self.receipt.borrow_mut().post = Some((result.0, recycled.as_ref().is_some()));
        Ok(())
    }
    fn PreRenameItem(&self, _: u32, _: Ref<IShellItem>, _: &PCWSTR) -> windows::core::Result<()> {
        Err(E_ABORT.into())
    }
    fn PostRenameItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: &PCWSTR,
        _: HRESULT,
        _: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        Err(E_ABORT.into())
    }
    fn PreMoveItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: Ref<IShellItem>,
        _: &PCWSTR,
    ) -> windows::core::Result<()> {
        Err(E_ABORT.into())
    }
    fn PostMoveItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: Ref<IShellItem>,
        _: &PCWSTR,
        _: HRESULT,
        _: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        Err(E_ABORT.into())
    }
    fn PreCopyItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: Ref<IShellItem>,
        _: &PCWSTR,
    ) -> windows::core::Result<()> {
        Err(E_ABORT.into())
    }
    fn PostCopyItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: Ref<IShellItem>,
        _: &PCWSTR,
        _: HRESULT,
        _: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        Err(E_ABORT.into())
    }
    fn PreNewItem(&self, _: u32, _: Ref<IShellItem>, _: &PCWSTR) -> windows::core::Result<()> {
        Err(E_ABORT.into())
    }
    fn PostNewItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: &PCWSTR,
        _: &PCWSTR,
        _: u32,
        _: HRESULT,
        _: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        Err(E_ABORT.into())
    }
    fn UpdateProgress(&self, _: u32, _: u32) -> windows::core::Result<()> {
        Ok(())
    }
    fn ResetTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }
    fn PauseTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }
    fn ResumeTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }
}

/// A test marker only; the caller must separately verify VM isolation.
pub fn disposable_vm() -> bool {
    let provider = FIRMWARE_TABLE_PROVIDER(u32::from_be_bytes(*b"RSMB"));
    let size = unsafe { GetSystemFirmwareTable(provider, 0, None) };
    if !(8..=1024 * 1024).contains(&size) {
        return false;
    }
    let mut table = vec![0; size as usize];
    if unsafe { GetSystemFirmwareTable(provider, 0, Some(&mut table)) } != size {
        return false;
    }
    let mut offset = 8;
    while offset + 4 <= table.len() {
        let kind = table[offset];
        let length = table[offset + 1] as usize;
        if length < 4 || offset + length >= table.len() {
            return false;
        }
        let strings = &table[offset + length..];
        let Some(end) = strings.windows(2).position(|part| part == [0, 0]) else {
            return false;
        };
        if kind == 1 && length >= 8 {
            let values = strings[..end].split(|byte| *byte == 0).collect::<Vec<_>>();
            let value = |index: u8| {
                index
                    .checked_sub(1)
                    .and_then(|index| values.get(index as usize))
                    .copied()
            };
            return value(table[offset + 4]) == Some(b"PC Care VM".as_slice())
                && value(table[offset + 5]) == Some(b"Disposable test VM".as_slice());
        }
        offset += length + end + 2;
    }
    false
}

struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() }
    }
}

fn identity(file: &File) -> std::io::Result<(u64, [u8; 16])> {
    let mut info = FILE_ID_INFO::default();
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            size_of::<FILE_ID_INFO>() as u32,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok((info.VolumeSerialNumber, info.FileId.Identifier))
}

fn rename_held(file: &File, destination: &Path) -> std::io::Result<()> {
    let name: Vec<u16> = destination.as_os_str().encode_wide().collect();
    if !destination.is_absolute() || name.len() > 32_000 || name.contains(&0) {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let bytes = offset_of!(FILE_RENAME_INFO, FileName) + (name.len() + 1) * 2;
    // usize storage gives the variable-length Win32 structure its required alignment.
    let mut storage = vec![0_usize; bytes.div_ceil(size_of::<usize>())];
    let info = storage.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    unsafe {
        (*info).Anonymous.ReplaceIfExists = false;
        (*info).RootDirectory = std::ptr::null_mut();
        (*info).FileNameLength = (name.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(name.as_ptr(), (*info).FileName.as_mut_ptr(), name.len());
        if SetFileInformationByHandle(
            file.as_raw_handle(),
            FileRenameInfo,
            info.cast(),
            bytes as u32,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

pub fn run() -> Result<String, Box<dyn std::error::Error>> {
    if !disposable_vm() {
        return Err("Not the isolated PC Care VM; no filesystem changes made".into());
    }
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).ok()? };
    let _apartment = Apartment;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/vm-probes")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&root)?;
    let root = root.canonicalize()?;
    let mut observations = Vec::new();
    for case in ["pinned", "released", "staged"] {
        let pinned = case == "pinned";
        let staged = case == "staged";
        let original = root.join(format!("artificial-{case}.tmp"));
        let mut path = original.clone();
        std::fs::write(&path, b"PC Care disposable VM fixture")?;
        let mut guard = if pinned || staged {
            Some(
                OpenOptions::new()
                    .read(true)
                    .access_mode(DELETE | FILE_READ_ATTRIBUTES)
                    .share_mode(FILE_SHARE_READ)
                    .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                    .open(&path)?,
            )
        } else {
            None
        };
        let mut staged_identity_preserved = None;
        let mut collision_preserved = None;
        if staged {
            let held = guard.as_ref().ok_or("Missing staging handle")?;
            let before = identity(held)?;
            let directory = root.join("stage");
            std::fs::create_dir(&directory)?;
            let collision = directory.join("collision-sentinel.dat");
            std::fs::write(&collision, b"collision sentinel must remain")?;
            assert!(rename_held(held, &collision).is_err());
            let unchanged = original.exists()
                && std::fs::read(&collision)? == b"collision sentinel must remain";
            assert!(unchanged, "Existing destination was changed");
            collision_preserved = Some(unchanged);
            path = directory.join(format!("{}.dat", uuid::Uuid::new_v4()));
            rename_held(held, &path)?;
            staged_identity_preserved = Some(before == identity(held)?);
            assert_eq!(staged_identity_preserved, Some(true));
            assert!(!original.exists());
            std::fs::write(&original, b"replacement sentinel must remain")?;
            drop(guard.take());
        }
        let path_string = path
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let receipt = Rc::new(RefCell::new(Receipt::default()));
        let sink: IFileOperationProgressSink = Sink {
            receipt: receipt.clone(),
        }
        .into();
        let outcome = (|| -> windows::core::Result<(Option<i32>, bool)> {
            let operation: IFileOperation =
                unsafe { CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER)? };
            unsafe {
                operation.SetOperationFlags(
                    FOFX_RECYCLEONDELETE
                        | FOFX_ADDUNDORECORD
                        | FOFX_EARLYFAILURE
                        | FOF_NOERRORUI
                        | FOF_SILENT
                        | FOF_NOCONFIRMATION
                        | FOF_WANTNUKEWARNING
                        | FOF_NO_CONNECTED_ELEMENTS
                        | FOF_NORECURSION,
                )?
            };
            let item: IShellItem =
                unsafe { SHCreateItemFromParsingName(PCWSTR(path_string.as_ptr()), None)? };
            unsafe { operation.DeleteItem(&item, &sink)? };
            let performed = unsafe { operation.PerformOperations() };
            let aborted = unsafe { operation.GetAnyOperationsAborted() }?;
            Ok((
                performed.err().map(|error| error.code().0),
                aborted.as_bool(),
            ))
        })();
        let exists = path.exists();
        if pinned {
            assert!(exists, "Pinned fixture unexpectedly removed");
        }
        let replacement_untouched = if staged {
            let unchanged = std::fs::read(&original)? == b"replacement sentinel must remain";
            assert!(unchanged, "Replacement at original path was changed");
            Some(unchanged)
        } else {
            None
        };
        let receipt = receipt.borrow();
        observations.push(serde_json::json!({"case":case,"pinned":pinned,"operation":outcome.as_ref().map_err(|error|error.code().0),"predelete":receipt.pre,"postdelete":receipt.post,"original_exists":exists,"staged_identity_preserved":staged_identity_preserved,"replacement_untouched":replacement_untouched,"collision_preserved":collision_preserved}));
        drop(guard);
    }
    Ok(serde_json::to_string(&observations)?)
}
