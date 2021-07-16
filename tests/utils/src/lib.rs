pub use maplit::hashmap;
pub use pretty_assertions::assert_eq as equals;
use vino_transport::message_transport::JsonOutput;

pub type TestResult<T> = Result<T, TestError>;

#[macro_use]
extern crate tracing;

use std::collections::HashMap;
use std::fs;

use vino_manifest::{
  Loadable,
  NetworkDefinition,
  NetworkManifest,
  SchematicDefinition,
};
use vino_runtime::network::Network;
use vino_wascap::KeyPair;
pub type TestError = anyhow::Error;
pub use anyhow::*;

pub async fn init_network_from_yaml(path: &str) -> TestResult<(Network, String)> {
  let manifest = NetworkManifest::V0(vino_manifest::v0::NetworkManifest::from_yaml(
    &fs::read_to_string(path)?,
  )?);
  let def = NetworkDefinition::from(manifest);
  debug!("Manifest loaded");
  let kp = KeyPair::new_server();

  let network = Network::new(def, &kp.seed()?)?;
  debug!("Initializing network");
  let init = network.init().await;
  info!("Init status : {:?}", init);
  init?;

  let network_id = network.id.clone();
  Ok((network, network_id))
}

pub fn load_network_manifest(path: &str) -> TestResult<NetworkDefinition> {
  let manifest = NetworkManifest::V0(vino_manifest::v0::NetworkManifest::from_yaml(
    &fs::read_to_string(path)?,
  )?);
  let def = NetworkDefinition::from(manifest);
  debug!("Manifest loaded");
  Ok(def)
}

pub fn new_schematic(name: &str) -> SchematicDefinition {
  SchematicDefinition {
    name: name.to_owned(),
    ..SchematicDefinition::default()
  }
}

use std::panic;
use std::process::Stdio;

use regex::Regex;
use serde_json::Value;
use tokio::io::{
  AsyncBufReadExt,
  BufReader,
};
use tokio::select;
use tokio::sync::mpsc::{
  self,
  Sender,
};
use tokio::task::JoinHandle;
use tracing::debug;

pub async fn vinoc_invoke(
  name: &str,
  data: Value,
) -> Result<HashMap<String, JsonOutput>, TestError> {
  debug!("Executing vinoc for schematic {}", name);
  let vinoc_output = tokio_test_bin::get_test_bin("vinoc")
    .env_clear()
    .args([
      "invoke",
      name,
      &serde_json::to_string(&data)?,
      "--port",
      "8060",
      "--trace",
    ])
    .stderr(Stdio::inherit())
    .output()
    .await?;
  debug!("Result from vinoc is {:?}", vinoc_output);

  let output = &String::from_utf8_lossy(&vinoc_output.stdout);
  debug!("Result from vinoc is {}", output);

  let result: HashMap<String, JsonOutput> = serde_json::from_str(output)?;

  Ok(result)
}

#[derive(Debug)]
pub enum Signal {
  Kill,
  Continue(String),
}

impl Signal {
  pub fn to_port(self) -> String {
    match self {
      Signal::Continue(s) => s,
      _ => panic!("not a continuation"),
    }
  }
}

pub async fn start_provider(
  name: &str,
  args: &[&str],
) -> Result<(Sender<Signal>, JoinHandle<Result<()>>, String)> {
  debug!("Starting provider bin: {}", name);
  let mut provider = tokio_test_bin::get_test_bin(name)
    .args(args)
    .env_clear()
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

  let stderr = provider.stderr.take().unwrap();
  let stdout = provider.stdout.take().unwrap();

  let mut err_reader = BufReader::new(stderr).lines();
  let mut out_reader = BufReader::new(stdout).lines();

  let (tx, mut rx) = mpsc::channel::<Signal>(1);

  debug!("Spawning task to handle provider");
  let handle = tokio::spawn(async move {
    select! {
      status = provider.wait() => {
        println!("provider status was: {:?}", status);
        Err(anyhow!("Provider stopped early"))
      },
      _signal = rx.recv() => {
        println!("provider received signal");
        provider.kill().await?;
        Ok(())
      }
    }
  });

  let re = Regex::new(r"Server bound to 127.0.0.1:(\d+)").unwrap();
  let (tx2, mut rx2) = mpsc::channel::<Signal>(1);

  tokio::spawn(async move {
    debug!("Reading provider STDOUT");
    while let Ok(Some(l)) = out_reader.next_line().await {
      debug!("Provider STDOUT: {}", l);

      if let Some(regex_match) = re.captures(&l) {
        let port = regex_match.get(1).unwrap();
        let _ = tx2.send(Signal::Continue(port.as_str().to_owned())).await;
      }
    }
    debug!("Continuing");
  });

  tokio::spawn(async move {
    debug!("Reading provider STDERR");
    while let Ok(Some(l)) = err_reader.next_line().await {
      debug!("Provider STDERR: {}", l);
    }
    debug!("Continuing");
  });

  println!("Waiting for provider to start up");
  let port = rx2.recv().await.unwrap();
  println!("Provider started, continuing");

  Ok((tx, handle, port.to_port()))
}

pub async fn start_vino(
  manifest: &str,
  envs: Vec<(&str, &str)>,
) -> Result<(Sender<Signal>, JoinHandle<Result<()>>)> {
  debug!("Spawning host with env {:?}", envs);

  let mut host = tokio_test_bin::get_test_bin("vino")
    .env_clear()
    .envs(envs)
    .args(["start", manifest, "--trace"])
    .stdout(Stdio::null())
    .stderr(Stdio::piped())
    .spawn()?;

  let (tx, mut rx) = mpsc::channel::<Signal>(1);

  let stderr = host.stderr.take().unwrap();
  let mut reader = BufReader::new(stderr).lines();

  debug!("Spawning task to handle host");
  let handle = tokio::spawn(async move {
    select! {
      status = host.wait() => {
        println!("Host stopped early: {:?}", status?);
        Err(anyhow!("Host stopped early"))
      },
      _signal = rx.recv() => {
        println!("host received signal");
        host.kill().await?;
        Ok(())
      }
    }
  });
  let re = Regex::new(r"Bound to 127.0.0.1:(\d+)").unwrap();

  let (tx2, mut rx2) = mpsc::channel::<Signal>(1);

  tokio::spawn(async move {
    debug!("Reading host STDERR");
    loop {
      if let Ok(Some(l)) = reader.next_line().await {
        debug!("Host STDERR: {}", l);

        if let Some(regex_match) = re.captures(&l) {
          let port = regex_match.get(1).unwrap();
          let _ = tx2.send(Signal::Continue(port.as_str().to_owned())).await;
        }
      }
    }
  });

  println!("Waiting for host to start up");
  rx2.recv().await;
  println!("Host started, continuing");

  Ok((tx, handle))
}