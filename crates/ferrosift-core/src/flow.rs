//! What a step tells the executor to do next.
//!
//! Until this existed the executor had a cursor rather than a program counter:
//! `index += 1` was the only way forward and the only way anywhere. That was
//! enough for every operation that transforms a value, and it is why `Label`
//! shipped as a pass-through naming a place nothing could jump to.
//!
//! A [`FlowDirective`] is the one thing an operation can say about *control*
//! rather than about the value. Everything in the catalog says [`Next`] by
//! default; the four flow-control operations say something else, and the
//! executor is the only thing that acts on it.
//!
//! The split matters for one concrete reason: `Conditional Jump` and
//! `Subsection` decide with a regular expression, and the regex engine lives in
//! `ferrosift-operations` behind a feature. Letting the *operation* answer
//! "jump" or "here are the sections" keeps the engine there and keeps
//! `ferrosift-core` free of it — the executor never parses a pattern, it only
//! moves a counter and slices a string at offsets it was handed.
//!
//! [`Next`]: FlowDirective::Next

use alloc::{string::String, vec::Vec};

/// Where execution goes after a step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowDirective {
    /// The following step, with the jump allowance untouched.
    ///
    /// Every operation that transforms a value, and the one conditional case
    /// the reference leaves alone: a `Conditional Jump` with an empty pattern
    /// tests nothing and so neither spends nor refunds the allowance.
    Next,
    /// The following step, with the jump allowance cleared.
    ///
    /// A jump that was offered and not taken. The reference keeps a single
    /// counter for the whole recipe and zeroes it whenever a jump does not
    /// fire, so a loop that exits and is entered again gets its full allowance
    /// back. That is observable when two jumps share the counter, which is why
    /// it is a separate answer rather than the same one as [`Self::Next`].
    NotTaken,
    /// The step after the first `Label` named `label`, within this region.
    ///
    /// Refused — and treated as [`Self::NotTaken`] — when no such label exists
    /// or when `max_jumps` have already been taken. Both refusals are the
    /// reference's, and `max_jumps` is the reason `Jump` has a second argument
    /// at all: a backward jump is a loop, and a loop needs a bound.
    ///
    /// The bound is not the only one. Every step through this one counts
    /// against [`ExecutionBudget::max_operation_invocations`], so a recipe with
    /// many jump sites each willing to fire a thousand times still stops.
    ///
    /// [`ExecutionBudget::max_operation_invocations`]: crate::ExecutionBudget::max_operation_invocations
    Jump {
        /// Name carried by the `Label` to resume at.
        label: String,
        /// How many jumps the recipe may take before this one stops firing.
        max_jumps: u32,
    },
    /// Nothing further. The run ends here and answers with the value in hand.
    ///
    /// Inside a Fork branch or a Subsection tranche this ends that branch
    /// rather than the run, because the reference runs each one as its own
    /// recipe and a `Return` there returns from that recipe.
    Stop,
    /// Run the steps up to the matching `Merge` on each span separately.
    ///
    /// The spans are byte ranges into the text the step was given, so the
    /// executor slices without knowing what selected them. An empty list is
    /// not the same as no directive: it means the pattern was valid and
    /// matched nothing, and the reference then skips the region entirely.
    Sections {
        /// Ranges to run the region on, in order, non-overlapping.
        spans: Vec<Section>,
        /// When true, a tranche that fails contributes nothing instead of
        /// aborting the run.
        ignore_errors: bool,
    },
}

/// A half-open byte range of the value a section covers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Section {
    /// First byte of the section.
    pub start: usize,
    /// One past the last byte of the section.
    pub end: usize,
}

impl Section {
    /// Creates a section covering `start..end`.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Whether the range is ordered and lies inside `len` bytes.
    #[must_use]
    pub const fn fits(&self, len: usize) -> bool {
        self.start <= self.end && self.end <= len
    }
}
