//! Local DOS paths only. No filesystem access and no linguistic/Unicode normalization.
use crate::domain::policy::Denial;
use windows_sys::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};

#[derive(Debug, Clone)]
pub struct LocalPath(String);

impl LocalPath {
    pub fn parse(input: &str) -> Result<Self, Denial> {
        let path = input.replace('/', "\\");
        let path = path.strip_prefix(r"\\?\").unwrap_or(&path);
        let bytes = path.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || &bytes[1..3] != b":\\"
            || path.encode_utf16().count() > 32_000
            || path.split('\\').count() > 257
        {
            return Err(Denial::UnsupportedPath);
        }
        let path = path.trim_end_matches('\\');
        for part in path[2..].split('\\').skip(1) {
            let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            if part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with(['.', ' '])
                || part
                    .chars()
                    .any(|c| c.is_control() || ":*?\"<>|".contains(c))
                || ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str())
                || stem
                    .strip_prefix("COM")
                    .or_else(|| stem.strip_prefix("LPT"))
                    .is_some_and(|n| {
                        ["1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³"].contains(&n)
                    })
            {
                return Err(Denial::UnsupportedPath);
            }
        }
        Ok(Self(if path.len() == 2 {
            format!("{}\\", path)
        } else {
            path.into()
        }))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn is_root(&self) -> bool {
        self.0.len() == 3
    }
    pub fn child(&self, relative: &str) -> Result<Self, Denial> {
        Self::parse(&format!("{}\\{}", self.0.trim_end_matches('\\'), relative))
    }
    pub fn within(&self, parent: &Self) -> bool {
        let mut parts = self.0.split('\\');
        parent
            .0
            .trim_end_matches('\\')
            .split('\\')
            .all(|p| parts.next().is_some_and(|s| ordinal_equal(s, p)))
    }
    pub fn prefixes(&self) -> Vec<Self> {
        let mut result = vec![Self(format!("{}\\", &self.0[..2]))];
        let mut current = self.0[..2].to_string();
        for part in self.0[3..].split('\\').filter(|p| !p.is_empty()) {
            current.push('\\');
            current.push_str(part);
            result.push(Self(current.clone()));
        }
        result
    }
    pub fn wide(&self) -> Vec<u16> {
        format!(r"\\?\{}", self.0)
            .encode_utf16()
            .chain([0])
            .collect()
    }
    pub(super) fn wide_path(&self) -> String {
        format!(r"\\?\{}", self.0)
    }
}

fn ordinal_equal(a: &str, b: &str) -> bool {
    let a: Vec<u16> = a.encode_utf16().collect();
    let b: Vec<u16> = b.encode_utf16().collect();
    // Buffers are valid for their explicit UTF-16 lengths (bounded by parse).
    unsafe {
        CompareStringOrdinal(a.as_ptr(), a.len() as i32, b.as_ptr(), b.len() as i32, 1)
            == CSTR_EQUAL
    }
}
impl PartialEq for LocalPath {
    fn eq(&self, other: &Self) -> bool {
        ordinal_equal(&self.0, &other.0)
    }
}
impl Eq for LocalPath {}
