//! Persistent CLI configuration and the lab's `.env` file.
//!
//! `~/.config/ahe/config.toml` records where the lab and the wizard repo
//! live, so `ahe` works from any directory. The lab's `.env` is the single
//! source of truth for the endpoint (read by evolve.py and by `ahe run`).

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AheConfig {
    /// wizard-ahe checkout (contains evolve.py, agents/, dataset/).
    pub lab_dir: PathBuf,
    /// wizard checkout used to build the agent-under-test binary.
    pub wizard_repo: PathBuf,
    /// Static musl wizard binary uploaded into task containers.
    pub wizard_binary: PathBuf,
}

pub fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir().context("no config dir")?.join("ahe");
    Ok(dir.join("config.toml"))
}

impl AheConfig {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        let raw = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "no CLI config at {} — run `ahe setup` first",
                path.display()
            )
        })?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

/// Locate the lab: explicit config first, else walk up from the current
/// directory looking for the evolve.py + agents/wizard_harness signature.
pub fn find_lab_dir() -> Option<PathBuf> {
    if let Ok(cfg) = AheConfig::load() {
        if is_lab(&cfg.lab_dir) {
            return Some(cfg.lab_dir);
        }
    }
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if is_lab(&dir) {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn is_lab(dir: &Path) -> bool {
    dir.join("evolve.py").is_file() && dir.join("agents/wizard_harness").is_dir()
}

/// Minimal .env round-trip: preserves unknown keys, updates or appends ours.
pub fn read_env(lab_dir: &Path) -> Vec<(String, String)> {
    let Ok(raw) = std::fs::read_to_string(lab_dir.join(".env")) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (k, v) = line.split_once('=')?;
            Some((k.trim().to_string(), v.trim().trim_matches('"').to_string()))
        })
        .collect()
}

pub fn env_get(env: &[(String, String)], key: &str) -> Option<String> {
    env.iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.to_string())
}

pub fn write_env(lab_dir: &Path, updates: &[(&str, &str)]) -> Result<()> {
    let path = lab_dir.join(".env");
    let mut kept: Vec<String> = Vec::new();
    if let Ok(raw) = std::fs::read_to_string(&path) {
        for line in raw.lines() {
            let updated = updates.iter().any(|(k, _)| {
                line.trim_start()
                    .strip_prefix(k)
                    .is_some_and(|rest| rest.trim_start().starts_with('='))
            });
            if !updated {
                kept.push(line.to_string());
            }
        }
    }
    for (k, v) in updates {
        kept.push(format!("{k}={v}"));
    }
    std::fs::write(&path, kept.join("\n") + "\n")?;
    Ok(())
}

/// The URL task containers use to reach the endpoint: loopback hosts are
/// rewritten to the docker bridge gateway, anything else passes through.
pub fn container_base_url(base_url: &str) -> String {
    base_url
        .replace("://localhost", "://172.17.0.1")
        .replace("://127.0.0.1", "://172.17.0.1")
}

/// Probe an OpenAI-compatible endpoint; returns the model ids it advertises.
pub fn probe_endpoint(base_url: &str, api_key: &str) -> Result<Vec<String>> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let resp = ureq::get(&url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .with_context(|| format!("cannot reach {url}"))?;
    let body: serde_json::Value = resp.into_json()?;
    let Some(data) = body.get("data").and_then(|d| d.as_array()) else {
        bail!("{url} responded but not with an OpenAI-style model list");
    };
    Ok(data
        .iter()
        .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(str::to_string))
        .collect())
}
