//! Internal read-only handles. No creation, writing, or IPC path entry point.
use std::{io, ptr};
use windows_sys::Win32::{Foundation::*, System::Registry::*};
pub(super) struct Key(pub HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}
pub(super) fn open(
    hive: HKEY,
    path: &str,
    view: u32,
    enumerate: bool,
) -> io::Result<Option<Vec<Key>>> {
    let mut keys: Vec<Key> = vec![];
    let link: Vec<u16> = "SymbolicLinkValue".encode_utf16().chain([0]).collect();
    for part in path.split('\\') {
        let name: Vec<u16> = part.encode_utf16().chain([0]).collect();
        let mut raw = ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                keys.last().map_or(hive, |k| k.0),
                name.as_ptr(),
                REG_OPTION_OPEN_LINK,
                KEY_QUERY_VALUE | view | if enumerate { KEY_ENUMERATE_SUB_KEYS } else { 0 },
                &mut raw,
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        let key = Key(raw);
        let (mut kind, mut bytes) = (0, 0);
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                link.as_ptr(),
                ptr::null(),
                &mut kind,
                ptr::null_mut(),
                &mut bytes,
            )
        };
        if (status == ERROR_SUCCESS || status == ERROR_MORE_DATA) && kind == REG_LINK {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        keys.push(key);
    }
    Ok(Some(keys))
}
