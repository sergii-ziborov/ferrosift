//! Multi-source pattern loading through a caller-supplied resolver.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::ast::{Pattern, SourceEntry, SourceOrigin};
use crate::error::{PatternError, Position, SourceId};
use crate::lexer;
use crate::source::{ImportKind, PatternResolver, ResolveLimits, ResolveRequest};

use super::cursor::Cursor;
use super::grammar;

/// Parses a root pattern and expands `import` / `#include` through `resolver`.
///
/// `import` and `#include` both contribute declarations to one flat namespace
/// in this experiment. They are recorded with different [`SourceOrigin`] values
/// so a later survey can measure whether reference semantics diverge.
///
/// # Errors
///
/// Returns lex/parse failures from any loaded source, or resolve-ceiling codes.
pub fn parse_with(
    root_label: impl Into<String>,
    root_source: &str,
    resolver: &dyn PatternResolver,
    limits: ResolveLimits,
) -> Result<Pattern, PatternError> {
    let mut loader = Loader {
        resolver,
        limits,
        sources: Vec::new(),
        labels: BTreeMap::new(),
        once: BTreeSet::new(),
        stack: Vec::new(),
        total_bytes: 0,
    };
    let root_id = loader.allocate(
        root_label.into(),
        SourceOrigin::Root,
        root_source.len(),
        Position {
            line: 1,
            column: 1,
            source: SourceId::ROOT,
        },
    )?;
    let mut pattern = loader.load(root_id, root_source, 0)?;
    pattern.sources = loader.sources;
    Ok(pattern)
}

pub(super) struct Loader<'a> {
    resolver: &'a dyn PatternResolver,
    limits: ResolveLimits,
    pub(super) sources: Vec<SourceEntry>,
    labels: BTreeMap<String, SourceId>,
    once: BTreeSet<SourceId>,
    stack: Vec<SourceId>,
    total_bytes: usize,
}

impl Loader<'_> {
    fn allocate(
        &mut self,
        label: String,
        origin: SourceOrigin,
        bytes: usize,
        at: Position,
    ) -> Result<SourceId, PatternError> {
        if self.sources.len() >= self.limits.max_sources {
            return Err(PatternError::new(
                "pattern.resolve.too_many_sources",
                at,
                format!("limit={}", self.limits.max_sources),
            ));
        }
        let next_total = self.total_bytes.saturating_add(bytes);
        if next_total > self.limits.max_total_bytes {
            return Err(PatternError::new(
                "pattern.resolve.too_large",
                at,
                format!("limit={}", self.limits.max_total_bytes),
            ));
        }
        let id = SourceId::from_index(self.sources.len() as u32);
        self.total_bytes = next_total;
        self.labels.insert(label.clone(), id);
        self.sources.push(SourceEntry { id, label, origin });
        Ok(id)
    }

    fn load(
        &mut self,
        source_id: SourceId,
        text: &str,
        depth: usize,
    ) -> Result<Pattern, PatternError> {
        if depth > self.limits.max_depth {
            return Err(PatternError::new(
                "pattern.resolve.depth_exceeded",
                Position {
                    line: 1,
                    column: 1,
                    source: source_id,
                },
                format!("limit={}", self.limits.max_depth),
            ));
        }
        if self.stack.contains(&source_id) {
            return Err(PatternError::new(
                "pattern.resolve.cycle",
                Position {
                    line: 1,
                    column: 1,
                    source: source_id,
                },
                format!("source={}", source_id.index()),
            ));
        }
        self.stack.push(source_id);
        let tokens = lexer::scan_with(text, source_id)?;
        let mut cursor = Cursor::new(tokens);
        let pattern = grammar::pattern_resolving(&mut cursor, self, source_id, depth)?;
        self.stack.pop();
        if pattern
            .directives
            .iter()
            .any(|directive| directive.name == "once")
        {
            self.once.insert(source_id);
        }
        Ok(pattern)
    }

    pub(super) fn include(
        &mut self,
        kind: ImportKind,
        specifier: &str,
        from: SourceId,
        at: Position,
        depth: usize,
    ) -> Result<Pattern, PatternError> {
        let request = ResolveRequest {
            kind,
            specifier,
            from,
        };
        let resolved = self
            .resolver
            .resolve(&request)
            .map_err(|error| PatternError::new(error.code(), at, error.detail().to_string()))?;
        if let Some(existing) = self.labels.get(&resolved.label).copied() {
            if self.once.contains(&existing) || !self.stack.contains(&existing) {
                return Ok(Pattern::default());
            }
            return Err(PatternError::new(
                "pattern.resolve.cycle",
                at,
                format!("label={}", resolved.label),
            ));
        }
        let origin = match kind {
            ImportKind::Include => SourceOrigin::Include {
                from,
                specifier: specifier.to_string(),
            },
            ImportKind::Import => SourceOrigin::Import {
                from,
                specifier: specifier.to_string(),
            },
        };
        let child_id = self.allocate(resolved.label, origin, resolved.text.len(), at)?;
        self.load(child_id, &resolved.text, depth + 1)
    }
}
