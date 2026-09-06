//! Allowlisted filesystem reads.

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use crate::error::{HostError, HostResult};

/// Roots that hosted open may read.
///
/// Access control lives here, not in MCP Roots. Each open canonicalizes the
/// requested path and requires it to stay under one configured root.
#[derive(Clone, Debug, Default)]
pub struct PathAllowlist {
    roots: Vec<PathBuf>,
}

impl PathAllowlist {
    /// Creates an empty allowlist that rejects every path.
    #[must_use]
    pub fn empty() -> Self {
        Self { roots: Vec::new() }
    }

    /// Creates an allowlist from roots that must already exist.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when a root cannot be canonicalized.
    pub fn new(roots: impl IntoIterator<Item = impl AsRef<Path>>) -> HostResult<Self> {
        let mut resolved = Vec::new();
        for root in roots {
            let path = root.as_ref();
            let canonical = path.canonicalize().map_err(|error| {
                HostError::new(
                    "host.path.root_unresolved",
                    format!("{}: {error}", path.display()),
                )
            })?;
            resolved.push(canonical);
        }
        Ok(Self { roots: resolved })
    }

    /// Whether any root is configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// Canonical roots currently allowed.
    #[must_use]
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Reads a file only when its resolved path stays inside an allowed root.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] for traversal, missing files, oversized reads, or
    /// I/O failure.
    pub fn read_limited(&self, path: &Path, limit: u64) -> HostResult<Vec<u8>> {
        let resolved = self.resolve(path)?;
        let mut file = File::open(&resolved).map_err(|error| {
            HostError::new(
                "host.io.read",
                format!("{}: {error}", resolved.display()),
            )
        })?;
        let mut bytes = Vec::new();
        file.by_ref()
            .take(limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|error| HostError::new("host.io.read", error.to_string()))?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
            return Err(HostError::new(
                "host.input.too_large",
                format!("limit={limit}"),
            ));
        }
        Ok(bytes)
    }

    fn resolve(&self, path: &Path) -> HostResult<PathBuf> {
        if self.roots.is_empty() {
            return Err(HostError::new(
                "host.path.access_denied",
                "no input roots are configured",
            ));
        }
        let resolved = path.canonicalize().map_err(|error| {
            HostError::new(
                "host.path.unresolved",
                format!("{}: {error}", path.display()),
            )
        })?;
        if self
            .roots
            .iter()
            .any(|root| resolved.starts_with(root))
        {
            Ok(resolved)
        } else {
            Err(HostError::new(
                "host.path.access_denied",
                format!("{}", resolved.display()),
            ))
        }
    }
}
