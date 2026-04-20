use std::process::Command;

use dotctl_testkit::{TestSandbox, workspace_root};
use miette::{IntoDiagnostic, Result};

fn actual_path_with_fake_front(sandbox: &TestSandbox) -> std::ffi::OsString {
    let mut paths = vec![sandbox.bin.clone()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    std::env::join_paths(paths).expect("join PATH")
}

#[test]
fn install_sh_skips_package_manager_when_toolchain_is_already_present() -> Result<()> {
    let sandbox = TestSandbox::new()?;
    let log = sandbox.log.display().to_string();

    sandbox.write_logged_script(
        "cargo",
        &format!(
            r#"if [ "$1" = "install" ]; then
  mkdir -p "$HOME/.local/bin"
  cat > "$HOME/.local/bin/dotctl" <<'SH'
#!/bin/sh
LOG="{log}"
echo "dotctl $*" >> "$LOG"
exit 0
SH
  chmod +x "$HOME/.local/bin/dotctl"
fi
exit 0
"#
        ),
    )?;
    sandbox.write_logged_script("rustc", "exit 0\n")?;
    sandbox.write_logged_script("curl", "exit 0\n")?;
    sandbox.write_logged_script("apt-get", "exit 97\n")?;
    sandbox.write_logged_script("dnf", "exit 97\n")?;
    sandbox.write_logged_script("pacman", "exit 97\n")?;
    sandbox.write_logged_script("sudo", "exit 97\n")?;

    let repo_root = workspace_root();
    let status = Command::new("sh")
        .arg(repo_root.join("install.sh"))
        .args([
            "--profile",
            "laptop",
            "--no-prompt",
            "--git-name",
            "Temp User",
            "--git-email",
            "temp@example.com",
        ])
        .env("HOME", &sandbox.home)
        .env("PATH", actual_path_with_fake_front(&sandbox))
        .current_dir(&repo_root)
        .status()
        .into_diagnostic()?;
    assert!(status.success());

    let log = sandbox.read_log()?;
    assert!(log.contains("cargo install --locked"));
    assert!(log.contains("dotctl bootstrap --repo"));
    assert!(!log.contains("apt-get"));
    assert!(!log.contains("dnf"));
    assert!(!log.contains("pacman"));
    assert!(!log.contains("sudo"));
    Ok(())
}
