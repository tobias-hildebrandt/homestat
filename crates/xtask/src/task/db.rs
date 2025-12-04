use clap::Subcommand;
use xshell::cmd;

use crate::task::Runnable;

/// Firmware tasks.
#[derive(Debug, Subcommand)]
pub(crate) enum DbTask {
    /// Delete database files
    #[clap(alias = "wipe")]
    Delete,
    /// Initialize database files
    #[clap(alias = "create")]
    Init,
    /// Delete database files, then recreate them
    #[clap(alias = "reinit")]
    Reset,
}

impl Runnable for DbTask {
    fn run(&self, sh: &mut xshell::Shell) -> anyhow::Result<()> {
        match self {
            DbTask::Delete => Delete.run(sh),
            DbTask::Init => Init.run(sh),
            DbTask::Reset => Reset.run(sh),
        }
    }
}

struct Delete;

impl Runnable for Delete {
    fn run(&self, sh: &mut xshell::Shell) -> anyhow::Result<()> {
        // wipe .sqlx dir?
        // let sqlx_dir = Utf8PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        //     .join("crates")
        //     .join("homestat-db")
        //     .join(".sqlx");

        // for entry in std::fs::read_dir(sqlx_dir)? {
        //     let entry = entry?;
        //     std::fs::remove_file(entry.path())?;
        // }

        if let Ok(database_path) = std::env::var("DATABASE_URL") {
            let database_path: &str = database_path
                .split(':')
                .nth(1)
                .expect("unable to parse path");

            let backup_path = format!("{database_path}.bak");

            cmd!(sh, "mv {database_path} {backup_path}").run()?;
        } else {
            println!("DATABASE_URL not set, not deleting")
        }

        Ok(())
    }
}

struct Init;

impl Runnable for Init {
    fn run(&self, sh: &mut xshell::Shell) -> anyhow::Result<()> {
        cmd!(sh, "env -C crates/homestat-db sqlx database create").run()?;
        cmd!(sh, "env -C crates/homestat-db sqlx migrate run").run()?;
        cmd!(sh, "env -C crates/homestat-db cargo sqlx prepare").run()?;

        Ok(())
    }
}

struct Reset;

impl Runnable for Reset {
    fn run(&self, sh: &mut xshell::Shell) -> anyhow::Result<()> {
        Delete.run(sh)?;
        Init.run(sh)?;
        Ok(())
    }
}
