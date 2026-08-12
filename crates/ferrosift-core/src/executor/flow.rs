//! Fork / Merge flow-control helpers for the linear executor.

use ferrosift_model::OperationId;

/// Canonical id of the Fork operation.
pub const FORK_ID: &str = "flow.fork@1";
/// Canonical id of the Merge operation.
pub const MERGE_ID: &str = "flow.merge@1";

pub(super) fn is_fork(id: &OperationId) -> bool {
    id.as_str() == FORK_ID
}

pub(super) fn is_merge(id: &OperationId) -> bool {
    id.as_str() == MERGE_ID
}

/// Finds the Merge that closes the Fork at `fork_index`.
///
/// Nested Fork/Subsection-style nesting is counted the same way as `CyberChef`:
/// each nested Fork increments depth, each Merge decrements it, and the
/// matching Merge for this Fork is the first Merge that returns depth to zero
/// (or any Merge whose `merge_all` argument is true).
pub(super) fn find_merge_index(
    fork_index: usize,
    ids: &[OperationId],
    merge_all_flags: &[bool],
    disabled: &[bool],
) -> Option<usize> {
    let mut depth = 1_i32;
    let mut index = fork_index + 1;
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
        } else if is_fork(&ids[index]) {
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
        assert_eq!(
            find_merge_index(0, &ids, &merge_all, &disabled),
            Some(5)
        );
        assert_eq!(
            find_merge_index(2, &ids, &merge_all, &disabled),
            Some(4)
        );
    }

    #[test]
    fn parse_delimiter_handles_newline_escape() {
        assert_eq!(parse_delimiter("\\n"), "\n");
        assert_eq!(parse_delimiter("\n"), "\n");
        assert_eq!(parse_delimiter("|"), "|");
    }
}
