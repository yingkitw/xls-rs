use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct CliRuntime {
    pub config_path: Option<PathBuf>,
    pub quiet: bool,
    pub verbose: bool,
    pub overwrite: bool,
}

static RUNTIME: OnceLock<CliRuntime> = OnceLock::new();

pub fn init(runtime: CliRuntime) {
    let _ = RUNTIME.set(runtime);
}

pub fn get() -> &'static CliRuntime {
    RUNTIME.get().expect("CLI runtime not initialized")
}

pub fn log(msg: impl AsRef<str>) {
    if !get().quiet {
        eprintln!("{}", msg.as_ref());
    }
}

pub fn debug(msg: impl AsRef<str>) {
    if get().verbose && !get().quiet {
        eprintln!("{}", msg.as_ref());
    }
}

pub fn ensure_can_write(path: &str) -> anyhow::Result<()> {
    if path == "-" {
        return Ok(());
    }
    let p = Path::new(path);
    if p.exists() && !get().overwrite {
        anyhow::bail!(
            "Refusing to overwrite '{}'. Pass --overwrite to allow.",
            p.display()
        );
    }
    Ok(())
}

