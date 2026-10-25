//! Fixed system executable, held read-only. Signature evidence is not launch authority.
use super::{files, path::LocalPath};
use crate::domain::uninstall::InstallerBinaryStatus as Status;
use std::{
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr,
};
use windows_sys::Win32::{
    Foundation::*,
    Security::{
        Cryptography::{Catalog::*, *},
        WinTrust::*,
    },
    Storage::FileSystem::*,
    System::SystemInformation::GetSystemDirectoryW,
};

pub(super) fn check() -> Status {
    match system_target().and_then(|path| verify(&path)) {
        Ok(status) => status,
        Err(status) => status,
    }
}
fn system_target() -> Result<LocalPath, Status> {
    let mut buffer = vec![0u16; 32768];
    let count = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if count == 0 || count >= buffer.len() {
        return Err(Status::Unavailable);
    }
    let directory = String::from_utf16(&buffer[..count]).map_err(|_| Status::UnsafePath)?;
    LocalPath::parse(&directory)
        .and_then(|p| p.child("msiexec.exe"))
        .map_err(|_| Status::UnsafePath)
}
#[derive(Debug, PartialEq, Eq)]
struct Stamp {
    volume: u64,
    id: [u8; 16],
    size: i64,
    write: i64,
    change: i64,
    attributes: u32,
    links: u32,
}
fn stamp(handle: &OwnedHandle) -> Result<Stamp, Status> {
    let id: FILE_ID_INFO = files::info(handle, FileIdInfo).map_err(|_| Status::Unavailable)?;
    let basic: FILE_BASIC_INFO =
        files::info(handle, FileBasicInfo).map_err(|_| Status::Unavailable)?;
    let standard: FILE_STANDARD_INFO =
        files::info(handle, FileStandardInfo).map_err(|_| Status::Unavailable)?;
    Ok(Stamp {
        volume: id.VolumeSerialNumber,
        id: id.FileId.Identifier,
        size: standard.EndOfFile,
        write: basic.LastWriteTime,
        change: basic.ChangeTime,
        attributes: basic.FileAttributes,
        links: standard.NumberOfLinks,
    })
}
struct Guard {
    handles: Vec<OwnedHandle>,
}
impl Guard {
    fn open(path: &LocalPath) -> Result<Self, Status> {
        let root: Vec<u16> = path.as_str()[..3].encode_utf16().chain([0]).collect();
        if unsafe { GetDriveTypeW(root.as_ptr()) } != 3
        /* DRIVE_FIXED */
        {
            return Err(Status::UnsafePath);
        }
        let prefixes = path.prefixes();
        let mut handles = Vec::new();
        for (index, prefix) in prefixes.iter().enumerate() {
            let leaf = index + 1 == prefixes.len();
            let raw = unsafe {
                CreateFileW(
                    prefix.wide().as_ptr(),
                    if leaf {
                        GENERIC_READ
                    } else {
                        FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY
                    },
                    if leaf {
                        FILE_SHARE_READ
                    } else {
                        FILE_SHARE_READ | FILE_SHARE_WRITE
                    },
                    ptr::null(),
                    OPEN_EXISTING,
                    FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
                    ptr::null_mut(),
                )
            };
            if raw == INVALID_HANDLE_VALUE {
                return Err(Status::Unavailable);
            }
            let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
            let basic: FILE_BASIC_INFO =
                files::info(&handle, FileBasicInfo).map_err(|_| Status::Unavailable)?;
            files::check_attributes(basic.FileAttributes, false).map_err(|_| Status::UnsafePath)?;
            if (basic.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0) == leaf
                || basic.FileAttributes & FILE_ATTRIBUTE_ENCRYPTED != 0
            {
                return Err(Status::UnsafePath);
            }
            if files::final_path(&handle).map_err(|_| Status::UnsafePath)? != *prefix {
                return Err(Status::UnsafePath);
            }
            if !leaf {
                let case: FILE_CASE_SENSITIVE_INFO =
                    files::info(&handle, FileCaseSensitiveInfo).map_err(|_| Status::Unavailable)?;
                if case.Flags != 0 {
                    return Err(Status::UnsafePath);
                }
            }
            // Windows system binaries may be hardlinked to WinSxS. No mutation is authorized here.
            handles.push(handle)
        }
        Ok(Self { handles })
    }
    fn file(&self) -> &OwnedHandle {
        self.handles.last().expect("a parsed local path has a leaf")
    }
}
fn verify(path: &LocalPath) -> Result<Status, Status> {
    let guard = Guard::open(path)?;
    let before = stamp(guard.file())?;
    let status = signature(path, guard.file());
    if stamp(guard.file())? != before
        || files::final_path(guard.file()).map_err(|_| Status::TargetChanged)? != *path
    {
        return Err(Status::TargetChanged);
    }
    Ok(status)
}
struct TrustState {
    data: WINTRUST_DATA,
    action: windows_sys::core::GUID,
}
impl Drop for TrustState {
    fn drop(&mut self) {
        self.data.dwStateAction = WTD_STATEACTION_CLOSE;
        unsafe {
            WinVerifyTrust(
                INVALID_HANDLE_VALUE,
                &mut self.action,
                (&mut self.data as *mut WINTRUST_DATA).cast(),
            )
        };
    }
}
fn signature(path: &LocalPath, file: &OwnedHandle) -> Status {
    let name: Vec<u16> = path.as_str().encode_utf16().chain([0]).collect();
    let mut info = WINTRUST_FILE_INFO {
        cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: name.as_ptr(),
        hFile: file.as_raw_handle(),
        ..Default::default()
    };
    let data = trust_data(WTD_CHOICE_FILE, WINTRUST_DATA_0 { pFile: &mut info });
    match trusted_signature(data) {
        Ok(status) => status,
        Err(code) if code as u32 == 0x800b0100 => catalog_signature(path, file), // TRUST_E_NOSIGNATURE only
        Err(_) => Status::UntrustedSignature,
    }
}
fn trust_data(choice: u32, object: WINTRUST_DATA_0) -> WINTRUST_DATA {
    WINTRUST_DATA {
        cbStruct: size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: choice,
        Anonymous: object,
        dwStateAction: WTD_STATEACTION_VERIFY,
        dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL | WTD_REVOCATION_CHECK_NONE | WTD_DISABLE_MD2_MD4,
        ..Default::default()
    }
}
fn trusted_signature(data: WINTRUST_DATA) -> Result<Status, i32> {
    let mut state = TrustState {
        action: WINTRUST_ACTION_GENERIC_VERIFY_V2,
        data,
    };
    let result = unsafe {
        WinVerifyTrust(
            INVALID_HANDLE_VALUE,
            &mut state.action,
            (&mut state.data as *mut WINTRUST_DATA).cast(),
        )
    };
    if result != 0 {
        return Err(result);
    }
    let provider = unsafe { WTHelperProvDataFromStateData(state.data.hWVTStateData) };
    if provider.is_null() {
        return Ok(Status::UntrustedSignature);
    }
    let signer = unsafe { WTHelperGetProvSignerFromChain(provider, 0, 0, 0) };
    if signer.is_null() {
        return Ok(Status::UntrustedSignature);
    }
    let signer = unsafe { &*signer };
    if signer.csCertChain == 0 || signer.pasCertChain.is_null() {
        return Ok(Status::UntrustedSignature);
    }
    let cert = unsafe { (*signer.pasCertChain).pCert };
    if cert.is_null() {
        return Ok(Status::UntrustedSignature);
    }
    let mut organization = vec![0u16; 512];
    let count = unsafe {
        CertGetNameStringW(
            cert,
            CERT_NAME_ATTR_TYPE,
            0,
            szOID_ORGANIZATION_NAME.cast(),
            organization.as_mut_ptr(),
            organization.len() as u32,
        )
    } as usize;
    if count <= 1 || count > organization.len() {
        return Ok(Status::UntrustedSignature);
    }
    if String::from_utf16(&organization[..count - 1])
        .is_ok_and(|name| name == "Microsoft Corporation")
    {
        Ok(Status::MicrosoftSignatureOffline)
    } else {
        Ok(Status::UntrustedSignature)
    }
}
struct CatalogContext {
    admin: isize,
    current: isize,
}
impl Drop for CatalogContext {
    fn drop(&mut self) {
        unsafe {
            if self.current != 0 {
                CryptCATAdminReleaseCatalogContext(self.admin, self.current, 0);
            }
            CryptCATAdminReleaseContext(self.admin, 0);
        }
    }
}
fn catalog_signature(path: &LocalPath, file: &OwnedHandle) -> Status {
    let mut admin = 0;
    if unsafe {
        CryptCATAdminAcquireContext2(
            &mut admin,
            ptr::null(),
            windows_sys::core::w!("SHA256"),
            ptr::null(),
            0,
        )
    } == 0
        || admin == 0
    {
        return Status::Unavailable;
    }
    let mut context = CatalogContext { admin, current: 0 };
    let mut length = 0;
    if unsafe {
        CryptCATAdminCalcHashFromFileHandle2(
            admin,
            file.as_raw_handle(),
            &mut length,
            ptr::null_mut(),
            0,
        )
    } == 0
        || length != 32
    {
        return Status::Unavailable;
    }
    let mut hash = vec![0u8; length as usize];
    if unsafe {
        CryptCATAdminCalcHashFromFileHandle2(
            admin,
            file.as_raw_handle(),
            &mut length,
            hash.as_mut_ptr(),
            0,
        )
    } == 0
        || length != 32
    {
        return Status::Unavailable;
    }
    let tag: String = hash.iter().map(|b| format!("{b:02X}")).collect();
    let tag: Vec<u16> = tag.encode_utf16().chain([0]).collect();
    let name: Vec<u16> = path.as_str().encode_utf16().chain([0]).collect();
    for _ in 0..16 {
        // Enumeration advances/releases the previous context; Drop releases the final one.
        context.current = unsafe {
            CryptCATAdminEnumCatalogFromHash(admin, hash.as_ptr(), length, 0, &mut context.current)
        };
        if context.current == 0 {
            break;
        }
        let mut catalog = CATALOG_INFO {
            cbStruct: size_of::<CATALOG_INFO>() as u32,
            ..Default::default()
        };
        if unsafe { CryptCATCatalogInfoFromContext(context.current, &mut catalog, 0) } == 0 {
            return Status::Unavailable;
        }
        if !catalog.wszCatalogFile.contains(&0) {
            return Status::Unavailable;
        }
        let mut info = WINTRUST_CATALOG_INFO {
            cbStruct: size_of::<WINTRUST_CATALOG_INFO>() as u32,
            pcwszCatalogFilePath: catalog.wszCatalogFile.as_ptr(),
            pcwszMemberTag: tag.as_ptr(),
            pcwszMemberFilePath: name.as_ptr(),
            hMemberFile: file.as_raw_handle(),
            pbCalculatedFileHash: hash.as_mut_ptr(),
            cbCalculatedFileHash: length,
            hCatAdmin: admin,
            ..Default::default()
        };
        let result = trusted_signature(trust_data(
            WTD_CHOICE_CATALOG,
            WINTRUST_DATA_0 {
                pCatalog: &mut info,
            },
        ));
        if matches!(result, Ok(Status::MicrosoftSignatureOffline)) {
            return Status::MicrosoftSignatureOffline;
        }
    }
    Status::UntrustedSignature
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_windows_installer_has_microsoft_signature_without_launching_it() {
        assert_eq!(check(), Status::MicrosoftSignatureOffline)
    }
    #[test]
    fn tampered_system_binary_copy_cannot_borrow_the_original_catalog_signature() {
        let source = system_target().unwrap();
        let mut bytes = std::fs::read(source.wide_path()).unwrap();
        assert!(bytes.len() > 4096);
        bytes[4096] ^= 1;
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target/installer-binary-fixtures")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("tampered-fixture.exe");
        std::fs::write(&file, bytes).unwrap();
        let path = LocalPath::parse(file.canonicalize().unwrap().to_str().unwrap()).unwrap();
        assert_ne!(verify(&path).unwrap(), Status::MicrosoftSignatureOffline)
    }
    #[test]
    fn unsigned_fixture_is_not_trusted_and_held_file_cannot_be_reopened_for_writing() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target/installer-binary-fixtures")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("unsigned.exe");
        let text = b"artificial fixture, never execute";
        std::fs::write(&file, text).unwrap();
        let path = LocalPath::parse(file.canonicalize().unwrap().to_str().unwrap()).unwrap();
        let guard = Guard::open(&path).unwrap();
        let directory = LocalPath::parse(root.canonicalize().unwrap().to_str().unwrap()).unwrap();
        assert!(matches!(Guard::open(&directory), Err(Status::UnsafePath)));
        assert!(std::fs::OpenOptions::new().write(true).open(&file).is_err());
        assert_eq!(signature(&path, guard.file()), Status::UntrustedSignature);
        assert_eq!(std::fs::read(file).unwrap(), text)
    }
}
