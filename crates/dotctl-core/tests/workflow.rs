use std::fs;
use std::path::Path;

use dotctl_core::config::{StateSnapshot, load_installer_registry};
use dotctl_core::system::Runtime;
use dotctl_core::{App, ApplyOptions, BootstrapOptions, DiffOptions, DiffOutcome, UpdateOptions};
use dotctl_testkit::{TestSandbox, workspace_root};
use miette::{IntoDiagnostic, Result};

fn make_runtime(sandbox: &TestSandbox) -> Runtime {
    let mut paths = vec![sandbox.bin.clone()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    Runtime::new(
        sandbox.home.clone(),
        std::env::join_paths(paths).expect("join fake workflow PATH"),
    )
}

fn neovim_asset_name() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "nvim-linux-x86_64.tar.gz",
        ("linux", "aarch64") => "nvim-linux-arm64.tar.gz",
        ("macos", "x86_64") => "nvim-macos-x86_64.tar.gz",
        ("macos", "aarch64") => "nvim-macos-arm64.tar.gz",
        other => panic!("unsupported test platform for neovim asset: {other:?}"),
    }
}

fn install_common_stubs(sandbox: &TestSandbox) -> Result<()> {
    let asset = neovim_asset_name();
    let curl = format!(
        r#"#!/bin/sh
LOG="{}"
raw="$*"
echo "curl $raw" >> "$LOG"
out=""
while [ $# -gt 0 ]; do
  case "$1" in
    --output)
      out="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
case "$raw" in
  *api.github.com/repos/neovim/neovim/releases/latest*)
    cat <<'JSON'
{{"tag_name":"v0.11.4","assets":[{{"name":"{asset}","browser_download_url":"https://example.invalid/{asset}","digest":"sha256:deadbeef"}}]}}
JSON
    ;;
  *)
    if [ -n "$out" ]; then
      : > "$out"
    fi
    ;;
esac
exit 0
"#,
        sandbox.log.display()
    );
    sandbox.write_script("curl", &curl)?;

    sandbox.write_logged_script(
        "git", "exit 0
",
    )?;
    sandbox.write_logged_script(
        "brew",
        r#"if [ "$1" = "list" ]; then
  exit 1
fi
exit 0
"#,
    )?;
    sandbox.write_logged_script(
        "chezmoi",
        r#"case "$1" in
  apply)
    mkdir -p "$HOME/.config/nvim"
    : > "$HOME/.config/nvim/init.lua"
    exit 0
    ;;
  verify)
    exit 0
    ;;
  diff)
    if [ -f "$HOME/.fake-chezmoi-dirty" ]; then
      exit 1
    fi
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
"#,
    )?;
    sandbox.write_logged_script(
        "zsh", "exit 0
",
    )?;
    sandbox.write_logged_script(
        "nvim",
        r#"if [ "$1" = "--version" ]; then
  echo "NVIM v0.11.4"
  exit 0
fi
case "$raw" in
  *print_treesitter_languages*)
    printf '\n'
    ;;
  *print_mason_packages*)
    printf '\n'
    ;;
esac
exit 0
"#,
    )?;
    sandbox.write_logged_script(
        "uv",
        r#"if [ "$1" = "tool" ] && [ "$2" = "list" ]; then
  echo "pynvim v0.5.0"
  exit 0
fi
exit 0
"#,
    )?;
    sandbox.write_logged_script(
        "cargo",
        r#"if [ "$1" = "install" ] && [ "$2" = "--list" ]; then
  echo "tree-sitter-cli v0.25.0:"
  exit 0
fi
exit 0
"#,
    )?;
    sandbox.write_logged_script(
        "rustup", "exit 0
",
    )?;
    for cmd in [
        "node", "npm", "cc", "make", "tar", "unzip", "fzf", "zoxide", "eza",
    ] {
        sandbox.write_logged_script(cmd, "exit 0\n")?;
    }

    sandbox.create_home_command(".local/bin/nvim", "#!/bin/sh\nexec nvim \"$@\"\n")?;
    sandbox.create_home_command(
        ".local/opt/nvim-v0.11.4/bin/nvim",
        "#!/bin/sh\nexec nvim \"$@\"\n",
    )?;
    fs::create_dir_all(sandbox.home.join(".antidote/.git")).into_diagnostic()?;
    Ok(())
}

fn write_local_config(home: &Path, repo_root: &Path) -> Result<()> {
    let local = format!(
        r#"profile = "minimal"

[repo]
source_dir = "{}"

[features]
github = false
terminal_apps = false
git_lfs = false
ai_tools = false
fastfetch = false

[identity.git]
name = "Temp User"
email = "temp@example.com"
signing_key = ""
gpg_program = ""
sign_commits = false

[system]
package_manager_override = "brew"
"#,
        repo_root.display()
    );
    let path = home.join(".config/dotfiles/local.toml");
    fs::create_dir_all(path.parent().unwrap()).into_diagnostic()?;
    fs::write(path, local).into_diagnostic()?;
    Ok(())
}

