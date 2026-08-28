//! Deterministic, fail-closed operation discovery.

use alloc::{boxed::Box, collections::BTreeMap, collections::BTreeSet, string::String, vec::Vec};

use ferrosift_model::{
    Arguments, CompatibilityProfile, EvidenceManifest, OperationId, OperationSpec, Value,
};

use crate::{FlowDirective, Operation, OperationContext, OperationError};

mod error;

pub use error::RegistryError;

type ProfileAliases = BTreeMap<String, OperationId>;
type AliasKey = (CompatibilityProfile, String);

/// One catalog entry: the validated specification, and what implements it.
///
/// The specification is held here rather than fetched through the boxed
/// implementation because it was validated at registration and must not change
/// afterwards. Everything else is forwarded — and *every* method must be, or
/// the trait's default answer silently replaces the implementation's. A `Jump`
/// registered here reported "continue with the next step" for exactly that
/// reason, because this wrapper had `execute` and nothing else.
struct RegisteredOperation {
    spec: OperationSpec,
    implementation: Box<dyn Operation>,
}

impl Operation for RegisteredOperation {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        self.implementation.execute(input, arguments, context)
    }

    fn direct(
        &self,
        value: &Value,
        arguments: &Arguments,
        context: &OperationContext<'_>,
    ) -> Result<FlowDirective, OperationError> {
        self.implementation.direct(value, arguments, context)
    }
}

/// An in-memory catalog of validated portable operations.
pub struct OperationRegistry {
    operations: BTreeMap<OperationId, RegisteredOperation>,
    aliases: BTreeMap<CompatibilityProfile, ProfileAliases>,
    evidence: EvidenceManifest,
}

impl OperationRegistry {
    /// Creates an empty registry that claims no evidence.
    ///
    /// Registering an operation into it is refused, because every operation
    /// declares at least one target and this registry has checked none. That is
    /// the honest answer rather than an inconvenient one — see
    /// [`Self::declare_evidence`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            operations: BTreeMap::new(),
            aliases: BTreeMap::new(),
            evidence: EvidenceManifest::unverified(),
        }
    }

    /// Records what this build checked, and where to read the check.
    ///
    /// A catalog is a set of claims and this is what stands behind them: the
    /// provenance, the licence, the published measurements, and which targets
    /// were actually compiled and run. It was carried on every specification
    /// until it became clear that not one dimension of it was a fact about an
    /// operation — the same five records, two hundred and fifty-four times, one
    /// of them naming a single test file for the whole catalog.
    ///
    /// Declared before registering, because [`Self::register`] checks each
    /// operation's targets against it.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] when the manifest's own records are
    /// inconsistent, or when an operation already registered declares a target
    /// the new manifest does not cover.
    pub fn declare_evidence(&mut self, evidence: EvidenceManifest) -> Result<(), RegistryError> {
        evidence.validate()?;
        for operation in self.operations.values() {
            operation.spec.check_targets(&evidence)?;
        }
        self.evidence = evidence;
        Ok(())
    }

    /// What this build checked.
    #[must_use]
    pub const fn evidence(&self) -> &EvidenceManifest {
        &self.evidence
    }

    /// Validates and atomically registers one operation.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] when the contract is invalid, when the
    /// operation declares a target this build has not checked, or when any
    /// canonical ID or profile-scoped alias is already registered. The registry
    /// is unchanged on every failure path.
    pub fn register<O>(&mut self, operation: O) -> Result<(), RegistryError>
    where
        O: Operation + 'static,
    {
        let spec = operation.spec();
        spec.validate()?;
        spec.check_targets(&self.evidence)?;

        let id = spec.id.clone();
        if self.operations.contains_key(&id) {
            return Err(RegistryError::DuplicateOperation { id });
        }

        let alias_keys: Vec<_> = spec
            .aliases
            .iter()
            .map(|alias| (alias.profile, alias.name.clone()))
            .collect();
        self.validate_aliases(&alias_keys)?;

        self.operations.insert(
            id.clone(),
            RegisteredOperation {
                spec: spec.clone(),
                implementation: Box::new(operation),
            },
        );
        for (profile, name) in alias_keys {
            self.aliases
                .entry(profile)
                .or_default()
                .insert(name, id.clone());
        }
        Ok(())
    }

    /// Returns an operation by canonical ID.
    #[must_use]
    pub fn get(&self, id: &OperationId) -> Option<&dyn Operation> {
        self.operations
            .get(id)
            .map(|operation| operation as &dyn Operation)
    }

    /// Resolves an exact alias within one compatibility profile.
    #[must_use]
    pub fn resolve_alias(
        &self,
        profile: CompatibilityProfile,
        name: &str,
    ) -> Option<&dyn Operation> {
        let id = self.aliases.get(&profile)?.get(name)?;
        self.get(id)
    }

    /// Iterates over specifications in canonical operation-ID order.
    #[must_use]
    pub fn catalog(&self) -> impl ExactSizeIterator<Item = &OperationSpec> {
        self.operations.values().map(|operation| &operation.spec)
    }

    /// Returns the number of registered operations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Returns whether no operation is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    fn validate_aliases(&self, alias_keys: &[AliasKey]) -> Result<(), RegistryError> {
        let mut pending = BTreeSet::new();
        for (profile, name) in alias_keys {
            let collides_with_registry = self
                .aliases
                .get(profile)
                .is_some_and(|aliases| aliases.contains_key(name));
            if collides_with_registry || !pending.insert((*profile, name.as_str())) {
                return Err(RegistryError::DuplicateAlias {
                    profile: *profile,
                    name: name.clone(),
                });
            }
        }
        Ok(())
    }
}

impl Default for OperationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
