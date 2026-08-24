use ferrosift_core::OperationRegistry;

use crate::error::Error;
use crate::pipeline::Pipeline;

/// A validated operation registry, built once and reused.
///
/// Building the registry constructs and validates every built-in operation,
/// so the cost grows with the size of the catalog. An [`Engine`] pays that
/// cost once; [`crate::CompiledPipeline`] then resolves a pipeline against it
/// once more, leaving each run to do nothing but execute.
///
/// ```
/// use ferrosift::Engine;
///
/// let engine = Engine::portable()?;
/// let pipeline = engine.pipeline().from_base64().compile(&engine)?;
///
/// for encoded in [&b"Zm9v"[..], b"YmFy"] {
///     assert_eq!(pipeline.run_bytes(encoded)?.len(), 3);
/// }
/// # Ok::<(), ferrosift::Error>(())
/// ```
pub struct Engine {
    registry: OperationRegistry,
}

impl Engine {
    /// Builds an engine over every portable built-in operation.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Registry`] if an internal operation contract does not
    /// validate. The registry is never partially initialized.
    pub fn portable() -> Result<Self, Error> {
        Ok(Self {
            registry: ferrosift_operations::default_registry()?,
        })
    }

    /// Builds an engine over a registry the caller assembled.
    ///
    /// This is the seam for a reduced catalog: register only the operations a
    /// deployment needs instead of paying for all of them.
    #[must_use]
    pub const fn with_registry(registry: OperationRegistry) -> Self {
        Self { registry }
    }

    /// The registry backing this engine.
    #[must_use]
    pub const fn registry(&self) -> &OperationRegistry {
        &self.registry
    }

    /// Number of operations available to pipelines built from this engine.
    #[must_use]
    pub fn len(&self) -> usize {
        self.registry.len()
    }

    /// Whether the engine has no operations at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.registry.len() == 0
    }

    /// Starts an empty pipeline; compile it against this same engine.
    #[must_use]
    pub fn pipeline(&self) -> Pipeline {
        Pipeline::new()
    }
}
