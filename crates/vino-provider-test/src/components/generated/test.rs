use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use vino_provider::port::{Port, Receiver, Sender};
use vino_runtime::deserialize;

#[derive(Debug, PartialEq, Deserialize, Serialize, Default, Clone)]
pub struct Inputs {
  pub input: String,
}

pub(crate) fn inputs_list() -> Vec<String> {
  vec!["input".to_string()]
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Default, Clone)]
pub(crate) struct InputEncoded {
  #[serde(rename = "input")]
  pub input: Vec<u8>,
}

pub(crate) fn deserialize_inputs(
  args: InputEncoded,
) -> std::result::Result<Inputs, Box<dyn std::error::Error + Send + Sync>> {
  Ok(Inputs {
    input: deserialize(&args.input)?,
  })
}

#[derive(Default)]
pub struct Outputs {
  pub(crate) output: OutputSender,
}

pub(crate) fn outputs_list() -> Vec<String> {
  vec!["output".to_string()]
}

pub struct OutputSender {
  port: Arc<Mutex<Port>>,
}
impl Default for OutputSender {
  fn default() -> Self {
    Self {
      port: Arc::new(Mutex::new(Port::new("output"))),
    }
  }
}
impl Sender for OutputSender {
  type Payload = String;

  fn get_port(&self) -> Arc<Mutex<Port>> {
    self.port.clone()
  }
}

pub fn get_outputs() -> (Outputs, Receiver) {
  let outputs = Outputs::default();
  let ports = vec![outputs.output.port.clone()];
  let receiver = Receiver::new(ports);
  (outputs, receiver)
}