//! Opaque in-memory artifact handles.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use ferrosift_core::ValueSummary;
use ferrosift_model::{Value, ValueKind};
use sha2::{Digest, Sha256};

use crate::error::{HostError, HostResult};

/// Opaque handle for a stored value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ArtifactId(String);

impl ArtifactId {
    /// Stable string form used on the wire.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for ArtifactId {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Bounded metadata returned without the full payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactMeta {
    /// Opaque handle.
    pub id: ArtifactId,
    /// Value representation.
    pub kind: ValueKind,
    /// Logical payload size.
    pub size_bytes: u64,
    /// Content digest of the stored payload encoding.
    pub digest_sha256: String,
}

/// Store configuration and ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoreConfig {
    /// Maximum total logical bytes retained.
    pub max_total_bytes: u64,
    /// Maximum lifetime of an artifact after insertion.
    pub ttl: Duration,
    /// Maximum preview returned by inspect.
    pub max_preview_bytes: usize,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            max_total_bytes: 256 * 1024 * 1024,
            ttl: Duration::from_hours(1),
            max_preview_bytes: 1024,
        }
    }
}

#[derive(Debug)]
struct StoredArtifact {
    value: Value,
    created_at: Instant,
    digest_sha256: String,
    size_bytes: u64,
}

/// Process-local immutable artifact store with TTL and quota.
#[derive(Debug)]
pub struct ArtifactStore {
    artifacts: HashMap<ArtifactId, StoredArtifact>,
    next_id: u64,
    total_bytes: u64,
    config: StoreConfig,
}

impl ArtifactStore {
    /// Creates an empty store.
    #[must_use]
    pub fn new(config: StoreConfig) -> Self {
        Self {
            artifacts: HashMap::new(),
            next_id: 1,
            total_bytes: 0,
            config,
        }
    }

    /// Store configuration.
    #[must_use]
    pub const fn config(&self) -> StoreConfig {
        self.config
    }

    /// Inserts an immutable artifact and returns its handle.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when the store quota would be exceeded.
    pub fn insert(&mut self, value: Value) -> HostResult<ArtifactMeta> {
        self.expire();
        let size_bytes = ValueSummary::from_value(&value).size_bytes;
        let next_total = self.total_bytes.saturating_add(size_bytes);
        if next_total > self.config.max_total_bytes {
            return Err(HostError::new(
                "host.artifact.quota_exceeded",
                format!(
                    "total={next_total} limit={}",
                    self.config.max_total_bytes
                ),
            ));
        }
        let id = ArtifactId(format!("art_{:016x}", self.next_id));
        self.next_id = self.next_id.saturating_add(1);
        let digest_sha256 = digest_value(&value);
        let meta = ArtifactMeta {
            id: id.clone(),
            kind: value.kind(),
            size_bytes,
            digest_sha256: digest_sha256.clone(),
        };
        self.artifacts.insert(
            id,
            StoredArtifact {
                value,
                created_at: Instant::now(),
                digest_sha256,
                size_bytes,
            },
        );
        self.total_bytes = next_total;
        Ok(meta)
    }

    /// Returns metadata for a live artifact.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when the handle is unknown or expired.
    pub fn meta(&mut self, id: &ArtifactId) -> HostResult<ArtifactMeta> {
        let artifact = self.get_live(id)?;
        Ok(ArtifactMeta {
            id: id.clone(),
            kind: artifact.value.kind(),
            size_bytes: artifact.size_bytes,
            digest_sha256: artifact.digest_sha256.clone(),
        })
    }

    /// Clones the stored value for a live artifact.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when the handle is unknown or expired.
    pub fn get_value(&mut self, id: &ArtifactId) -> HostResult<Value> {
        Ok(self.get_live(id)?.value.clone())
    }

    /// Removes an artifact if present.
    pub fn remove(&mut self, id: &ArtifactId) {
        if let Some(artifact) = self.artifacts.remove(id) {
            self.total_bytes = self.total_bytes.saturating_sub(artifact.size_bytes);
        }
    }

    /// Parses a wire handle.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when the string is not a valid handle form.
    pub fn parse_id(raw: &str) -> HostResult<ArtifactId> {
        if raw.starts_with("art_") && raw.len() > 4 && raw.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'_'
        }) {
            Ok(ArtifactId(raw.to_owned()))
        } else {
            Err(HostError::new(
                "host.artifact.invalid_id",
                raw.to_owned(),
            ))
        }
    }

    fn get_live(&mut self, id: &ArtifactId) -> HostResult<&StoredArtifact> {
        self.expire();
        self.artifacts
            .get(id)
            .ok_or_else(|| HostError::new("host.artifact.expired", id.as_str()))
    }

    fn expire(&mut self) {
        let ttl = self.config.ttl;
        let now = Instant::now();
        let expired: Vec<ArtifactId> = self
            .artifacts
            .iter()
            .filter(|(_, artifact)| now.duration_since(artifact.created_at) > ttl)
            .map(|(id, _)| id.clone())
            .collect();
        for id in expired {
            self.remove(&id);
        }
    }
}

fn digest_value(value: &Value) -> String {
    let mut hasher = Sha256::new();
    match value {
        Value::Bytes(bytes) => hasher.update(bytes),
        Value::Text(text) => hasher.update(text.text.as_bytes()),
        Value::Empty => {}
        other => hasher.update(format!("{other:?}").as_bytes()),
    }
    format!("sha256:{:x}", hasher.finalize())
}
