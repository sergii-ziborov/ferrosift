use alloc::string::String;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// The nineteen flags a permission string can carry.
///
/// Kept as one flat struct rather than nested per-class groups because the
/// textual and octal forms both address the bits in a fixed order, and the
/// output renders them in a third order again. Grouping would mean three
/// different traversals of a shape that suits none of them.
#[expect(
    clippy::struct_excessive_bools,
    reason = "a UNIX mode is nineteen independent flags; a bitmask would hide which bit is which at every use"
)]
#[derive(Default, Clone, Copy)]
struct Perms {
    directory: bool,
    symlink: bool,
    named_pipe: bool,
    socket: bool,
    char_device: bool,
    block_device: bool,
    door: bool,
    sticky: bool,
    setuid: bool,
    setgid: bool,
    read_user: bool,
    write_user: bool,
    exec_user: bool,
    read_group: bool,
    write_group: bool,
    exec_group: bool,
    read_other: bool,
    write_other: bool,
    exec_other: bool,
}

/// Explains a permission string given in octal or textual form.
pub(super) fn parse_permissions(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let characters: alloc::vec::Vec<char> = input.chars().collect();
    let start = characters
        .iter()
        .position(|c| !is_js_space(*c))
        .unwrap_or(characters.len());
    let rest = &characters[start..];

    // Octal is tried first, so a string that could read as either is octal.
    // `0` alone is a valid octal permission and also nothing textual, which is
    // the only overlap worth worrying about.
    let (perms, textual) = if rest.first().is_some_and(|c| ('0'..='7').contains(c)) {
        (from_octal(rest), false)
    } else if rest.first().is_some_and(|c| is_textual(*c)) {
        (from_textual(rest), true)
    } else {
        return Err(failed("filesystem.permissions.unrecognised_format"));
    };

    context.ensure_active()?;
    Ok(render(&perms, textual))
}

