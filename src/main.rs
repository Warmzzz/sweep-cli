mod artifact;
mod cleaner;
mod config;
mod directory;
mod path_list;
mod report;
mod rules;
mod safety;
mod scanner;
mod size;
mod updater;

use anyhow::Result;
use artifact::Artifact;
use clap::{Parser, Subcommand};
use config::Config;
use dialoguer::MultiSelect;
use path_list::PathList;
use report::Report;
use rules::Rule;
use safety::SafetyGuard;
use size::Size;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "sweep",
    version,
    about = "A lightweight cross-platform project cleanup CLI",
    args_conflicts_with_subcommands = true
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Project directory to scan
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Show what would be removed without deleting anything
    #[arg(long)]
    dry_run: bool,

    /// Directory to leave untouched, with everything inside it (can be repeated)
    #[arg(long, value_name = "PATH")]
    exclude: Vec<PathBuf>,

    /// Delete without asking for confirmation
    #[arg(short, long)]
    yes: bool,

    /// Config file with your own rules [default: ~/.config/sweep/config.toml]
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Choose the directories to delete from a checkbox list
    #[arg(short, long)]
    interactive: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Download and install the latest version of sweep
    Update,
}

fn main() -> Result<()> {
    let mut args = Args::parse();

    if let Some(Command::Update) = args.command {
        return updater::update();
    }

    let config = load_config(args.config.take())?;

    // Scripts (`--yes`) do not need a notice; anyone can turn it off in the config file.
    let update_notice = match config::default_path() {
        Some(config_path) if config.update_check && !args.yes => Some(
            updater::check_in_background(config_path.with_file_name("last-update-check")),
        ),
        _ => None,
    };

    run(args, &config.rules)?;

    if let Some(receiver) = update_notice {
        updater::print_notice(receiver);
    }
    Ok(())
}

fn run(args: Args, all_rules: &[Rule]) -> Result<()> {
    if !args.path.is_dir() {
        anyhow::bail!(
            "The specified path is not a directory: {}",
            args.path.display()
        );
    }

    // A typo in an exclusion would silently delete what the user wanted to keep.
    if let Some(missing) = args.exclude.iter().find(|path| !path.exists()) {
        anyhow::bail!("The excluded path does not exist: {}", missing.display());
    }

    println!("Sweep - scanning {}", args.path.display());

    let exclusions = PathList::new(args.exclude);
    let report = Report::new(scanner::scan(&args.path, all_rules, &exclusions));

    if report.is_empty() {
        println!("No cleanup targets found.");
        return Ok(());
    }

    println!("\n{report}");

    if args.dry_run {
        println!("\nDry run: nothing was deleted.");
        return Ok(());
    }

    let report = if args.interactive {
        match select_artifacts(report.into_artifacts())? {
            Some(selected) if !selected.is_empty() => Report::new(selected),
            _ => {
                println!("Nothing selected: nothing was deleted.");
                return Ok(());
            }
        }
    } else {
        report
    };

    if !args.yes && !confirm_deletion(report.total_size())? {
        println!("Cancelled: nothing was deleted.");
        return Ok(());
    }

    println!();
    let guard = SafetyGuard::new(&args.path);
    let summary = cleaner::delete_all(report.artifacts(), &guard);
    println!("\n{summary}");

    Ok(())
}

/// The user's rules come first: when two rules claim the same folder,
/// the project name shown is the user's.
fn load_config(config_path: Option<PathBuf>) -> Result<Config> {
    let mut config = Config::default();

    if let Some(path) = config::find(config_path) {
        config = config::load(&path)?;
        println!(
            "Loaded {} custom rule(s) from {}",
            config.rules.len(),
            path.display()
        );
    }

    config.rules.extend(rules::built_in());
    Ok(config)
}

/// Shows a checkbox list: Space toggles a directory, Enter confirms.
/// Returns `None` when the user leaves with Esc or q.
fn select_artifacts(artifacts: Vec<Artifact>) -> Result<Option<Vec<Artifact>>> {
    let checked = MultiSelect::new()
        .with_prompt("Select the directories to delete (Space: toggle, a: all, Enter: confirm)")
        .items(&artifacts)
        .defaults(&vec![true; artifacts.len()])
        .interact_opt()?;

    Ok(checked.map(|indexes| {
        artifacts
            .into_iter()
            .enumerate()
            .filter(|(index, _)| indexes.contains(index))
            .map(|(_, artifact)| artifact)
            .collect()
    }))
}

/// Asks the user to type `yes`. Any other answer, or no answer at all, cancels.
fn confirm_deletion(space: Size) -> io::Result<bool> {
    print!("\nDelete these directories and free {space}? Type 'yes' to confirm: ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;

    Ok(answer.trim().eq_ignore_ascii_case("yes"))
}
