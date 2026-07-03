//! `ahe setup`: interactive one-time wiring, idempotent on re-run.
//!
//! Endpoint prompts → probe → .env; wizard clone + static musl build; Nix
//! task-base image; uv-managed python env; final doctor table.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use dialoguer::theme::ColorfulTheme;
use indicatif::{ProgressBar, ProgressStyle};

use crate::cfg::{self, AheConfig};
use crate::endpoint;

pub struct Opts {
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub wizard_repo: Option<PathBuf>,
    pub skip_build: bool,
}

const WIZARD_GIT_URL: &str = "https://github.com/teddytennant/wizard.git";
const WIZARD_BRANCH: &str = "feat/harness-bundle";
const MUSL_TARGET: &str = "x86_64-unknown-linux-musl";

pub fn run(opts: Opts) -> Result<()> {
    let theme = ColorfulTheme::default();
    let lab_dir = cfg::find_lab_dir()
        .context("cannot find the wizard-ahe lab (run inside the checkout once)")?;
    println!("lab: {}", lab_dir.display());

    let wizard_repo = resolve_wizard_repo(&opts)?;
    let login_wizard = ensure_login_wizard(&wizard_repo)?;

    let env = cfg::read_env(&lab_dir);
    let endpoint = endpoint::collect(
        &theme,
        &lab_dir,
        &env,
        opts.base_url,
        opts.api_key,
        opts.model,
        Some(login_wizard.as_path()),
    )?;

    let container_url = cfg::container_base_url(&endpoint.base_url);
    cfg::write_env(
        &lab_dir,
        &[
            ("LLM_BASE_URL", &endpoint.base_url),
            ("LLM_API_KEY", &endpoint.api_key),
            ("LLM_MODEL", &endpoint.model),
            ("WIZARD_LLM_BASE_URL", &container_url),
        ],
    )?;
    println!("wrote {}", lab_dir.join(".env").display());
    if container_url != endpoint.base_url {
        println!(
            "task containers will use {container_url} — make sure the host \
             firewall allows docker0 traffic to that port\n  (NixOS: \
             networking.firewall.interfaces.\"docker0\".allowedTCPPorts)"
        );
    }

    let wizard_binary = wizard_repo
        .join("target")
        .join(MUSL_TARGET)
        .join("release/wizard");

    if !opts.skip_build {
        build_wizard_musl(&wizard_repo)?;
        if !is_static(&wizard_binary) {
            bail!(
                "{} is not a static executable — see docs/WIZARD-AHE.md step 0",
                wizard_binary.display()
            );
        }
        if which("nix").is_some() {
            step_command(
                "building Nix task-base image",
                Command::new(lab_dir.join("scripts/build-task-image.sh")).current_dir(&lab_dir),
            )?;
        } else {
            println!(
                "warning: `nix` not found — dataset/wizard images need the \
                 wizard-ahe/task-base image (scripts/build-task-image.sh)"
            );
        }
        step_command(
            "syncing python env (uv)",
            Command::new("uv")
                .args(["sync", "--python-preference", "only-managed"])
                .current_dir(&lab_dir),
        )?;
    }

    let config = AheConfig {
        lab_dir: lab_dir.clone(),
        wizard_repo,
        wizard_binary: wizard_binary.clone(),
    };
    config.save()?;

    println!("\nsetup summary");
    check("docker daemon", docker_ok());
    check("endpoint reachable", !endpoint.models.is_empty());
    check("wizard binary (static musl)", is_static(&wizard_binary));
    check("task-base image loaded", task_image_ok());
    check("python venv", lab_dir.join(".venv/bin/python").is_file());
    println!("\nnext: `ahe run --config wizard-smoke` (wiring gate), then `ahe run`");
    Ok(())
}

fn resolve_wizard_repo(opts: &Opts) -> Result<PathBuf> {
    match opts.wizard_repo.clone().or_else(|| {
        AheConfig::load()
            .ok()
            .map(|existing| existing.wizard_repo)
            .filter(|p| p.join("Cargo.toml").is_file())
    }) {
        Some(p) => Ok(p),
        None => {
            let default_clone = dirs::data_dir()
                .context("no data dir")?
                .join("ahe")
                .join("wizard");
            if !default_clone.join("Cargo.toml").is_file() {
                step_command(
                    "cloning wizard",
                    Command::new("git").args([
                        "clone",
                        "--branch",
                        WIZARD_BRANCH,
                        WIZARD_GIT_URL,
                        default_clone.to_str().unwrap(),
                    ]),
                )?;
            }
            Ok(default_clone)
        }
    }
}

