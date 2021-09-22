use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::Arc;

use redis::aio::Connection;
use redis::FromRedisValue;
use tokio::sync::RwLock;
use vino_provider::native::prelude::*;
use vino_rpc::error::RpcError;
use vino_rpc::{
  RpcHandler,
  RpcResult,
};

use crate::error::Error;
use crate::generated::{
  self,
  Dispatcher,
};

pub(crate) type Context = Arc<RedisConnection>;

#[allow(missing_debug_implementations)]
pub struct RedisConnection(RwLock<Connection>);

pub type RedisResult<T> = std::result::Result<T, Error>;

impl From<Error> for NativeComponentError {
  fn from(e: Error) -> Self {
    NativeComponentError::new(e.to_string())
  }
}

impl RedisConnection {
  pub async fn run_cmd<T: FromRedisValue + std::fmt::Debug>(
    &self,
    cmd: &mut redis::Cmd,
  ) -> RedisResult<T> {
    let mut con = self.0.write().await;
    let result: Result<T> = cmd
      .query_async(&mut *con)
      .await
      .map_err(|e| Error::RedisError(e.to_string()));

    if log_enabled!(log::Level::Trace) {
      let bytes = cmd.get_packed_command();
      let cmdstring = String::from_utf8_lossy(&bytes);
      trace!("REDIS:EXEC[{}]", cmdstring);

      trace!("REDIS:RESULT[{:?}]", result);
    }
    result
  }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
#[allow(missing_debug_implementations)]
pub struct State {
  pub connections: RwLock<HashMap<String, Context>>,
}

#[derive(Clone, Default)]
#[must_use]
#[allow(missing_debug_implementations)]
pub struct Provider {
  context: Arc<RwLock<State>>,
}

impl Provider {
  pub fn default() -> Self {
    Self {
      context: Arc::new(RwLock::new(State::default())),
    }
  }
  pub fn new() -> Self {
    Self::default()
  }
  pub async fn connect(&self, namespace: String, url: String) -> Result<()> {
    let client = redis::Client::open(url.clone())
      .map_err(|e| Error::Init(format!("connection to {}: {}", url, e)))?;

    let connection = client
      .get_async_connection()
      .await
      .map_err(|e| Error::Init(format!("connection to {}: {}", url, e)))?;

    let context = self.context.write().await;

    let mut update_map = context.connections.write().await;
    update_map.insert(
      namespace,
      Arc::new(RedisConnection(RwLock::new(connection))),
    );
    Ok(())
  }
}

#[async_trait]
impl RpcHandler for Provider {
  async fn invoke(&self, entity: Entity, payload: TransportMap) -> RpcResult<BoxedTransportStream> {
    let context = self.context.read().await;
    let connections = context.connections.read().await;
    let namespace = "default".to_owned();
    let connection = connections
      .get(&namespace)
      .ok_or_else(|| RpcError::ProviderError(Error::ConnectionNotFound(namespace).to_string()))?;
    debug!("Dispatching to {}", entity.url());
    let component = entity.name();
    let stream = Dispatcher::dispatch(&component, connection.clone(), payload)
      .await
      .map_err(|e| RpcError::ProviderError(e.to_string()))?;

    Ok(Box::pin(stream))
  }

  async fn get_list(&self) -> RpcResult<Vec<HostedType>> {
    let components = generated::get_all_components();
    Ok(components.into_iter().map(HostedType::Component).collect())
  }
}

#[cfg(test)]
mod tests {

  use anyhow::{
    anyhow,
    Result,
  };
  use futures::prelude::*;
  use rand::Rng;
  use serde::de::DeserializeOwned;

  use super::*;

  async fn get_next<De: DeserializeOwned + std::fmt::Debug>(
    stream: &mut BoxedTransportStream,
  ) -> Result<De> {
    let output = match stream.next().await {
      Some(o) => o,
      None => return Err(anyhow!("Nothing received from stream")),
    };
    debug!("payload from port: '{}': {:?}", output.port, output.payload);
    let val: De = output.payload.try_into()?;
    debug!("decoded payload: {:?}", val);
    Ok(val)
  }

  async fn get_maybe_next<De: DeserializeOwned + std::fmt::Debug>(
    stream: &mut BoxedTransportStream,
  ) -> Result<Option<De>> {
    let output = match stream.next().await {
      Some(o) => o,
      None => return Err(anyhow!("Nothing received from stream")),
    };
    debug!("payload from [{}]: {:?}", output.port, output.payload);
    if output.payload.is_ok() {
      let val: De = output.payload.try_into()?;
      debug!("decoded payload: {:?}", val);
      Ok(Some(val))
    } else {
      match output.payload {
        MessageTransport::Exception(msg) => Ok(None),
        MessageTransport::Error(e) => Err(anyhow!("{}", e)),
        _ => unreachable!(),
      }
    }
  }

