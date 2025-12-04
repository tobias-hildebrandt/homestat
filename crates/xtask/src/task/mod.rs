use clap::Subcommand;
use xshell::Shell;

use crate::enum_call;

pub(crate) mod db;
pub(crate) mod firmware;
pub(crate) mod pico;

pub(crate) trait Runnable {
    fn run(&self, sh: &mut Shell) -> anyhow::Result<()>;
}

#[derive(Debug, Subcommand)]
pub(crate) enum Task {
    #[command(subcommand)]
    Firmware(firmware::FirmwareTask),
    #[command(subcommand)]
    Pico(pico::PicoTask),
    #[command(subcommand)]
    #[clap(alias = "db")]
    Database(db::DbTask),
}

impl Runnable for Task {
    fn run(&self, sh: &mut Shell) -> anyhow::Result<()> {
        enum_call!(Task, [Firmware, Pico, Database], self, inner, {
            inner.run(sh)
        })
    }
}