#[test]
fn bootstrap_writes_local_state_generated_config_and_snapshot() -> Result<()> {
    let sandbox = TestSandbox::new()?;
    install_common_stubs(&sandbox)?;
    let repo_root = workspace_root();
    let app = App::from_runtime(make_runtime(&sandbox));

    app.bootstrap(BootstrapOptions {
        repo: Some(repo_root.clone()),
        profile: Some("minimal".to_string()),
        package_manager: Some("brew".to_string()),
        git_name: Some("Temp User".to_string()),
        git_email: Some("temp@example.com".to_string()),
        no_check: true,
        no_prompt: true,
        ..BootstrapOptions::default()
    })?;

    let local = sandbox.home.join(".config/dotfiles/local.toml");
    let state = sandbox.home.join(".config/dotfiles/state.toml");
    let chezmoi = sandbox.home.join(".config/chezmoi/chezmoi.toml");
    assert!(local.is_file());
    assert!(state.is_file());
    assert!(chezmoi.is_file());

    let snapshot: StateSnapshot =
        toml::from_str(&fs::read_to_string(&state).into_diagnostic()?).into_diagnostic()?;
    assert_eq!(snapshot.package_manager, "brew");
    assert_eq!(snapshot.profile, "minimal");
    assert_eq!(
        snapshot.applied_bundles,
        vec!["core".to_string(), "dev".to_string()]
    );

    let chezmoi_raw = fs::read_to_string(&chezmoi).into_diagnostic()?;
    assert!(chezmoi_raw.contains("sourceDir"));
    assert!(chezmoi_raw.contains("Temp User"));
    assert!(chezmoi_raw.contains("terminal_apps = false"));

    Ok(())
}

#[test]
fn update_reuses_local_state_and_records_snapshot() -> Result<()> {
    let sandbox = TestSandbox::new()?;
    install_common_stubs(&sandbox)?;
    let repo_root = workspace_root();
    write_local_config(&sandbox.home, &repo_root)?;
    let app = App::from_runtime(make_runtime(&sandbox));

    app.update(UpdateOptions { no_check: true })?;

    let state = sandbox.home.join(".config/dotfiles/state.toml");
    assert!(state.is_file());
    let log = sandbox.read_log()?;
    assert!(log.contains("uv self update"));
    assert!(log.contains("rustup self update"));
    assert!(log.contains("rustup update"));
    Ok(())
}

#[test]
fn apply_does_not_write_snapshot() -> Result<()> {
    let sandbox = TestSandbox::new()?;
    install_common_stubs(&sandbox)?;
    let repo_root = workspace_root();
    write_local_config(&sandbox.home, &repo_root)?;
    let app = App::from_runtime(make_runtime(&sandbox));

    app.apply(ApplyOptions::default())?;

    assert!(sandbox.home.join(".config/chezmoi/chezmoi.toml").is_file());
    assert!(!sandbox.home.join(".config/dotfiles/state.toml").exists());
    Ok(())
}

#[test]
fn diff_reports_clean_and_dirty() -> Result<()> {
    let sandbox = TestSandbox::new()?;
    install_common_stubs(&sandbox)?;
    let repo_root = workspace_root();
    write_local_config(&sandbox.home, &repo_root)?;
    let app = App::from_runtime(make_runtime(&sandbox));

    assert_eq!(app.diff(DiffOptions::default())?, DiffOutcome::Clean);
    fs::write(sandbox.home.join(".fake-chezmoi-dirty"), "1").into_diagnostic()?;
    assert_eq!(app.diff(DiffOptions::default())?, DiffOutcome::Dirty);
    Ok(())
}

#[test]
fn installer_registry_is_the_source_of_installer_metadata() -> Result<()> {
    let registry = load_installer_registry(&workspace_root())?;

    let starship = registry.installers.get("starship").expect("starship spec");
    assert_eq!(starship.kind, "script");
    assert_eq!(starship.interpreter.as_deref(), Some("sh"));
    assert_eq!(starship.binary.as_deref(), Some("starship"));
    assert!(starship.args.iter().any(|arg| arg == "{{home}}/.local/bin"));

    let codex = registry.installers.get("codex").expect("codex spec");
    assert_eq!(codex.kind, "npm");
    assert_eq!(codex.package.as_deref(), Some("@openai/codex"));
    assert_eq!(codex.prefix.as_deref(), Some("{{home}}/.local"));

    let neovim = registry.installers.get("neovim").expect("neovim spec");
    assert_eq!(neovim.kind, "github_release");
    assert_eq!(neovim.asset_strategy.as_deref(), Some("neovim"));
    assert_eq!(neovim.binary.as_deref(), Some("nvim"));

    Ok(())
}
