//! `ahe run`: drive evolve.py with live progress, and `ahe status`.
//!
//! Progress comes from two sources: evolve.py's stdout (iteration/phase
//! markers) and the filesystem (per-rollout `reward.txt` files appearing
//! under the experiment dir), which is robust to harbor's rich TUI output.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{bail, Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde_yaml::Value;

use crate::cfg::{self, AheConfig};

pub struct Opts {
    pub config: String,
    pub iterations: Option<u32>,
    pub tasks: Option<Vec<String>>,
    pub k: Option<u32>,
    pub target: Option<f64>,
    pub no_progress: bool,
    pub json: bool,
    pub dry_run: bool,
}

pub fn run(opts: Opts) -> Result<()> {
    let cfg = AheConfig::load()?;
    let lab = &cfg.lab_dir;
    let env = cfg::read_env(lab);
    let base_url =
        cfg::env_get(&env, "LLM_BASE_URL").context(".env has no LLM_BASE_URL — run `ahe setup`")?;
    let model =
        cfg::env_get(&env, "LLM_MODEL").context(".env has no LLM_MODEL — run `ahe setup`")?;
    if !cfg.wizard_binary.is_file() {
        bail!(
            "wizard binary missing at {} — run `ahe setup`",
            cfg.wizard_binary.display()
        );
    }

    // Resolve + overlay the experiment config.
    let config_path = resolve_config(lab, &opts.config)?;
    let mut doc: Value = serde_yaml::from_str(&std::fs::read_to_string(&config_path)?)?;
    if let Some(n) = opts.iterations {
        doc["max_iterations"] = n.into();
    }
    if let Some(tasks) = &opts.tasks {
        doc["task_names"] = Value::Sequence(tasks.iter().map(|t| t.as_str().into()).collect());
    }
    if let Some(k) = opts.k {
        doc["harbor"]["k"] = k.into();
    }
    if let Some(t) = opts.target {
        doc["target_pass_rate"] = t.into();
    }
    let effective_path = if opts.iterations.is_some()
        || opts.tasks.is_some()
        || opts.k.is_some()
        || opts.target.is_some()
    {
        // Same directory as the source config so its relative `_base` resolves.
        let overlay = config_path.parent().unwrap().join(".ahe-run.yaml");
        std::fs::write(&overlay, serde_yaml::to_string(&doc)?)?;
        overlay
    } else {
        config_path.clone()
    };

    let iterations = doc["max_iterations"].as_u64().unwrap_or(5);
    let k = doc["harbor"]["k"].as_u64().unwrap_or(1);
    let tasks_total = count_tasks(lab, &doc) * k;
    let name = doc["_name"].as_str().unwrap_or("run").to_string();

    println!(
        "run '{name}': {} task-rollouts/iter × {iterations} iteration(s) — model {model} @ {base_url}",
        tasks_total
    );
    if opts.dry_run {
        println!("\neffective config ({}):", effective_path.display());
        print!("{}", serde_yaml::to_string(&doc)?);
        return Ok(());
    }

    // Spawn evolve.py with the endpoint wired through the environment.
    let container_url = cfg::env_get(&env, "WIZARD_LLM_BASE_URL")
        .unwrap_or_else(|| cfg::container_base_url(&base_url));
    let started = SystemTime::now();
    let mut child = Command::new("uv")
        .args(["run", "--no-sync", "python", "evolve.py", "--config"])
        .arg(&effective_path)
        .current_dir(lab)
        .env("WIZARD_BINARY", &cfg.wizard_binary)
        .env("WIZARD_LLM_BASE_URL", &container_url)
        .env("WIZARD_MODEL", &model)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawning evolve.py via uv")?;

    // Forward child output lines to the render loop.
    let (tx, rx) = mpsc::channel::<String>();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    for reader in [
        Box::new(stdout) as Box<dyn std::io::Read + Send>,
        Box::new(stderr),
    ] {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(reader).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);

    let exp_dir = render_loop(
        lab,
        &name,
        started,
        iterations,
        tasks_total,
        rx,
        opts.no_progress,
    );

    let status = child.wait()?;
    let log_hint = exp_dir
        .as_deref()
        .map(|d| d.display().to_string())
        .unwrap_or_else(|| "experiments/".into());
    if !status.success() {
        bail!("evolve.py exited with {status} — inspect {log_hint}");
    }

    match exp_dir {
        Some(dir) => summarize(&dir, opts.json),
        None => {
            println!("run finished but no experiment dir was found under experiments/");
            Ok(())
        }
    }
}

