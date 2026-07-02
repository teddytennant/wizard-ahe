//! `ahe` — CLI for the wizard-ahe harness-evolution lab.
//!
//! `ahe setup` wires everything once (endpoint, wizard binary, task image,
//! python env); `ahe run` drives an evolution run with live progress;
//! `ahe status` shows the latest experiment.

mod cfg;
mod run;
mod setup;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ahe", version, about = "wizard-ahe: evolve wizard's harness bundle")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Interactive one-time setup: LLM endpoint, wizard install (static musl
    /// build), Nix task image, python env. Re-run any time; it's idempotent.
    Setup {
        /// OpenAI-compatible base URL (e.g. http://localhost:8088/v1). Asked
        /// interactively when omitted.
        #[arg(long)]
        base_url: Option<String>,
        /// Model name at that endpoint. Asked interactively when omitted.
        #[arg(long)]
        model: Option<String>,
        /// API key (any non-empty string for keyless local servers). Asked
        /// interactively when omitted.
        #[arg(long)]
        api_key: Option<String>,
        /// Existing wizard repo checkout to use instead of cloning.
        #[arg(long)]
        wizard_repo: Option<std::path::PathBuf>,
        /// Skip the long build steps (wizard binary, task image, uv sync).
        #[arg(long)]
        skip_build: bool,
    },

    /// Run the evolution loop with live progress bars.
    Run {
        /// Experiment: a name (wizard, wizard-smoke, wizard-slice) or a path
        /// to a config yaml.
        #[arg(long, default_value = "wizard")]
        config: String,
        /// Override max_iterations (how many evaluate→analyze→improve rounds).
        #[arg(long, short = 'i')]
        iterations: Option<u32>,
        /// Override the task subset, comma-separated (e.g. even-sum,csv-report).
        #[arg(long, value_delimiter = ',')]
        tasks: Option<Vec<String>>,
        /// Override rollouts per task per iteration (harbor k).
        #[arg(long, short = 'k')]
        k: Option<u32>,
        /// Override target pass rate in [0,1]; the loop stops early when reached.
        #[arg(long)]
        target: Option<f64>,
        /// Plain line output instead of progress bars (for logs/CI).
        #[arg(long)]
        no_progress: bool,
        /// Print the final scores as JSON instead of a table.
        #[arg(long)]
        json: bool,
        /// Show the effective config and totals, then exit without running.
        #[arg(long)]
        dry_run: bool,
    },

    /// Show the latest experiment's scores and whether a run is active.
    Status,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Setup {
            base_url,
            model,
            api_key,
            wizard_repo,
            skip_build,
        } => setup::run(setup::Opts {
            base_url,
            model,
            api_key,
            wizard_repo,
            skip_build,
        }),
        Command::Run {
            config,
            iterations,
            tasks,
            k,
            target,
            no_progress,
            json,
            dry_run,
        } => run::run(run::Opts {
            config,
            iterations,
            tasks,
            k,
            target,
            no_progress,
            json,
            dry_run,
        }),
        Command::Status => run::status(),
    }
}
