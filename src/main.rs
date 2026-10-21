mod directory;
mod rules;
mod scanner;

use anyhow::Result;
use clap::Parser;
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
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.path.is_dir() {
        anyhow::bail!(
            "The specified path is not a directory: {}",
            args.path.display()
        );
    }

    println!("Sweep - scanning {}", args.path.display());

    let artifacts = scanner::scan(&args.path);

    if artifacts.is_empty() {
        println!("No cleanup targets found.");
        return Ok(());
    }

    println!("\nFound {} directory(ies):", artifacts.len());

    for artifact in &artifacts {
        println!("  {artifact}");
    }

    if args.dry_run {
        println!("\nDry run: nothing was deleted.");
        return Ok(());
    }

    println!(
        "\nDeletion is not implemented yet. \
         Use --dry-run to inspect targets safely."
    );

    Ok(())
}
