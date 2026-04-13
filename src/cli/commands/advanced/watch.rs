//! Watch mode - re-run command on file change

use anyhow::{Context, Result};
use notify::Watcher;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Handle the watch command
///
/// Watches the input file and re-runs the command on change.
pub fn handle_watch(input: String, command: String) -> Result<()> {
    #[cfg(feature = "watch")]
    {
        let path = std::path::Path::new(&input);
        if !path.exists() {
            anyhow::bail!("Input file does not exist: {}", input);
        }

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        })
        .context("Failed to set Ctrl-C handler")?;

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, notify::EventKind::Modify(_)) {
                    let _ = tx.send(());
                }
            }
        })
        .context("Failed to create file watcher")?;

        watcher
            .watch(path, notify::RecursiveMode::NonRecursive)
            .context("Failed to watch file")?;

        eprintln!("Watching {} (Ctrl-C to stop)", input);

        // Run once immediately
        run_command(&command, &input)?;

        while running.load(Ordering::SeqCst) {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(()) => {
                    eprintln!("\n--- File changed, re-running ---");
                    run_command(&command, &input)?;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        eprintln!("Stopped watching");
        Ok(())
    }

    #[cfg(not(feature = "watch"))]
    {
        let _ = (input, command);
        anyhow::bail!(
            "Watch mode requires the 'watch' feature. Build with: cargo build --features watch"
        )
    }
}

fn run_command(cmd: &str, input: &str) -> Result<()> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .env("XLS_RS_WATCH_INPUT", input)
        .status()
        .context("Failed to run command")?;

    if !status.success() {
        if let Some(code) = status.code() {
            std::process::exit(code);
        }
    }
    Ok(())
}
