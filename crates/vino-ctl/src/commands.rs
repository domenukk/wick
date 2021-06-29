pub mod invoke;
pub mod list;
pub mod stats;

use std::net::Ipv4Addr;

use logger::options::LoggingOptions;
use structopt::clap::AppSettings;
use structopt::StructOpt;

pub fn get_args() -> Cli {
  Cli::from_args()
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(
     global_settings(&[AppSettings::VersionlessSubcommands]),
     name = "vino", about = "Vino host runtime")]
pub struct Cli {
  #[structopt(flatten)]
  pub command: CliCommand,
}

#[derive(Debug, Clone, StructOpt)]
pub enum CliCommand {
  ///
  #[structopt(name = "start")]
  Invoke(invoke::InvokeCommand),
  ///
  #[structopt(name = "list")]
  List(list::ListCommand),
  ///
  #[structopt(name = "stats")]
  Stats(stats::StatsCommand),
}

#[derive(Debug, Clone, StructOpt)]
pub struct ConnectOptions {
  /// Port to listen on
  #[structopt(short, long)]
  pub port: u16,

  /// IP address to bind to
  #[structopt(short, long, default_value = "127.0.0.1")]
  pub address: Ipv4Addr,
}
