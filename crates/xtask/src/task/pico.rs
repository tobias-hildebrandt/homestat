use clap::Subcommand;
use xshell::{Shell, cmd};

use crate::task::Runnable;

/// Pico W tasks.
#[derive(Debug, Subcommand)]
pub(crate) enum PicoTask {
    /// Build Pico W flash image.
    Build,
    /// Build and upload Pico W flash image.
    Upload,
}

impl Runnable for PicoTask {
    fn run(&self, sh: &mut Shell) -> anyhow::Result<()> {
        match self {
            PicoTask::Build => {
                cmd!(sh, "env -C cross cargo build --release").run()?;
            }
            PicoTask::Upload => {
                cmd!(sh, "env -C cross cargo run --release").run()?;
            }
        }

        Ok(())
    }
}
