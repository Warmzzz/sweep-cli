use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use walkdir::WalkDir;

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

const TARGETS: &[&str] = &[
    "node_modules",
    "bin",
    "obj",
    "__pycache__",
    ".venv",
    "target",
];

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.path.is_dir() {
        anyhow::bail!(
            "The specified path is not a directory: {}",
            args.path.display()
        );
    }

    println!("Sweep - scanning {}", args.path.display());

    let mut found = Vec::new();

    for entry in WalkDir::new(&args.path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.file_name() != ".git"
        })
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!("Warning: {error}");
                continue;
            }
        };

        if entry.file_type().is_dir()
            && entry.depth() > 0
            && entry.file_name()
            .to_str()
            .is_some_and(|name| TARGETS.contains(&name))
        {
            found.push(entry.path().to_path_buf());
        }
    }

    if found.is_empty() {
        println!("No cleanup targets found.");
        return Ok(());
    }

    println!("\nFound {} directory(ies):", found.len());

    for path in &found {
        println!("  {}", path.display());
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
