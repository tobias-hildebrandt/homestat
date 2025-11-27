use clap::Parser;
use xshell::Shell;

use crate::task::{Runnable, Task};

pub(crate) mod misc;
pub(crate) mod task;

/// Path to the overall cargo workspace/git repo root.
///
/// Set by /.cargo/config.toml.
const CARGO_WORKSPACE: &str = env!("CARGO_WORKSPACE_DIR");

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let sh = &mut Shell::new()?;

    cli.task.run(sh)?;

    Ok(())
}

#[derive(Debug, Parser)]
struct Cli {
    /// Task to run.
    #[command(subcommand)]
    task: Task,
    // TODO: global config? for dir, verbosity, etc?
}
