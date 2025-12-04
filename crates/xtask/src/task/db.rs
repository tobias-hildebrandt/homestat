use clap::Subcommand;
use xshell::cmd;

use crate::task::Runnable;

/// Database tasks.
#[derive(Debug, Subcommand)]
pub(crate) enum DbTask {
    /// Delete database.
    #[clap(alias = "wipe")]
    Delete,
    /// Initialize database and run migrations.
    #[clap(alias = "create")]
    Init,
    /// Delete database then create a new one and run migrations.
    #[clap(alias = "reinit")]
    Reset,
    /// Prepare .sqlx files.
    Prepare,
}

impl Runnable for DbTask {
    fn run(&self, sh: &mut xshell::Shell) -> anyhow::Result<()> {
        match self {
            DbTask::Delete => Delete.run(sh),
            DbTask::Init => Init.run(sh),
            DbTask::Reset => Reset.run(sh),
            DbTask::Prepare => PrepareSqlx.run(sh),
        }
    }
}

struct Delete;

impl Runnable for Delete {
    fn run(&self, sh: &mut xshell::Shell) -> anyhow::Result<()> {
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

struct PrepareSqlx;

impl Runnable for PrepareSqlx {
    fn run(&self, sh: &mut xshell::Shell) -> anyhow::Result<()> {
        cmd!(sh, "env -C crates/homestat-db cargo sqlx prepare").run()?;
        Ok(())
    }
}
