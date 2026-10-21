mod artifact;
mod cleaner;
mod directory;
mod path_list;
mod report;
mod rules;
mod safety;
mod scanner;
mod size;

use anyhow::Result;
use clap::Parser;
use path_list::PathList;
use report::Report;
use safety::SafetyGuard;
use size::Size;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "sweep",
    version,
    about = "A lightweight cross-platform project cleanup CLI"
)]
struct Args {
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
}

fn main() -> Result<()> {
    let args = Args::parse();

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
    let report = Report::new(scanner::scan(&args.path, &exclusions));

    if report.is_empty() {
        println!("No cleanup targets found.");
        return Ok(());
    }

    println!("\n{report}");

    if args.dry_run {
        println!("\nDry run: nothing was deleted.");
        return Ok(());
    }

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

/// Asks the user to type `yes`. Any other answer, or no answer at all, cancels.
fn confirm_deletion(space: Size) -> io::Result<bool> {
    print!("\nDelete these directories and free {space}? Type 'yes' to confirm: ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;

    Ok(answer.trim().eq_ignore_ascii_case("yes"))
}
