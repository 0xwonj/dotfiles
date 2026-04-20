use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use miette::{IntoDiagnostic, Result};
use tempfile::TempDir;

pub struct TestSandbox {
    _temp: TempDir,
    root: PathBuf,
    pub home: PathBuf,
    pub bin: PathBuf,
    pub log: PathBuf,
}

impl TestSandbox {
    pub fn new() -> Result<Self> {
        let temp = tempfile::tempdir().into_diagnostic()?;
        let root = temp.path().to_path_buf();
        let home = root.join("home");
        let bin = root.join("bin");
        let log = root.join("commands.log");
        fs::create_dir_all(&home).into_diagnostic()?;
        fs::create_dir_all(&bin).into_diagnostic()?;
        fs::write(&log, "").into_diagnostic()?;
        Ok(Self {
            _temp: temp,
            root,
            home,
            bin,
            log,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path_env(&self) -> OsString {
        std::env::join_paths([self.bin.clone()]).expect("join fake PATH")
    }

    pub fn write_script(&self, name: &str, body: &str) -> Result<PathBuf> {
        let path = self.bin.join(name);
        fs::write(&path, body).into_diagnostic()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).into_diagnostic()?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).into_diagnostic()?;
        }
        Ok(path)
    }

    pub fn write_logged_script(&self, name: &str, body: &str) -> Result<PathBuf> {
        let script = format!(
            "#!/bin/sh\nLOG=\"{}\"\necho \"{} $*\" >> \"$LOG\"\n{}\n",
            self.log.display(),
            name,
            body
        );
        self.write_script(name, &script)
    }

    pub fn create_home_command(&self, relative: &str, body: &str) -> Result<PathBuf> {
        let path = self.home.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).into_diagnostic()?;
        }
        fs::write(&path, body).into_diagnostic()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).into_diagnostic()?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).into_diagnostic()?;
        }
        Ok(path)
    }

    pub fn read_log(&self) -> Result<String> {
        fs::read_to_string(&self.log).into_diagnostic()
    }
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonical workspace root")
}
