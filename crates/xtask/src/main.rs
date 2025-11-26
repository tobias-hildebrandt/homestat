use std::{fs::File, os::unix::fs::MetadataExt};

use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, Parser, Subcommand};
use homestat_build::Cyw43439Regions;
use xshell::{Shell, cmd};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut sh = Shell::new()?;
    match cli.command {
        Command::DownloadFirmware(firmware) => {
            firmware.download(&mut sh)?;
        }
        Command::CleanFirmwareFiles(firmware) => {
            firmware.delete()?;
        }
        Command::FlashFirmware(firmware) => {
            firmware.flash(&mut sh)?;
        }
        Command::BuildPico => {
            cmd!(sh, "env -C cross cargo build --release").run()?;
        }
        Command::UploadPico => {
            cmd!(sh, "env -C cross cargo run --release").run()?;
        }
    }

    Ok(())
}

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Path to the overall cargo workspace/git repo root.
///
/// Set by /.cargo/config.toml.
const CARGO_WORKSPACE: &str = env!("CARGO_WORKSPACE_DIR");

#[derive(Debug, Subcommand)]
enum Command {
    DownloadFirmware(Firmware),
    FlashFirmware(Firmware),
    CleanFirmwareFiles(Firmware),
    BuildPico,
    UploadPico,
}

#[derive(Debug, Args)]
struct Firmware {
    #[arg(short, long, default_value_t = Self::default_dir())]
    dir: Utf8PathBuf,
}

impl Firmware {
    const BASE_URL: &str = "https://raw.githubusercontent.com/Infineon/wifi-host-driver/refs/tags/latest-v3.X/WiFi_Host_Driver/resources/";
    const GH_URL: &str =
        "https://github.com/Infineon/wifi-host-driver/tree/latest-v3.X/WiFi_Host_Driver/resources";

    const LICENSE: &str = "LICENSE-permissive-binary-license-1.0.txt";

    const MAIN_BLOB_DL: &str = "firmware/COMPONENT_43439/43439a0.bin";
    const CLM_BLOB_DL: &str = "clm/COMPONENT_43439/43439A0.clm_blob";

    const MAIN_BLOB_NAME: &str = "43439A0.bin";
    const CLM_BLOB_NAME: &str = "43439A0_clm.bin";

    fn default_dir() -> Utf8PathBuf {
        let mut path = Utf8PathBuf::from(CARGO_WORKSPACE);

        path.push("misc");
        path.push("firmware");

        path
    }

    fn download(&self, sh: &mut Shell) -> anyhow::Result<FirmwarePaths> {
        println!("downloading from {}", Self::GH_URL);

        let license_url = format!("{}{}", Self::BASE_URL, Self::LICENSE);
        let license_out = strip_cargo_dir(self.dir.join(Self::LICENSE));
        download_file(sh, license_url, &license_out)?;

        let main_url = format!("{}{}", Self::BASE_URL, Self::MAIN_BLOB_DL);
        let main_out = strip_cargo_dir(self.dir.join(Self::MAIN_BLOB_NAME));
        download_file(sh, main_url, &main_out)?;

        let clm_url = format!("{}{}", Self::BASE_URL, Self::CLM_BLOB_DL);
        let clm_out = strip_cargo_dir(self.dir.join(Self::CLM_BLOB_NAME));
        download_file(sh, clm_url, &clm_out)?;

        Ok(FirmwarePaths {
            main_blob: main_out,
            clm_blob: clm_out,
        })
    }

    fn delete(&self) -> anyhow::Result<()> {
        let dir = &self.dir;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            std::fs::remove_file(entry.path())?;
        }

        Ok(())
    }

    fn flash(&self, sh: &mut Shell) -> anyhow::Result<()> {
        let main_path = strip_cargo_dir(self.dir.join(Self::MAIN_BLOB_NAME));
        let clm_path = strip_cargo_dir(self.dir.join(Self::CLM_BLOB_NAME));

        let main_size = std::fs::metadata(&main_path)?.size().try_into().unwrap();
        let clm_size = std::fs::metadata(&clm_path)?.size().try_into().unwrap();

        let cyw = Cyw43439Regions::at_end(main_size, clm_size);

        let main_address = format!("0x{:x}", cyw.main.origin);
        let clm_address = format!("0x{:x}", cyw.clm.origin);

        cmd!(
            sh,
            "picotool load --update {main_path} -t bin --offset {main_address}"
        )
        .run()?;

        cmd!(
            sh,
            "picotool load --update {clm_path} -t bin --offset {clm_address}"
        )
        .run()?;

        let flash_metadata_path = self.dir.join(Cyw43439Regions::JSON_FILENAME);
        let mut flash_metadata_file = File::create(&flash_metadata_path).unwrap();

        println!(
            "flashed firmware, wrote metadata to {}",
            flash_metadata_path
                .strip_prefix(CARGO_WORKSPACE)
                .unwrap_or(&flash_metadata_path)
        );

        cyw.write_json(&mut flash_metadata_file)?;

        Ok(())
    }
}

#[derive(Debug, Args)]
struct FirmwarePaths {
    main_blob: Utf8PathBuf,
    clm_blob: Utf8PathBuf,
}

/// Downloads a single file to the specified path using system's curl.
fn download_file(
    sh: &mut Shell,
    url: impl AsRef<str>,
    out: impl AsRef<Utf8Path>,
) -> anyhow::Result<()> {
    let url = url.as_ref();
    let out = out.as_ref();

    cmd!(sh, "curl -Ss {url} -o {out}").quiet().run()?;

    println!("downloaded {}", out);

    Ok(())
}

/// Strips [`CARGO_WORKSPACE`] from the path if possible.
fn strip_cargo_dir(path: impl AsRef<Utf8Path>) -> Utf8PathBuf {
    path.as_ref()
        .strip_prefix(CARGO_WORKSPACE)
        .unwrap_or(path.as_ref())
        .to_path_buf()
}
