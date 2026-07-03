//! Interactive LLM endpoint wiring for `ahe setup`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};

use crate::cfg;

pub struct EndpointConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub models: Vec<String>,
}

const OPENAI_API_KEY: &str = "oauth-via-proxy";
const CHATGPT_PROXY_PORT: u16 = 8089;
const XAI_PROXY_PORT: u16 = 8088;
const CHATGPT_PROXY_SCRIPT: &str = "chatgpt-oauth-proxy.py";
const XAI_PROXY_SCRIPT: &str = "xai-oauth-proxy.py";
const DEFAULT_CHATGPT_MODEL: &str = "gpt-5.2";
const DEFAULT_XAI_MODEL: &str = "grok-4.3";

pub fn collect(
    theme: &ColorfulTheme,
    lab_dir: &Path,
    env: &[(String, String)],
    explicit_base_url: Option<String>,
    explicit_api_key: Option<String>,
    explicit_model: Option<String>,
    wizard_binary: Option<&Path>,
) -> Result<EndpointConfig> {
    if explicit_base_url.is_some() || explicit_api_key.is_some() {
        return collect_openai_api(theme, env, explicit_base_url, explicit_api_key, explicit_model);
    }

    let backend = Select::with_theme(theme)
        .with_prompt("LLM backend")
        .items(&[
            "OpenAI-compatible API (URL + API key)",
            "ChatGPT OAuth (ChatGPT/Codex subscription via `codex login`)",
            "xAI OAuth (Grok subscription via `wizard --login xai`)",
        ])
        .default(0)
        .interact()?;

    match backend {
        0 => collect_openai_api(theme, env, None, None, explicit_model),
        1 => collect_chatgpt_oauth(theme, lab_dir, env, explicit_model),
        2 => collect_xai_oauth(theme, lab_dir, env, wizard_binary, explicit_model),
        _ => unreachable!(),
    }
}

fn collect_openai_api(
    theme: &ColorfulTheme,
    env: &[(String, String)],
    base_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
) -> Result<EndpointConfig> {
    let base_url: String = match base_url {
        Some(v) => v,
        None => Input::with_theme(theme)
            .with_prompt("OpenAI-compatible base URL (e.g. https://api.openai.com/v1)")
            .default(cfg::env_get(env, "LLM_BASE_URL").unwrap_or_default())
            .interact_text()?,
    };
    let base_url = base_url.trim_end_matches('/').to_string();

    let api_key: String = match api_key {
        Some(v) => v,
        None => Password::with_theme(theme)
            .with_prompt("API key (any non-empty string for keyless local servers)")
            .with_confirmation("confirm API key", "keys differ")
            .interact()?,
    };

    let models = probe_or_confirm(theme, &base_url, &api_key)?;
    let model = pick_model(theme, env, &models, model, DEFAULT_CHATGPT_MODEL)?;
    Ok(EndpointConfig {
        base_url,
        api_key,
        model,
        models,
    })
}

fn collect_chatgpt_oauth(
    theme: &ColorfulTheme,
    lab_dir: &Path,
    env: &[(String, String)],
    model: Option<String>,
) -> Result<EndpointConfig> {
    println!(
        "ChatGPT OAuth uses your ChatGPT/Codex subscription — no API key.\n\
         Run `codex login` in the browser if you have not signed in yet."
    );
    ensure_chatgpt_session(theme, lab_dir)?;
    let base_url = ensure_oauth_proxy(lab_dir, CHATGPT_PROXY_SCRIPT, CHATGPT_PROXY_PORT)?;
    let api_key = OPENAI_API_KEY.to_string();
    let models = probe_or_confirm(theme, &base_url, &api_key)?;
    let model = pick_model(
        theme,
        env,
        &models,
        model,
        DEFAULT_CHATGPT_MODEL,
    )?;
    Ok(EndpointConfig {
        base_url,
        api_key,
        model,
        models,
    })
}

fn collect_xai_oauth(
    theme: &ColorfulTheme,
    lab_dir: &Path,
    env: &[(String, String)],
    wizard_binary: Option<&Path>,
    model: Option<String>,
) -> Result<EndpointConfig> {
    println!(
        "xAI OAuth uses your Grok subscription — no API key.\n\
         Run `wizard --login xai` in the browser if you have not signed in yet."
    );
    ensure_xai_session(theme, wizard_binary)?;
    let base_url = ensure_oauth_proxy(lab_dir, XAI_PROXY_SCRIPT, XAI_PROXY_PORT)?;
    let api_key = OPENAI_API_KEY.to_string();
    let models = probe_or_confirm(theme, &base_url, &api_key)?;
    let model = pick_model(theme, env, &models, model, DEFAULT_XAI_MODEL)?;
    Ok(EndpointConfig {
        base_url,
        api_key,
        model,
        models,
    })
}

fn pick_model(
    theme: &ColorfulTheme,
    env: &[(String, String)],
    models: &[String],
    explicit: Option<String>,
    default: &str,
) -> Result<String> {
    let model: String = match explicit {
        Some(v) => v,
        None => Input::with_theme(theme)
            .with_prompt("model name")
            .default(
                cfg::env_get(env, "LLM_MODEL")
                    .or_else(|| models.first().cloned())
                    .unwrap_or_else(|| default.to_string()),
            )
            .interact_text()?,
    };
    if !models.is_empty() && !models.iter().any(|m| m == &model) {
        println!("warning: '{model}' is not in the endpoint's model list");
    }
    Ok(model)
}

