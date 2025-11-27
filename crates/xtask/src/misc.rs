use super::CARGO_WORKSPACE;
use camino::{Utf8Path, Utf8PathBuf};
use xshell::{Shell, cmd};

/// Downloads a single file to the specified path using system's curl.
pub(crate) fn download_file(
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
pub(crate) fn strip_cargo_dir(path: impl AsRef<Utf8Path>) -> Utf8PathBuf {
    path.as_ref()
        .strip_prefix(CARGO_WORKSPACE)
        .unwrap_or(path.as_ref())
        .to_path_buf()
}