/// Consume child output + poll reward files, rendering progress. Returns the
/// experiment directory once known.
fn render_loop(
    lab: &Path,
    name: &str,
    started: SystemTime,
    iterations: u64,
    tasks_total: u64,
    rx: mpsc::Receiver<String>,
    no_progress: bool,
) -> Option<PathBuf> {
    let multi = MultiProgress::new();
    let iter_bar = multi.add(
        ProgressBar::new(iterations).with_style(
            ProgressStyle::with_template(
                "{prefix:.bold} [{bar:20.cyan/blue}] iteration {pos}/{len}  {msg}",
            )
            .unwrap()
            .progress_chars("=> "),
        ),
    );
    iter_bar.set_prefix("loop ");
    let roll_bar = multi.add(
        ProgressBar::new(tasks_total).with_style(
            ProgressStyle::with_template(
                "{prefix:.bold} [{bar:20.green/blue}] {pos}/{len} rollouts  {msg}",
            )
            .unwrap()
            .progress_chars("=> "),
        ),
    );
    roll_bar.set_prefix("tasks");
    let phase = multi.add(
        ProgressBar::new_spinner()
            .with_style(ProgressStyle::with_template("{spinner:.cyan} {msg}").unwrap()),
    );
    phase.enable_steady_tick(Duration::from_millis(100));
    phase.set_message("starting…");
    if no_progress {
        multi.set_draw_target(indicatif::ProgressDrawTarget::hidden());
    }

    let mut exp_dir: Option<PathBuf> = None;
    let mut current_iter: u64 = 0;
    let mut last_scan = Instant::now() - Duration::from_secs(10);

    loop {
        // Drain available lines without blocking for long.
        let mut disconnected = false;
        loop {
            match rx.recv_timeout(Duration::from_millis(300)) {
                Ok(line) => {
                    if no_progress {
                        println!("{line}");
                    }
                    if let Some((cur, _total)) = parse_iteration(&line) {
                        current_iter = cur;
                        iter_bar.set_position(cur.saturating_sub(1));
                        roll_bar.set_position(0);
                        phase.set_message(format!("iteration {cur}: evaluating"));
                    } else if line.contains("Starting evaluation") {
                        phase.set_message(format!("iteration {current_iter}: evaluating"));
                    } else if line.contains("Analysis") || line.contains("[analysis]") {
                        phase.set_message(format!("iteration {current_iter}: analyzing traces"));
                    } else if line.contains("[evolve]") && line.contains("Starting") {
                        phase.set_message(format!("iteration {current_iter}: evolving bundle"));
                    } else if line.contains("Evolve agent completed") {
                        phase.set_message(format!("iteration {current_iter}: scoring"));
                    } else if let Some(rate) = parse_pass_rate(&line) {
                        iter_bar.set_position(current_iter);
                        iter_bar.set_message(format!("pass rate {rate}"));
                    } else if line.contains("Rolled back") || line.contains("rollback") {
                        iter_bar.set_message("regression rolled back");
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        if exp_dir.is_none() {
            exp_dir = find_experiment_dir(lab, name, started);
        }
        if last_scan.elapsed() >= Duration::from_secs(2) {
            last_scan = Instant::now();
            if let Some(dir) = &exp_dir {
                let (done, passed) = scan_rewards(dir, current_iter.max(1));
                roll_bar.set_position(done);
                roll_bar.set_message(format!("{passed} passed"));
            }
        }
        if disconnected {
            break;
        }
    }

    iter_bar.finish();
    roll_bar.finish();
    phase.finish_with_message("done");
    exp_dir
}

fn parse_iteration(line: &str) -> Option<(u64, u64)> {
    let rest = line.trim().strip_prefix("Iteration ")?;
    let (cur, total) = rest.split_once('/')?;
    Some((
        cur.trim().parse().ok()?,
        total.split_whitespace().next()?.parse().ok()?,
    ))
}

fn parse_pass_rate(line: &str) -> Option<String> {
    let idx = line.find("pass rate: ")?;
    Some(
        line[idx + "pass rate: ".len()..]
            .split_whitespace()
            .next()?
            .to_string(),
    )
}

/// Newest experiments/<ts>__<name> created after `started`.
fn find_experiment_dir(lab: &Path, name: &str, started: SystemTime) -> Option<PathBuf> {
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(lab.join("experiments")).ok()?.flatten() {
        let path = entry.path();
        if !path.is_dir()
            || !path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(&format!("__{name}")))
        {
            continue;
        }
        let created = entry.metadata().and_then(|m| m.modified()).ok()?;
        if created >= started && best.as_ref().is_none_or(|(t, _)| created > *t) {
            best = Some((created, path));
        }
    }
    best.map(|(_, p)| p)
}

/// Count reward.txt files (and how many read 1) for one iteration.
fn scan_rewards(exp_dir: &Path, iteration: u64) -> (u64, u64) {
    let root = exp_dir
        .join("runs")
        .join(format!("iteration_{iteration:03}"));
    let mut done = 0;
    let mut passed = 0;
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "reward.txt") {
                done += 1;
                if std::fs::read_to_string(&path)
                    .is_ok_and(|s| s.trim().parse::<f64>().is_ok_and(|v| v >= 1.0))
                {
                    passed += 1;
                }
            }
        }
    }
    (done, passed)
}