/// Whether a character is whitespace to a JavaScript regular expression.
///
/// This is not `char::is_whitespace`: the two disagree on `U+0085` (which Rust
/// counts and JavaScript does not) and on `U+FEFF` (the reverse). Leading
/// whitespace decides whether a string is read as a permission at all, so the
/// disagreement would change the answer rather than just the trimming.
fn is_js_space(character: char) -> bool {
    matches!(
        character,
        '\u{9}'..='\u{d}'
            | ' '
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

/// Whether a character can appear in the textual form.
fn is_textual(character: char) -> bool {
    matches!(
        character,
        'd' | 'l' | 'p' | 'c' | 'b' | 'D' | 'r' | 'w' | 'x' | 's' | 'S' | 't' | 'T' | '-'
    )
}

/// Reads the octal form: up to four digits, high to low.
///
/// Four digits put the setuid, setgid and sticky bits in the first; fewer
/// leave those clear and start at the user digit. A fifth digit is simply not
/// consumed, so `75551` is read as `7555`.
fn from_octal(rest: &[char]) -> Perms {
    let digits: alloc::vec::Vec<u32> = rest
        .iter()
        .take_while(|c| ('0'..='7').contains(*c))
        .take(4)
        .filter_map(|c| c.to_digit(8))
        .collect();

    let (special, user, group, other) = if digits.len() == 4 {
        (digits[0], digits[1], digits[2], digits[3])
    } else {
        (
            0,
            digits.first().copied().unwrap_or(0),
            digits.get(1).copied().unwrap_or(0),
            digits.get(2).copied().unwrap_or(0),
        )
    };

    Perms {
        setuid: special >> 2 & 1 == 1,
        setgid: special >> 1 & 1 == 1,
        sticky: special & 1 == 1,
        read_user: user >> 2 & 1 == 1,
        write_user: user >> 1 & 1 == 1,
        exec_user: user & 1 == 1,
        read_group: group >> 2 & 1 == 1,
        write_group: group >> 1 & 1 == 1,
        exec_group: group & 1 == 1,
        read_other: other >> 2 & 1 == 1,
        write_other: other >> 1 & 1 == 1,
        exec_other: other & 1 == 1,
        ..Perms::default()
    }
}

/// Reads the textual form, position by position.
///
/// Every position is optional: a string shorter than ten characters simply
/// leaves the remaining flags clear rather than being rejected. That is why
/// each field is guarded on length instead of the whole form being validated
/// up front.
fn from_textual(rest: &[char]) -> Perms {
    let field: alloc::vec::Vec<char> = rest
        .iter()
        .copied()
        .take_while(|c| is_textual(*c))
        .take(10)
        .collect();
    let at = |index: usize| field.get(index).copied();
    let mut perms = Perms::default();

    match at(0) {
        Some('d') => perms.directory = true,
        Some('l') => perms.symlink = true,
        Some('p') => perms.named_pipe = true,
        Some('s') => perms.socket = true,
        Some('c') => perms.char_device = true,
        Some('b') => perms.block_device = true,
        Some('D') => perms.door = true,
        _ => {}
    }

    perms.read_user = at(1) == Some('r');
    perms.write_user = at(2) == Some('w');
    match at(3) {
        Some('x') => perms.exec_user = true,
        Some('s') => {
            perms.exec_user = true;
            perms.setuid = true;
        }
        Some('S') => perms.setuid = true,
        _ => {}
    }

    perms.read_group = at(4) == Some('r');
    perms.write_group = at(5) == Some('w');
    match at(6) {
        Some('x') => perms.exec_group = true,
        Some('s') => {
            perms.exec_group = true;
            perms.setgid = true;
        }
        Some('S') => perms.setgid = true,
        _ => {}
    }

    perms.read_other = at(7) == Some('r');
    perms.write_other = at(8) == Some('w');
    match at(9) {
        Some('x') => perms.exec_other = true,
        Some('t') => {
            perms.exec_other = true;
            perms.sticky = true;
        }
        Some('T') => perms.sticky = true,
        _ => {}
    }

    perms
}

/// The horizontal rule between every row of the permission matrix.
const RULE: &str = " +---------+-------+-------+-------+\n";

/// Builds the report.
///
/// The file-type line appears only for textual input. Octal carries no type
/// information, so naming one would be inventing it — the reference omits the
/// line rather than guessing "Regular file".
fn render(perms: &Perms, textual: bool) -> String {
    let mut output = String::new();
    output.push_str("Textual representation: ");
    push_textual(&mut output, perms);
    output.push_str("\nOctal representation:   ");
    push_octal(&mut output, perms);

    if textual {
        output.push_str("\nFile type: ");
        output.push_str(file_type(perms));
    }

    if perms.setuid {
        output.push_str("\nThe setuid flag is set");
    }
    if perms.setgid {
        output.push_str("\nThe setgid flag is set");
    }
    if perms.sticky {
        output.push_str("\nThe sticky bit is set");
    }

    output.push_str("\n\n");
    output.push_str(RULE);
    output.push_str(" |         | User  | Group | Other |\n");
    output.push_str(RULE);
    push_row(
        &mut output,
        "    Read",
        perms.read_user,
        perms.read_group,
        perms.read_other,
    );
    output.push_str(RULE);
    push_row(
        &mut output,
        "   Write",
        perms.write_user,
        perms.write_group,
        perms.write_other,
    );
    output.push_str(RULE);
    push_row(
        &mut output,
        " Execute",
        perms.exec_user,
        perms.exec_group,
        perms.exec_other,
    );
    output.push_str(" +---------+-------+-------+-------+");
    output
}

/// Appends one row of the permission matrix.
fn push_row(output: &mut String, label: &str, user: bool, group: bool, other: bool) {
    output.push_str(" |");
    output.push_str(label);
    output.push_str(" |");
    for granted in [user, group, other] {
        output.push_str("   ");
        output.push(if granted { 'X' } else { ' ' });
        output.push_str("   |");
    }
    output.push('\n');
}

/// The three permission classes, in the order the textual form writes them.
#[derive(Clone, Copy)]
enum Class {
    User,
    Group,
    Other,
}

/// Appends the ten-character textual form.
///
/// The leading character is chosen by the *last* type flag that is set, which
/// is what a chain of plain assignments does in the reference. Only one is
/// ever set by the parser, so the order matters solely for agreement rather
/// than for any input that occurs.
fn push_textual(output: &mut String, perms: &Perms) {
    output.push(match () {
        () if perms.door => 'D',
        () if perms.block_device => 'b',
        () if perms.char_device => 'c',
        () if perms.socket => 's',
        () if perms.named_pipe => 'p',
        () if perms.symlink => 'l',
        () if perms.directory => 'd',
        () => '-',
    });
    for class in [Class::User, Class::Group, Class::Other] {
        push_triple(output, perms, class);
    }
}

/// Appends one read/write/execute triple.
///
/// The execute slot carries two bits: whether execute is granted and whether
/// the class's special bit is set. The special bit shows in upper case when
/// execute is absent, which is how `chmod` reports a flag that cannot take
/// effect. Only the "other" class spells that pair `t` and `T`; the first two
/// use `s` and `S`.
fn push_triple(output: &mut String, perms: &Perms, class: Class) {
    let (read, write, execute, special, both, special_only) = match class {
        Class::User => (
            perms.read_user,
            perms.write_user,
            perms.exec_user,
            perms.setuid,
            's',
            'S',
        ),
        Class::Group => (
            perms.read_group,
            perms.write_group,
            perms.exec_group,
            perms.setgid,
            's',
            'S',
        ),
        Class::Other => (
            perms.read_other,
            perms.write_other,
            perms.exec_other,
            perms.sticky,
            't',
            'T',
        ),
    };
    output.push(if read { 'r' } else { '-' });
    output.push(if write { 'w' } else { '-' });
    output.push(match (execute, special) {
        (true, true) => both,
        (false, true) => special_only,
        (true, false) => 'x',
        (false, false) => '-',
    });
}

/// Appends the four-digit octal form, special bits first.
fn push_octal(output: &mut String, perms: &Perms) {
    let weigh = |four: bool, two: bool, one: bool| {
        u32::from(four) * 4 + u32::from(two) * 2 + u32::from(one)
    };
    for digit in [
        weigh(perms.setuid, perms.setgid, perms.sticky),
        weigh(perms.read_user, perms.write_user, perms.exec_user),
        weigh(perms.read_group, perms.write_group, perms.exec_group),
        weigh(perms.read_other, perms.write_other, perms.exec_other),
    ] {
        output.push(char::from_digit(digit, 8).unwrap_or('0'));
    }
}

/// Names the file type the leading character stands for.
fn file_type(perms: &Perms) -> &'static str {
    match () {
        () if perms.directory => "Directory",
        () if perms.symlink => "Symbolic link",
        () if perms.named_pipe => "Named pipe",
        () if perms.socket => "Socket",
        () if perms.char_device => "Character device",
        () if perms.block_device => "Block device",
        () if perms.door => "Door",
        () => "Regular file",
    }
}