  async fn key_set(provider: &Provider, key: &str, value: &str, expires: u32) -> Result<String> {
    debug!("key-set:{}::{}::{}", key, value, expires);
    let payload = vino_interface_keyvalue::key_set::Inputs {
      key: key.to_owned(),
      value: value.to_owned(),
      expires,
    };

    let mut stream = provider
      .invoke(Entity::component_direct("key-set"), payload.into())
      .await?;

    let actual: String = get_next(&mut stream).await?;

    assert_eq!(key, actual);

    Ok(actual)
  }

  async fn key_get(provider: &Provider, key: &str) -> Result<Option<String>> {
    debug!("key-get:{}", key);
    let payload = vino_interface_keyvalue::key_get::Inputs {
      key: key.to_owned(),
    };

    let mut stream = provider
      .invoke(Entity::component_direct("key-get"), payload.into())
      .await?;

    let actual: Option<String> = get_maybe_next(&mut stream).await?;

    Ok(actual)
  }

  async fn delete(provider: &Provider, key: &str) -> Result<String> {
    debug!("delete:{}", key);
    let payload = vino_interface_keyvalue::delete::Inputs {
      key: key.to_owned(),
    };

    let mut stream = provider
      .invoke(Entity::component_direct("delete"), payload.into())
      .await?;

    let actual: String = get_next(&mut stream).await?;

    Ok(actual)
  }

  async fn exists(provider: &Provider, key: &str) -> Result<bool> {
    debug!("exists:{}", key);
    let payload = vino_interface_keyvalue::exists::Inputs {
      key: key.to_owned(),
    };

    let mut stream = provider
      .invoke(Entity::component_direct("exists"), payload.into())
      .await?;

    let output = match stream.next().await {
      Some(o) => o,
      None => return Err(anyhow!("Nothing received from stream")),
    };

    if output.payload.is_ok() {
      let deleted_key: String = output.payload.try_into()?;
      assert_eq!(deleted_key, key);
      Ok(true)
    } else if matches!(output.payload, MessageTransport::Exception(_)) {
      Ok(false)
    } else {
      Err(anyhow!("Error with exists"))
    }
  }

  async fn list_add(provider: &Provider, key: &str, value: &str) -> Result<String> {
    debug!("list-add:{}::{}", key, value);
    let payload = vino_interface_keyvalue::list_add::Inputs {
      key: key.to_owned(),
      value: value.to_owned(),
    };

    let mut stream = provider
      .invoke(Entity::component_direct("list-add"), payload.into())
      .await?;

    let actual: String = get_next(&mut stream).await?;

    Ok(actual)
  }

  async fn list_range(provider: &Provider, key: &str, start: i32, end: i32) -> Result<Vec<String>> {
    debug!("list-range:{}::{}::{}", key, start, end);
    let payload = vino_interface_keyvalue::list_range::Inputs {
      key: key.to_owned(),
      start,
      end,
    };

    let mut stream = provider
      .invoke(Entity::component_direct("list-range"), payload.into())
      .await?;

    let actual: Vec<String> = get_next(&mut stream).await?;

    Ok(actual)
  }

  async fn list_remove(provider: &Provider, key: &str, value: &str) -> Result<String> {
    debug!("list-remove:{}::{}", key, value);
    let payload = vino_interface_keyvalue::list_remove::Inputs {
      key: key.to_owned(),
      value: value.to_owned(),
    };

    let mut stream = provider
      .invoke(Entity::component_direct("list-remove"), payload.into())
      .await?;

    let actual: String = get_next(&mut stream).await?;

    Ok(actual)
  }

  async fn set_add(provider: &Provider, key: &str, value: &str) -> Result<String> {
    debug!("set-add:{}::{}", key, value);
    let payload = vino_interface_keyvalue::set_add::Inputs {
      key: key.to_owned(),
      value: value.to_owned(),
    };

    let mut stream = provider
      .invoke(Entity::component_direct("set-add"), payload.into())
      .await?;

    let actual: String = get_next(&mut stream).await?;

    Ok(actual)
  }

