use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use parking_lot::RwLock;
use vino_codec::messagepack::{deserialize, serialize};
use vino_packet::v0::Payload;
use vino_packet::Packet;
use vino_transport::TransportMap;
use vino_wapc::{LogLevel, OutputSignal};

use crate::provider::HostLinkCallback;
use crate::transaction::Transaction;

type InvocationFn =
  dyn Fn(&str, &str, &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> + 'static + Sync + Send;

pub(crate) fn create_log_handler() -> Box<InvocationFn> {
  Box::new(move |level: &str, msg: &str, _: &[u8]| {
    match LogLevel::from_str(level) {
      Ok(lvl) => match lvl {
        LogLevel::Info => info!("WASM: {}", msg),
        LogLevel::Error => error!("WASM: {}", msg),
        LogLevel::Warn => warn!("WASM: {}", msg),
        LogLevel::Debug => debug!("WASM: {}", msg),
        LogLevel::Trace => trace!("WASM: {}", msg),
        LogLevel::Mark => {
          let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
          trace!("WASM:[{}]: {}", now.as_millis(), msg);
        }
      },
      Err(_) => {
        return Err(format!("Invalid log level: {}", level).into());
      }
    };
    Ok(vec![])
  })
}

pub(crate) fn create_link_handler(callback: Arc<Option<Box<HostLinkCallback>>>) -> Box<InvocationFn> {
  Box::new(
    move |origin: &str, target: &str, payload: &[u8]| match callback.as_ref() {
      Some(cb) => {
        trace!(origin, target, "wasm link call");
        let now = Instant::now();
        let result = (cb)(origin, target, deserialize::<TransportMap>(payload)?);
        let micros = now.elapsed().as_micros();
        trace!(origin, target, durasion_us = %micros, ?result, "wasm link call result");

        match result {
          Ok(packets) => {
            // ensure all packets are messagepack-ed
            let packets: Vec<_> = packets
              .into_iter()
              .map(|mut p| {
                p.payload.to_messagepack();
                p
              })
              .collect();
            trace!(origin, target, ?payload, "wasm link call payload");
            Ok(serialize(&packets)?)
          }
          Err(e) => Err(e.into()),
        }
      }
      None => Err("Host link called with no callback provided in the WaPC host.".into()),
    },
  )
}

pub(crate) fn create_output_handler(tx_map: Arc<RwLock<HashMap<u32, RwLock<Transaction>>>>) -> Box<InvocationFn> {
  Box::new(move |port: &str, output_signal, bytes: &[u8]| {
    let payload = &bytes[4..bytes.len()];
    let mut be_bytes: [u8; 4] = [0; 4];
    be_bytes.copy_from_slice(&bytes[0..4]);
    let id: u32 = u32::from_be_bytes(be_bytes);
    trace!(id, port, ?payload, "output payload");
    let mut lock = tx_map.write();
    let mut tx = lock
      .get_mut(&id)
      .ok_or(format!("Invalid transaction (TX: {})", id))?
      .write();

    match OutputSignal::from_str(output_signal) {
      Ok(signal) => match signal {
        OutputSignal::Output => {
          if tx.ports.contains(port) {
            Err(format!("Port '{}' already closed for (TX: {})", port, id).into())
          } else {
            tx.buffer.push_back((port.to_owned(), payload.into()));
            Ok(vec![])
          }
        }
        OutputSignal::OutputDone => {
          if tx.ports.contains(port) {
            Err(format!("Port '{}' already closed for (TX: {})", port, id).into())
          } else {
            tx.buffer.push_back((port.to_owned(), payload.into()));
            tx.buffer.push_back((port.to_owned(), Packet::V0(Payload::Done)));
            trace!(id, port, "port closing");
            tx.ports.insert(port.to_owned());
            Ok(vec![])
          }
        }
        OutputSignal::Done => {
          tx.buffer.push_back((port.to_owned(), Packet::V0(Payload::Done)));
          trace!(id, port, "port closing");
          tx.ports.insert(port.to_owned());
          Ok(vec![])
        }
      },
      Err(_) => Err("Invalid signal".into()),
    }
  })
}