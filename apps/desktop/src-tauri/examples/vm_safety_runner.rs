#![windows_subsystem = "windows"]
use std::{
    fs,
    io::{self, Write},
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
};

const MB_OKCANCEL: u32 = 1;
const IDOK: i32 = 1;
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(
        window: *mut core::ffi::c_void,
        text: *const u16,
        title: *const u16,
        flags: u32,
    ) -> i32;
}

fn message(text: &str, flags: u32) -> i32 {
    let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "PC Care · VM tests".encode_utf16().chain(Some(0)).collect();
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), flags) }
}

fn copy_binding(source: &Path, destination: &Path) -> io::Result<()> {
    let bytes = fs::read(source)?;
    match fs::read(destination) {
        Ok(existing) if existing == bytes => return Ok(()),
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Fixture bindings differ from payload; use a clean VM workspace",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    file.write_all(&bytes)?;
    file.sync_all()
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    if !pc_care::win::adapters::recycle_probe::disposable_vm() {
        return Err("Not the disposable PC Care VM; no filesystem changes made".into());
    }
    let executable = std::env::current_exe()?;
    let base = executable.parent().ok_or("No payload directory")?;
    let (probe, question, report_name) = match executable.file_stem().and_then(|name| name.to_str())
    {
        Some("vm-test-runner") => (
            false,
            "Запустить Rust-тесты на искусственных файлах?",
            "PCcare-VM-test-results.txt",
        ),
        Some("vm-probe-runner") => (
            true,
            "Проверить перенос по handle и COM Recycle Bin на искусственных файлах?",
            "PCcare-VM-recycle-probe.json",
        ),
        _ => return Err("Use vm-test-runner.exe or vm-probe-runner.exe on the test ISO".into()),
    };
    let child = base.join(if probe {
        "vm_recycle_probe.exe"
    } else {
        "pc-care-tests.exe"
    });
    if !child.is_file() {
        return Err("Companion test executable missing".into());
    }
    if message(
        &format!(
            "{question}\n\nИзменения ограничены новыми тестовыми fixtures внутри VM. Пользовательские каталоги не очищаются. Перед запуском нужен snapshot."
        ),
        MB_OKCANCEL,
    ) != IDOK
    {
        return Ok(0);
    }
    if !probe {
        let generated = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("Invalid compile-time workspace")?
            .join("src/generated");
        fs::create_dir_all(&generated)?;
        let mut count = 0;
        for entry in fs::read_dir(base)? {
            let path = entry?.path();
            if path.extension().is_some_and(|extension| extension == "ts") {
                count += 1;
                if count > 64 {
                    return Err("Too many generated bindings".into());
                }
                copy_binding(
                    &path,
                    &generated.join(path.file_name().ok_or("Invalid binding")?),
                )?;
            }
        }
    }
    let report = std::env::temp_dir().join(report_name);
    let output = fs::File::create(&report)?;
    let mut command = Command::new(child);
    if !probe {
        command.arg("--test-threads=1");
    }
    let status = command
        .current_dir(base)
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(output.try_clone()?)
        .stderr(output.try_clone()?)
        .status()?;
    drop(command);
    output.sync_all()?;
    message(
        &format!(
            "Проверка завершена: {status}\n\nОтчёт: {}",
            report.display()
        ),
        0,
    );
    Ok(status.code().unwrap_or(1))
}

fn main() {
    let code = run().unwrap_or_else(|error| {
        message(&format!("Проверка не выполнена:\n{error}"), 0);
        1
    });
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readonly_fixture_bindings_are_reused_and_different_files_are_preserved() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target/policy-fixtures")
            .join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.ts");
        let existing = root.join("existing.ts");
        fs::write(&source, "type Fixture = string;").unwrap();
        fs::write(&existing, "type Fixture = string;").unwrap();
        let mut permissions = fs::metadata(&existing).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&existing, permissions).unwrap();
        copy_binding(&source, &existing).unwrap();
        assert!(fs::metadata(&existing).unwrap().permissions().readonly());
        let fresh = root.join("fresh.ts");
        copy_binding(&source, &fresh).unwrap();
        assert!(!fs::metadata(&fresh).unwrap().permissions().readonly());
        fs::write(&source, "type Fixture = number;").unwrap();
        assert_eq!(
            copy_binding(&source, &existing).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            fs::read_to_string(existing).unwrap(),
            "type Fixture = string;"
        );
    }
}
