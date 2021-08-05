use std::sync::RwLock;

use nkeys::KeyPair;

use crate::{
  Host,
  Result,
};

/// The HostBuilder builds the configuration for a Vino Host.
#[derive(Debug, Copy, Clone)]
pub struct HostBuilder {}

impl Default for HostBuilder {
  fn default() -> Self {
    Self::new()
  }
}

impl HostBuilder {
  /// Creates a new host builder.
  #[must_use]
  pub fn new() -> HostBuilder {
    HostBuilder {}
  }

  pub async fn start(self) -> Result<Host> {
    let host = self.build();
    host.start().await?;
    Ok(host)
  }

  /// Constructs an instance of a Vino host.
  #[must_use]
  pub fn build(self) -> Host {
    let kp = KeyPair::new_server();
    let host_id = kp.public_key();
    Host {
      kp,
      host_id,
      started: RwLock::new(false),
      network: None,
    }
  }
}

#[cfg(test)]
mod test {
  use crate::HostBuilder;

  #[test]
  fn builds_default() {
    let _h = HostBuilder::new().build();
  }
}
