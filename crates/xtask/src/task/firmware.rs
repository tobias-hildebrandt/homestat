use std::{fs::File, os::unix::fs::MetadataExt};

use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, Subcommand};
use homestat_build::Cyw43439Regions;
use xshell::{Shell, cmd};

use crate::{
    CARGO_WORKSPACE,
    misc::{download_file, strip_cargo_dir},
    newtype_args,
    task::Runnable,
};

const BASE_URL: &str = "https://raw.githubusercontent.com/Infineon/wifi-host-driver/refs/tags/latest-v3.X/WiFi_Host_Driver/resources/";
const GH_URL: &str =
    "https://github.com/Infineon/wifi-host-driver/tree/latest-v3.X/WiFi_Host_Driver/resources";

const LICENSE: &str = "LICENSE-permissive-binary-license-1.0.txt";

const MAIN_BLOB_DL: &str = "firmware/COMPONENT_43439/43439a0.bin";
const CLM_BLOB_DL: &str = "clm/COMPONENT_43439/43439A0.clm_blob";

const MAIN_BLOB_NAME: &str = "43439A0.bin";
const CLM_BLOB_NAME: &str = "43439A0_clm.bin";

/// Firmware tasks.
#[derive(Debug, Subcommand)]
pub(crate) enum FirmwareTask {
    /// Download firmware files.
    Download(Download),
    /// Flash firmware to connected Pico W.
    Flash(Flash),
    /// Check status of firmware files and flash information.
    Status(Status),
    /// Clean downloaded firmware files.
    Clean(Clean),
}

impl Runnable for FirmwareTask {
    fn run(&self, sh: &mut Shell) -> anyhow::Result<()> {
        match self {
            FirmwareTask::Download(download) => download.run(sh),
            FirmwareTask::Flash(flash) => flash.run(sh),
            FirmwareTask::Status(status) => status.run(sh),
            FirmwareTask::Clean(clean) => clean.run(sh),
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct FirmwareArgs {
    #[arg(short, long, default_value_t = Self::default_dir())]
    pub(crate) dir: Utf8PathBuf,
}

impl FirmwareArgs {
    fn default_dir() -> Utf8PathBuf {
        let mut path = Utf8PathBuf::from(CARGO_WORKSPACE);

        path.push("misc");
        path.push("firmware");

        path
    }

    fn firmware_path(&self, path: impl AsRef<Utf8Path>) -> Utf8PathBuf {
        strip_cargo_dir(self.dir.join(path.as_ref()))
    }
}

fn get_download_url(file_url: impl AsRef<str>) -> String {
    format!("{}{}", BASE_URL, file_url.as_ref())
}

// declare wrapper newtypes
newtype_args!(Download, FirmwareArgs);
newtype_args!(Flash, FirmwareArgs);
newtype_args!(Status, FirmwareArgs);
newtype_args!(Clean, FirmwareArgs);

impl Runnable for Download {
    fn run(&self, sh: &mut Shell) -> anyhow::Result<()> {
        println!("downloading from {}", GH_URL);

        let license_url = get_download_url(LICENSE);
        let license_out = self.args.firmware_path(LICENSE);
        download_file(sh, license_url, &license_out)?;

        let main_url = get_download_url(MAIN_BLOB_DL);
        let main_out = self.args.firmware_path(MAIN_BLOB_NAME);
        download_file(sh, main_url, &main_out)?;

        let clm_url = get_download_url(CLM_BLOB_DL);
        let clm_out = self.args.firmware_path(CLM_BLOB_NAME);
        download_file(sh, clm_url, &clm_out)?;

        Ok(())
    }
}

impl Runnable for Flash {
    fn run(&self, sh: &mut Shell) -> anyhow::Result<()> {
        let main_path = self.args.firmware_path(MAIN_BLOB_NAME);
        let clm_path = self.args.firmware_path(CLM_BLOB_NAME);

        let main_size = std::fs::metadata(&main_path)?.size().try_into().unwrap();
        let clm_size = std::fs::metadata(&clm_path)?.size().try_into().unwrap();

        // calculate regions
        let cyw = Cyw43439Regions::at_end(main_size, clm_size);

        // flash
        flash_pico(sh, main_path, cyw.main.origin)?;
        flash_pico(sh, clm_path, cyw.clm.origin)?;

        // write metadata file
        let flash_metadata_path = self.args.firmware_path(Cyw43439Regions::JSON_FILENAME);
        cyw.write_json(File::create(&flash_metadata_path).unwrap())?;

        println!(
            "flashed firmware, wrote metadata to {}",
            flash_metadata_path
                .strip_prefix(CARGO_WORKSPACE)
                .unwrap_or(&flash_metadata_path)
        );

        Ok(())
    }
}

impl Runnable for Status {
    fn run(&self, _sh: &mut Shell) -> anyhow::Result<()> {
        todo!("check if flash metadata file matches firmware files")
    }
}

impl Runnable for Clean {
    fn run(&self, _sh: &mut Shell) -> anyhow::Result<()> {
        let dir = &self.args.dir;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            std::fs::remove_file(entry.path())?;
        }

        Ok(())
    }
}

/// Flash specific file connected Pico's flash at a specific offset.
fn flash_pico(sh: &mut Shell, path: impl AsRef<Utf8Path>, offset: u32) -> anyhow::Result<()> {
    let offset = format!("0x{:x}", offset);
    let path = path.as_ref();

    cmd!(sh, "picotool load --update {path} -t bin --offset {offset}").run()?;

    Ok(())
}
