use std::collections::HashMap;

use serde_json::Value;

use crate::config;

/// A separate Wick manifest to use as a collection.
#[derive(Debug, Clone, PartialEq, derive_assets::AssetManager)]
#[asset(config::AssetReference)]
pub struct ManifestComponent {
  /// The OCI reference/local path of the manifest to use as a collection.
  pub reference: config::AssetReference,
  /// The configuration for the collection
  #[asset(skip)]
  pub config: Value,
  /// The components to provide to the referenced component.
  #[asset(skip)]
  pub(crate) provide: HashMap<String, String>,
}