  async fn set_get(provider: &Provider, key: &str) -> Result<Vec<String>> {
    debug!("set-get:{}", key);
    let payload = vino_interface_keyvalue::set_get::Inputs {
      key: key.to_owned(),
    };

    let mut stream = provider
      .invoke(Entity::component_direct("set-get"), payload.into())
      .await?;

    let actual: Vec<String> = get_next(&mut stream).await?;

    Ok(actual)
  }

  async fn set_remove(provider: &Provider, key: &str, value: &str) -> Result<String> {
    debug!("set-remove:{}::{}", key, value);
    let payload = vino_interface_keyvalue::set_remove::Inputs {
      key: key.to_owned(),
      value: value.to_owned(),
    };

    let mut stream = provider
      .invoke(Entity::component_direct("set-remove"), payload.into())
      .await?;

    let actual: String = get_next(&mut stream).await?;

    Ok(actual)
  }

  async fn get_default_provider() -> Result<Provider> {
    let provider = Provider::default();
    let url = std::env::var(crate::REDIS_URL_ENV.to_owned())
      .unwrap_or_else(|_| "redis://0.0.0.0:6379".to_owned());
    provider.connect("default".to_owned(), url).await?;
    Ok(provider)
  }

  fn get_random_string() -> String {
    rand::thread_rng()
      .sample_iter(&rand::distributions::Alphanumeric)
      .take(30)
      .map(char::from)
      .collect()
  }

  #[test_logger::test(tokio::test)]
  async fn test_key_set_get_contains_delete() -> Result<()> {
    let provider = get_default_provider().await?;
    let nonexistant_key = get_random_string();
    let key = get_random_string();
    let expected = get_random_string();
    let expires = 10000;

    assert!(!exists(&provider, &key).await?);
    let key2 = key_set(&provider, &key, &expected, expires).await?;
    assert!(exists(&provider, &key).await?);
    let actual = key_get(&provider, &key2).await?.unwrap();
    assert_eq!(actual, expected);
    let nonexistant = key_get(&provider, &nonexistant_key).await?;
    assert_eq!(nonexistant, None);
    delete(&provider, &key2).await?;
    assert!(!exists(&provider, &key).await?);

    Ok(())
  }

  #[test_logger::test(tokio::test)]
  async fn test_list() -> Result<()> {
    let provider = get_default_provider().await?;
    let nonexistant_key = get_random_string();
    let key = get_random_string();
    let expected = get_random_string();

    assert!(!exists(&provider, &key).await?);
    let key2 = list_add(&provider, &key, &expected).await?;
    assert!(exists(&provider, &key).await?);
    let values = list_range(&provider, &key2, 0, 1).await?;
    let range = vec![expected.clone()];
    assert_eq!(values, range);
    let mut rest = vec![
      get_random_string(),
      get_random_string(),
      get_random_string(),
      get_random_string(),
    ];
    list_add(&provider, &key, &rest[0]).await?;
    list_add(&provider, &key, &rest[1]).await?;
    list_add(&provider, &key, &rest[2]).await?;
    list_add(&provider, &key, &rest[3]).await?;
    let values = list_range(&provider, &key2, 0, 0).await?;
    assert_eq!(values, range);
    let values = list_range(&provider, &key2, 0, 1).await?;
    assert_eq!(values, vec![expected.clone(), rest[0].clone()]);
    let values = list_range(&provider, &key2, 0, -1).await?;
    let mut all = range.clone();
    all.append(&mut rest);
    assert_eq!(values, all);
    list_remove(&provider, &key2, &expected).await?;
    let values = list_range(&provider, &key2, 0, -1).await?;
    assert_eq!(values, &all[1..]);
    delete(&provider, &key2).await?;
    let values = list_range(&provider, &key2, 0, -1).await?;
    let none: Vec<String> = vec![];
    assert_eq!(values, none);

    Ok(())
  }

  #[test_logger::test(tokio::test)]
  async fn test_set_add_get_remove() -> Result<()> {
    let provider = get_default_provider().await?;
    let key = get_random_string();
    let expected = get_random_string();
    let range = vec![expected.clone()];

    assert!(!exists(&provider, &key).await?);
    set_add(&provider, &key, &expected).await?;
    assert!(exists(&provider, &key).await?);
    let values = set_get(&provider, &key).await?;
    assert_eq!(values, range);
    set_add(&provider, &key, &expected).await?;
    let values = set_get(&provider, &key).await?;
    assert_eq!(values, range);
    set_remove(&provider, &key, &expected).await?;
    let values = set_get(&provider, &key).await?;
    let none: Vec<String> = vec![];
    assert_eq!(values, none);
    assert!(!exists(&provider, &key).await?);

    Ok(())
  }
}