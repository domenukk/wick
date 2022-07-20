use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

#[derive(Debug, Clone, Args)]
#[clap(rename_all = "kebab-case")]
pub(crate) struct Options {
  #[clap(flatten)]
  pub(crate) logging: logger::LoggingOptions,

  /// The filename to read (without path).
  #[clap(action)]
  path: PathBuf,

  /// Location of key files. Defaults to $WAFL_KEYS ($HOME/.wafl/keys or %USERPROFILE%/.wafl/keys).
  #[clap(long = "directory", env = "WAFL_KEYS", action)]
  pub(crate) directory: Option<PathBuf>,
}

#[allow(clippy::unused_async)]
pub(crate) async fn handle(opts: Options) -> Result<()> {
  let _guard = crate::utils::init_logger(&opts.logging)?;
  println!("Reading key: {}\n", opts.path.to_string_lossy());
  let kp = crate::keys::get_key(opts.directory, opts.path).await?;

  println!("Public key: {}", kp.public_key());
  println!("Private seed: {}", kp.seed()?);

  Ok(())
}