fn probe_or_confirm(theme: &ColorfulTheme, base_url: &str, api_key: &str) -> Result<Vec<String>> {
    match cfg::probe_endpoint(base_url, api_key) {
        Ok(models) => {
            println!(
                "endpoint OK — {} model(s): {}",
                models.len(),
                models
                    .iter()
                    .take(6)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            Ok(models)
        }
        Err(err) => {
            println!("endpoint probe failed: {err:#}");
            if !Confirm::with_theme(theme)
                .with_prompt("continue anyway?")
                .default(false)
                .interact()?
            {
                bail!("aborted: endpoint unreachable");
            }
            Ok(Vec::new())
        }
    }
}

fn ensure_chatgpt_session(theme: &ColorfulTheme, lab_dir: &Path) -> Result<()> {
    if chatgpt_session_present() {
        println!("ChatGPT OAuth session found");
        return Ok(());
    }
    if which("codex").is_none() {
        bail!(
            "no ChatGPT OAuth session and `codex` is not installed.\n\
             Install Codex CLI (https://developers.openai.com/codex/cli/) and run `codex login`,\n\
             or pick the OpenAI-compatible API backend instead."
        );
    }
    println!("no ChatGPT session — starting browser login via `codex login`");
    let status = Command::new("codex")
        .arg("login")
        .current_dir(lab_dir)
        .status()
        .context("spawning `codex login`")?;
    if !status.success() {
        bail!("`codex login` failed");
    }
    if !chatgpt_session_present() {
        if !Confirm::with_theme(theme)
            .with_prompt("still no session file — continue anyway?")
            .default(false)
            .interact()?
        {
            bail!("aborted: ChatGPT OAuth session missing");
        }
    }
    Ok(())
}

fn ensure_xai_session(theme: &ColorfulTheme, wizard_binary: Option<&Path>) -> Result<()> {
    if xai_session_present() {
        println!("xAI OAuth session found");
        return Ok(());
    }
    let wizard = resolve_wizard_binary(wizard_binary)?;
    println!("no xAI session — starting browser login via `wizard --login xai`");
    let status = Command::new(&wizard)
        .args(["--login", "xai"])
        .status()
        .with_context(|| format!("spawning `{} --login xai`", wizard.display()))?;
    if !status.success() {
        bail!("`wizard --login xai` failed");
    }
    if !xai_session_present() {
        if !Confirm::with_theme(theme)
            .with_prompt("still no session file — continue anyway?")
            .default(false)
            .interact()?
        {
            bail!("aborted: xAI OAuth session missing");
        }
    }
    Ok(())
}

fn resolve_wizard_binary(wizard_binary: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = wizard_binary.filter(|p| p.is_file()) {
        return Ok(path.to_path_buf());
    }
    if let Ok(cfg) = cfg::AheConfig::load() {
        if cfg.wizard_binary.is_file() {
            return Ok(cfg.wizard_binary);
        }
        let host = cfg
            .wizard_repo
            .join("target/release/wizard");
        if host.is_file() {
            return Ok(host);
        }
        let musl = cfg.wizard_binary;
        if musl.is_file() {
            return Ok(musl);
        }
    }
    if let Some(path) = which("wizard") {
        return Ok(path);
    }
    bail!(
        "wizard binary not found for `wizard --login xai` — re-run `ahe setup` \
         after the wizard build step, or pass --wizard-repo"
    );
}

pub fn ensure_oauth_proxy(lab_dir: &Path, script: &str, port: u16) -> Result<String> {
    let base_url = format!("http://127.0.0.1:{port}/v1");
    if cfg::probe_endpoint(&base_url, OPENAI_API_KEY).is_ok() {
        println!("OAuth proxy already reachable at {base_url}");
        return Ok(base_url);
    }

    let script_path = lab_dir.join("scripts").join(script);
    if !script_path.is_file() {
        bail!("missing proxy script: {}", script_path.display());
    }

    println!("starting OAuth proxy ({script}) on port {port}");
    Command::new("python3")
        .arg(&script_path)
        .env("PORT", port.to_string())
        .env("HOST", "0.0.0.0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("starting {}", script_path.display()))?;

    for attempt in 1..=30 {
        thread::sleep(Duration::from_millis(500));
        if cfg::probe_endpoint(&base_url, OPENAI_API_KEY).is_ok() {
            println!("OAuth proxy ready at {base_url}");
            return Ok(base_url);
        }
        if attempt == 30 {
            bail!(
                "OAuth proxy on port {port} did not become reachable — \
                 check that login succeeded and try `python3 scripts/{script}` manually"
            );
        }
    }
    unreachable!()
}

fn chatgpt_session_present() -> bool {
    let home = dirs::home_dir().unwrap_or_default();
    [home.join(".wizard/chatgpt_oauth.json"), home.join(".codex/auth.json")]
        .iter()
        .any(|path| session_has_access_token(path))
}

fn xai_session_present() -> bool {
    let home = dirs::home_dir().unwrap_or_default();
    session_has_access_token(&home.join(".wizard/xai_oauth.json"))
}

fn session_has_access_token(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    doc.get("access_token")
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.is_empty())
        || doc
            .get("tokens")
            .and_then(|t| t.get("access_token"))
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty())
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(bin))
        .find(|p| p.is_file())
}