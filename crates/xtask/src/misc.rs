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

/// Call the same code in all variants of enum. All variants must contain tuple data.
///
/// # Arguments:
/// - Enum type
/// - All enum variants, surrounded by `[]`, separated by commas
/// - `self`
/// - Identifier for the inner value of each variant
/// - Code to run on the inner value of each variant (should probably be surrounded by `{}`)
#[macro_export]
macro_rules! enum_call {
    ($enum_ty:ty, [$($variant:ident $(,)?)*], $self:expr, $inner:ident, $code: tt ) => {
       {
            use $enum_ty::*;
            match $self {
                $(
                    $variant($inner) => $code,
                )*
            }
        }
    };
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
