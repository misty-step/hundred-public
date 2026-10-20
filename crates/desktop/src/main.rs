mod ui;

use anyhow::{Context, Result, ensure};
use chrono::Local;
use clap::{Parser, Subcommand};
use hundred_client::{DEFAULT_SERVER, Store, default_data_dir};
use hundred_core::{Archive, Exercise, progress};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
};

#[derive(Parser)]
#[command(
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("HUNDRED_REVISION"), ")"),
    about = "A quiet daily rep tracker. Run without a command to open Hundred."
)]
struct Arguments {
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    server: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Log a set durably. Sync separately or open the app for automatic sync.
    Add {
        exercise: Exercise,
        reps: u16,
        #[arg(long)]
        at: Option<String>,
    },
    /// Cancel a set, or the latest active set when no ID is given.
    Undo { id: Option<String> },
    /// Show today's progress and pending count.
    Status,
    /// Print the versioned event archive to stdout.
    History,
    /// Synchronize pending sets and pull the complete cloud log.
    Sync,
    /// Write a new private archive file; never overwrite an existing file.
    Export { path: PathBuf },
    /// Merge an archive without replacing existing records.
    Restore { path: PathBuf },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let arguments = Arguments::parse();
    let directory = arguments.data_dir.unwrap_or_else(default_data_dir);
    let server = arguments
        .server
        .or_else(|| std::env::var("HUNDRED_SERVER").ok())
        .unwrap_or_else(|| DEFAULT_SERVER.into());
    let token = std::env::var("HUNDRED_TOKEN").unwrap_or_default();
    let Some(command) = arguments.command else {
        return ui::run(directory, server, token);
    };
    let mut store = Store::open(&directory)?;
    match command {
        Command::Add { exercise, reps, at } => {
            let event = store.add(exercise, reps, at)?;
            println!(
                "Saved {} {} · {}",
                reps,
                exercise.label().to_lowercase(),
                event.id
            );
        }
        Command::Undo { id } => {
            store.undo(id.as_deref())?;
            println!("Set undone. Saved here.");
        }
        Command::Status => {
            let today = Local::now().date_naive();
            let progress = progress(&store.events()?, today);
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "day": today, "totals": progress.days.get(&today).copied().unwrap_or_default(),
                    "current_streak":progress.current_streak, "best_streak":progress.best_streak,
                    "complete_days":progress.complete_days, "pending":store.pending()?
                }))?
            );
        }
        Command::History => println!("{}", serde_json::to_string_pretty(&store.archive()?)?),
        Command::Sync => {
            hundred_client::sync(&mut store, &server, &token)?;
            println!("Synced.");
        }
        Command::Export { path } => {
            let bytes = serde_json::to_vec_pretty(&store.archive()?)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .context("Choose a new archive filename")?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            println!("Exported {} records.", store.events()?.len());
        }
        Command::Restore { path } => {
            let mut bytes = Vec::new();
            std::fs::File::open(path)?
                .take(64_000_001)
                .read_to_end(&mut bytes)?;
            ensure!(bytes.len() <= 64_000_000, "Archive exceeds 64 MB");
            let archive: Archive = serde_json::from_slice(&bytes)?;
            store.restore(&archive)?;
            println!(
                "Restored {} records. Sync to reconcile with the cloud.",
                archive.events.len()
            );
        }
    }
    Ok(())
}
