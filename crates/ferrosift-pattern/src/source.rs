//! Caller-supplied pattern sources for `import` / `#include`.
//!
//! The pattern crate stays free of an ambient filesystem. A host that may read
//! files — CLI, MCP, an application — supplies a [`PatternResolver`]. Limits
//! keep a malicious or cyclic graph from exhausting memory.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use core::fmt;

use crate::ast::{Declaration, Pattern};
use crate::error::{PatternError, Position};

/// How another source was named in the pattern text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportKind {
    /// `#include <…>` or `#include "…"`.
    Include,
    /// `import a.b.c;`.
    Import,
}

/// One resolve request issued while loading a pattern graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolveRequest<'a> {
    /// Whether the site was `#include` or `import`.
    pub kind: ImportKind,
    /// Specifier as written after normalisation (`std/mem.pat`, `std.io`).
    pub specifier: &'a str,
    /// Source that contained the import site.
    pub from: crate::error::SourceId,
}

/// Text returned by a resolver for one specifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSource {
    /// Stable label used for cycle detection and `#pragma once`.
    ///
    /// Distinct from the specifier when a host canonicalises paths. Two
    /// requests that resolve to the same label are the same source.
    pub label: String,
    /// UTF-8 pattern text.
    pub text: String,
}

/// Why a resolver or the load ceilings refused a source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolveError {
    code: &'static str,
    detail: String,
}

impl ResolveError {
    /// Creates a machine-readable resolve failure.
    #[must_use]
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    /// Stable code such as `pattern.resolve.not_found`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Human detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

/// Ceilings applied while expanding imports and includes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolveLimits {
    /// Maximum nesting depth of import/include edges, counting the root as 0.
    pub max_depth: usize,
    /// Maximum distinct resolved sources in one graph.
    pub max_sources: usize,
    /// Maximum total UTF-8 bytes across every loaded source.
    pub max_total_bytes: usize,
}

impl Default for ResolveLimits {
    fn default() -> Self {
        Self {
            max_depth: 8,
            max_sources: 32,
            max_total_bytes: 1_048_576,
        }
    }
}

/// Supplies pattern text for `import` / `#include` without an ambient filesystem.
pub trait PatternResolver {
    /// Resolves one specifier to labelled source text.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] when the specifier is unknown or refused.
    fn resolve(&self, request: &ResolveRequest<'_>) -> Result<ResolvedSource, ResolveError>;
}

/// In-memory resolver for tests and hosts that have already loaded libraries.
#[derive(Clone, Debug, Default)]
pub struct MapResolver {
    by_specifier: BTreeMap<String, ResolvedSource>,
}

impl MapResolver {
    /// Empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `specifier` → `text`, using `specifier` as the stable label.
    pub fn insert(&mut self, specifier: impl Into<String>, text: impl Into<String>) -> &mut Self {
        let specifier = specifier.into();
        self.by_specifier.insert(
            specifier.clone(),
            ResolvedSource {
                label: specifier,
                text: text.into(),
            },
        );
        self
    }

    /// Registers a specifier with an explicit cycle-identity label.
    pub fn insert_labelled(
        &mut self,
        specifier: impl Into<String>,
        label: impl Into<String>,
        text: impl Into<String>,
    ) -> &mut Self {
        self.by_specifier.insert(
            specifier.into(),
            ResolvedSource {
                label: label.into(),
                text: text.into(),
            },
        );
        self
    }
}

impl PatternResolver for MapResolver {
    fn resolve(&self, request: &ResolveRequest<'_>) -> Result<ResolvedSource, ResolveError> {
        self.by_specifier
            .get(request.specifier)
            .cloned()
            .ok_or_else(|| {
                ResolveError::new(
                    "pattern.resolve.not_found",
                    format!(
                        "{} `{}` is not in the resolver map",
                        kind_label(request.kind),
                        request.specifier
                    ),
                )
            })
    }
}

pub(crate) fn merge_child(
    into: &mut Pattern,
    child: Pattern,
    at: Position,
) -> Result<(), PatternError> {
    let mut names: BTreeSet<String> = into.declarations.iter().map(declared_name).collect();
    for declaration in child.declarations {
        let name = declared_name(&declaration);
        if !names.insert(name.clone()) {
            return Err(PatternError::new(
                "pattern.parse.duplicate_declaration",
                at,
                format!("`{name}` is declared more than once"),
            ));
        }
        into.declarations.push(declaration);
    }
    into.directives.extend(child.directives);
    if into.endian.is_none() {
        into.endian = child.endian;
    }
    Ok(())
}

fn declared_name(declaration: &Declaration) -> String {
    match declaration {
        Declaration::Struct(value) => value.name.clone(),
        Declaration::Union(value) => value.name.clone(),
        Declaration::Enum(value) => value.name.clone(),
        Declaration::Bitfield(value) => value.name.clone(),
        Declaration::Alias(value) => value.name.clone(),
        Declaration::Placement(value) => value.name.clone(),
    }
}

fn kind_label(kind: ImportKind) -> &'static str {
    match kind {
        ImportKind::Include => "include",
        ImportKind::Import => "import",
    }
}

/// Normalises a `#include` argument into a specifier.
pub(crate) fn include_specifier(argument: &str) -> Result<String, String> {
    let trimmed = argument.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'<' && bytes[trimmed.len() - 1] == b'>')
            || (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
        {
            return Ok(String::from(&trimmed[1..trimmed.len() - 1]));
        }
    }
    if trimmed.is_empty() {
        return Err(String::from("#include requires a path"));
    }
    Ok(String::from(trimmed))
}
