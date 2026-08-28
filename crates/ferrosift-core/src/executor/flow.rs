//! Flow-control operation identity for the region interpreter.
//!
//! The executor recognises six operations by canonical id. Four of them open
//! or close a region (`Fork`, `Subsection`, `Merge`) or name a place in one
//! (`Label`); the rest transfer control and are recognised only so that
//! `docs/` and the compatibility layer can name them. Everything an operation
//! actually *decides* travels through
//! [`FlowDirective`](crate::FlowDirective) instead, which is why this file
//! holds strings and no behaviour.

use ferrosift_model::OperationId;

/// Canonical id of the Fork operation.
pub const FORK_ID: &str = "flow.fork@1";
/// Canonical id of the Merge operation.
pub const MERGE_ID: &str = "flow.merge@1";
/// Canonical id of the Label operation, the destination of a jump.
pub const LABEL_ID: &str = "flow.label@1";
/// Canonical id of the Jump operation.
pub const JUMP_ID: &str = "flow.jump@1";
/// Canonical id of the Conditional Jump operation.
pub const CONDITIONAL_JUMP_ID: &str = "flow.conditional_jump@1";
/// Canonical id of the Return operation.
pub const RETURN_ID: &str = "flow.return@1";
/// Canonical id of the Subsection operation.
pub const SUBSECTION_ID: &str = "flow.subsection@1";

pub(super) fn is_fork(id: &OperationId) -> bool {
    id.as_str() == FORK_ID
}

pub(super) fn is_merge(id: &OperationId) -> bool {
    id.as_str() == MERGE_ID
}

pub(super) fn is_label(id: &OperationId) -> bool {
    id.as_str() == LABEL_ID
}

pub(super) fn is_subsection(id: &OperationId) -> bool {
    id.as_str() == SUBSECTION_ID
}

/// Whether this operation opens a region the executor drives itself.
///
/// Fork maps its body over branches, Subsection over matched spans. They differ
/// in what they iterate and agree in everything else: both run `[start, merge)`
/// once per item, both nest, and both are closed by the same Merge.
pub(super) fn opens_region(id: &OperationId) -> bool {
    is_fork(id) || is_subsection(id)
}

/// Finds the Merge that closes the region opened at `open_index`.
///
/// Nesting is counted the same way as `CyberChef`: each nested Fork or
/// Subsection increments depth, each Merge decrements it, and the matching
/// Merge for this region is the first Merge that returns depth to zero (or any
/// Merge whose `merge_all` argument is true).
pub(super) fn find_merge_index(
    open_index: usize,
    ids: &[OperationId],
    merge_all_flags: &[bool],
    disabled: &[bool],
) -> Option<usize> {
    let mut depth = 1_i32;
    let mut index = open_index + 1;
    while index < ids.len() {
        if disabled[index] {
            index += 1;
            continue;
        }
        if is_merge(&ids[index]) {
            depth -= 1;
            if depth == 0 || merge_all_flags[index] {
                return Some(index);
            }
        } else if opens_region(&ids[index]) {
            depth += 1;
        }
        index += 1;
    }
    None
}

/// Parses CyberChef-style short binary string escapes used by Fork delimiters.
pub(super) fn parse_delimiter(input: &str) -> alloc::string::String {
    let mut output = alloc::string::String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(value) = chars.next() {
        if value != '\\' {
            output.push(value);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('\\') | None => output.push('\\'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrosift_model::OperationId;

    fn id(value: &str) -> OperationId {
        OperationId::new(value).expect("valid id")
    }

    #[test]
    fn finds_matching_merge_with_nesting() {
        let ids = [
            id(FORK_ID),
            id("encoding.hex.decode@1"),
            id(FORK_ID),
            id("encoding.base64.decode@1"),
            id(MERGE_ID),
            id(MERGE_ID),
        ];
        let merge_all = [false; 6];
        let disabled = [false; 6];
        assert_eq!(find_merge_index(0, &ids, &merge_all, &disabled), Some(5));
        assert_eq!(find_merge_index(2, &ids, &merge_all, &disabled), Some(4));
    }

    #[test]
    fn a_nested_subsection_takes_the_inner_merge() {
        // The reference counts Subsection alongside Fork when it walks forward
        // looking for its own Merge. Counting only Fork would have handed the
        // outer region the inner Merge and run the rest of the recipe inside it.
        let ids = [
            id(FORK_ID),
            id(SUBSECTION_ID),
            id("encoding.hex.encode@1"),
            id(MERGE_ID),
            id(MERGE_ID),
        ];
        let merge_all = [false; 5];
        let disabled = [false; 5];
        assert_eq!(find_merge_index(0, &ids, &merge_all, &disabled), Some(4));
        assert_eq!(find_merge_index(1, &ids, &merge_all, &disabled), Some(3));
    }

    #[test]
    fn parse_delimiter_handles_newline_escape() {
        assert_eq!(parse_delimiter("\\n"), "\n");
        assert_eq!(parse_delimiter("\n"), "\n");
        assert_eq!(parse_delimiter("|"), "|");
    }
}