fn resolve_config(lab: &Path, spec: &str) -> Result<PathBuf> {
    let as_path = PathBuf::from(spec);
    if as_path.is_file() {
        return Ok(as_path.canonicalize()?);
    }
    for candidate in [
        lab.join("configs/experiments").join(spec),
        lab.join("configs/experiments").join(format!("{spec}.yaml")),
        lab.join("configs/experiments").join(format!("exp-{spec}.yaml")),
    ] {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    bail!("no experiment config named '{spec}' (see configs/experiments/)");
}

fn count_tasks(lab: &Path, doc: &Value) -> u64 {
    if let Some(names) = doc["task_names"].as_sequence() {
        if !names.is_empty() {
            return names.len() as u64;
        }
    }
    let Some(path) = doc["path"].as_str() else {
        return 1;
    };
    let root = lab.join(path.trim_start_matches("./"));
    // A dataset dir contains task dirs; a single-task dir contains task.toml.
    if root.join("task.toml").is_file() {
        return 1;
    }
    std::fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().join("task.toml").is_file())
                .count() as u64
        })
        .unwrap_or(1)
        .max(1)
}

/// Print the final iteration_scores.yaml as a table (or JSON).
fn summarize(exp_dir: &Path, json: bool) -> Result<()> {
    let scores_path = exp_dir.join("iteration_scores.yaml");
    let raw = std::fs::read_to_string(&scores_path)
        .with_context(|| format!("no scores at {}", scores_path.display()))?;
    let doc: Value = serde_yaml::from_str(&raw)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&doc)?);
        return Ok(());
    }

    println!("\nresults — {}", exp_dir.display());
    println!("  iter   pass_rate   pass/fail/exc   eval_min");
    let mut best = 0.0f64;
    if let Some(scores) = doc["scores"].as_sequence() {
        for s in scores {
            let rate = s["pass_rate"].as_f64().unwrap_or(0.0);
            best = best.max(rate);
            println!(
                "  {:>4}   {:>8.1}%   {:>4}/{}/{}          {:>5.1}",
                s["iteration"].as_u64().unwrap_or(0),
                rate * 100.0,
                s["tasks"]["pass"].as_u64().unwrap_or(0),
                s["tasks"]["fail"].as_u64().unwrap_or(0),
                s["tasks"]["exception"].as_u64().unwrap_or(0),
                s["timing"]["eval_min"].as_f64().unwrap_or(0.0),
            );
        }
    }
    println!("  best pass rate: {:.1}%", best * 100.0);
    println!(
        "  evolved bundle:  {}  (git log -p for the edit history)",
        exp_dir.join("workspace").display()
    );
    Ok(())
}

/// `ahe status`: newest experiment + whether a loop is running.
pub fn status() -> Result<()> {
    let cfg = AheConfig::load()?;
    let active = Command::new("pgrep")
        .args(["-f", "python evolve.py --config"])
        .output()
        .is_ok_and(|o| o.status.success());
    println!("loop active: {}", if active { "yes" } else { "no" });

    let mut dirs: Vec<PathBuf> = std::fs::read_dir(cfg.lab_dir.join("experiments"))
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                // Run dirs are timestamped (2026-07-02__...); skip strays.
                .filter(|p| {
                    p.is_dir()
                        && p.file_name()
                            .is_some_and(|n| n.to_string_lossy().starts_with(char::is_numeric))
                })
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    match dirs.last() {
        Some(latest) if latest.join("iteration_scores.yaml").is_file() => summarize(latest, false),
        Some(latest) => {
            println!("latest experiment (no scores yet): {}", latest.display());
            Ok(())
        }
        None => {
            println!("no experiments yet — `ahe run --config wizard-smoke`");
            Ok(())
        }
    }
}
