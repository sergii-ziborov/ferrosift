#![allow(dead_code)]

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

pub fn run(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ferrosift"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("CLI must start");
    child
        .stdin
        .take()
        .expect("stdin must be piped")
        .write_all(input)
        .expect("test input must write");
    child.wait_with_output().expect("CLI must exit")
}

pub fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout must be UTF-8")
}

pub fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("stderr must be UTF-8")
}

pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(name: &str) -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "ferrosift-cli-{}-{sequence}-{name}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique test directory must be created");
        Self { path }
    }

    pub fn write(&self, name: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, contents).expect("test fixture must write");
        path
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if self.path.starts_with(std::env::temp_dir()) {
            fs::remove_dir_all(&self.path).expect("test directory must be removable");
        }
    }
}

pub fn path_text(path: &Path) -> &str {
    path.to_str().expect("test path must be Unicode")
}
