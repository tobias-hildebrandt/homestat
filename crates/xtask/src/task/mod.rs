use clap::Subcommand;
use xshell::Shell;

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
}

impl Runnable for Task {
    fn run(&self, sh: &mut Shell) -> anyhow::Result<()> {
        match self {
            Task::Firmware(command) => command.run(sh),
            Task::Pico(command) => command.run(sh),
        }
    }
}

/// Declares a wrapper struct with a single `args` field.
///
/// ### Arguments
/// - `$wrapper`: Identifier of the new wrapper struct
/// - `$args`: Type of the inner args field.
#[macro_export]
macro_rules! newtype_args {
    ($wrapper: ident, $args: ty) => {
        #[derive(Debug, Args)]
        pub(crate) struct $wrapper {
            #[command(flatten)]
            pub(crate) args: $args,
        }
    };
}
