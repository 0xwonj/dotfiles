use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use miette::{Context, IntoDiagnostic, Result};

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create directory {}", parent.display()))
}

pub fn expand_tilde(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home.join(rest);
    }
    PathBuf::from(path)
}

pub fn write_secure_file(path: &Path, content: &str) -> Result<()> {
    ensure_parent_dir(path)?;
    let mut file = fs::File::create(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", path.display()))?;
    file.write_all(content.as_bytes())
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to write {}", path.display()))?;
    set_mode_600(path)?;
    Ok(())
}

#[cfg(unix)]
pub fn set_mode_600(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to stat {}", path.display()))?
        .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to set permissions on {}", path.display()))
}

#[cfg(not(unix))]
pub fn set_mode_600(_path: &Path) -> Result<()> {
    Ok(())
}
