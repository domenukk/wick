use thiserror::Error;

// type BoxedSyncSendError = Box<dyn std::error::Error + Sync + std::marker::Send>;

#[derive(Error, Debug)]
pub enum ControlError {
  #[error("invalid configuration")]
  ConfigurationError,
  #[error("File not found {0}")]
  FileNotFound(String),
  #[error("Configuration disallows fetching artifacts with the :latest tag ({0})")]
  LatestDisallowed(String),
  #[error("Could not fetch '{0}': {1}")]
  OciFetchFailure(String, String),
  #[error("Could not start host: {0}")]
  HostStartFailure(String),
  #[error("Failed to deserialize configuration {0}")]
  ConfigurationDeserialization(String),
  #[error(transparent)]
  RpcError(#[from] vino_rpc::Error),
  #[error(transparent)]
  VinoHostError(#[from] vino_host::Error),
  #[error(transparent)]
  VinoRuntimeError(#[from] vino_runtime::Error),
  #[error(transparent)]
  ConnectionError(#[from] tonic::transport::Error),
  #[error(transparent)]
  IOError(#[from] std::io::Error),
  #[error(transparent)]
  SerdeJsonError(#[from] serde_json::Error),
  #[error(transparent)]
  LoggerError(#[from] log::SetLoggerError),
  #[error("General error : {0}")]
  Other(String),
}