/// Host `wizard` binary used only for `wizard --login xai` during setup.
fn ensure_login_wizard(repo: &Path) -> Result<PathBuf> {
    let host = repo.join("target/release/wizard");
    if host.is_file() {
        return Ok(host);
    }
    if which("wizard").is_some() {
        return Ok(which("wizard").unwrap());
    }
    build_wizard_host(repo)?;
    if host.is_file() {
        Ok(host)
    } else {
        bail!("wizard host build did not produce {}", host.display());
    }
}

fn build_wizard_host(repo: &Path) -> Result<()> {
    let mut cmd = if which("nix-shell").is_some() {
        let mut c = Command::new("nix-shell");
        c.args([
            "-p",
            "rustup",
            "--run",
            "rustup target add x86_64-unknown-linux-gnu >/dev/null 2>&1; cargo build --release",
        ]);
        c
    } else {
        let mut c = Command::new("cargo");
        c.args(["build", "--release"]);
        c
    };
    step_command("building wizard (host, for OAuth login)", cmd.current_dir(repo))
}

/// Static musl build. Prefers the nix-shell recipe (NixOS: nix musl-gcc
/// miscompiles with -static, rust-lld is the reliable linker); falls back to
/// plain cargo for hosts with musl-tools.
fn build_wizard_musl(repo: &Path) -> Result<()> {
    let build = "rustup target add x86_64-unknown-linux-musl >/dev/null 2>&1; \
                 CC_x86_64_unknown_linux_musl=musl-gcc \
                 CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
                 RUSTFLAGS=\"-C target-feature=+crt-static -C link-self-contained=yes\" \
                 cargo build --release --target x86_64-unknown-linux-musl";
    let mut cmd = if which("nix-shell").is_some() {
        let mut c = Command::new("nix-shell");
        c.args(["-p", "rustup", "musl", "--run", build]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", build]);
        c
    };
    step_command("building wizard (static musl)", cmd.current_dir(repo))
}

/// Run a long step behind a spinner; on failure show the captured tail.
fn step_command(label: &str, cmd: &mut Command) -> Result<()> {
    let spinner = ProgressBar::new_spinner()
        .with_style(ProgressStyle::with_template("{spinner:.cyan} {msg}").unwrap())
        .with_message(label.to_string());
    spinner.enable_steady_tick(std::time::Duration::from_millis(90));
    let output = cmd
        .output()
        .with_context(|| format!("spawning `{label}`"))?;
    if output.status.success() {
        spinner.finish_with_message(format!("{label} ✓"));
        Ok(())
    } else {
        spinner.finish_with_message(format!("{label} ✗"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let tail: String = stdout
            .lines()
            .chain(stderr.lines())
            .rev()
            .take(15)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        bail!("{label} failed:\n{tail}");
    }
}

fn check(label: &str, ok: bool) {
    println!("  [{}] {label}", if ok { "ok" } else { "!!" });
}

fn docker_ok() -> bool {
    Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
        .is_ok_and(|o| o.status.success())
}

fn task_image_ok() -> bool {
    Command::new("docker")
        .args(["image", "inspect", "wizard-ahe/task-base:latest"])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// A static executable has no PT_INTERP; `ldd` reports "not a dynamic
/// executable" (musl) or "statically linked" (glibc ldd) on it.
fn is_static(binary: &Path) -> bool {
    if !binary.is_file() {
        return false;
    }
    Command::new("ldd").arg(binary).output().is_ok_and(|o| {
        let text = String::from_utf8_lossy(&o.stdout).to_lowercase()
            + &String::from_utf8_lossy(&o.stderr).to_lowercase();
        text.contains("not a dynamic executable") || text.contains("statically linked")
    })
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(bin))
        .find(|p| p.is_file())
}