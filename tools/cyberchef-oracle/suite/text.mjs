// Text transforms, extraction, and defanging.

export const textCases = [
    {
        name: "html_entity_round_trip",
        input: {kind: "text", value: "a & b <c>"},
        recipe: [
            {op: "To HTML Entity", args: [false, "Named entities"]},
            {op: "From HTML Entity", args: []},
        ],
    },
    {
        name: "rot13_hello_world",
        input: {kind: "bytes", hex: "48656c6c6f2c20576f726c6421"},
        recipe: [{op: "ROT13", args: [true, true, false, 13]}],
    },
    {
        name: "charcode_round_trip",
        input: {kind: "text", value: "Hi"},
        recipe: [
            {op: "To Charcode", args: ["Space", 16]},
            {op: "From Charcode", args: ["Space", 16]},
        ],
    },
    // `From Charcode` splits an undelimited input into pairs when it is longer
    // than seventeen, and both the test and the split count *UTF-16 code
    // units*, because that is what a JavaScript string's length is. This port
    // counted UTF-8 bytes, which is the same number only for ASCII: for
    // anything else it split in the wrong places, and where a pair landed
    // inside a character it aborted rather than answering. `fuzz/decoders`
    // found it. Both cases below are longer than seventeen and neither is
    // ASCII, so the byte count and the code-unit count disagree.
    // The branch is reached by an input the delimiter does not split, so the
    // delimiter is an ordinary one and the input simply has none of it. `From
    // Charcode` offers only six delimiters and "Nothing" is not among them.
    {
        name: "from_charcode_undelimited_two_byte_characters",
        input: {kind: "text", value: "ˉˉˉˉˉˉˉˉˉˉ12345678"},
        recipe: [{op: "From Charcode", args: ["Space", 16]}],
    },
    {
        name: "from_charcode_undelimited_astral_characters",
        // Each of these is one character and two code units, so a pair-wise
        // split lands on a surrogate boundary rather than a character one.
        input: {kind: "text", value: "𝄞𝄞𝄞𝄞𝄞𝄞𝄞𝄞𝄞𝄞"},
        recipe: [{op: "From Charcode", args: ["Space", 16]}],
    },
    {
        name: "extract_ip_url_email",
        input: {
            kind: "text",
            value:
                "Contact admin@example.com or visit https://evil.example/path?x=1 see 8.8.8.8 and 192.168.1.1 also domain.example.org",
        },
        recipe: [
            {op: "Extract IP addresses", args: [true, false, false, false, false, false]},
        ],
    },
    {
        name: "extract_urls",
        input: {
            kind: "text",
            value:
                "Contact admin@example.com or visit https://evil.example/path?x=1 see 8.8.8.8",
        },
        recipe: [{op: "Extract URLs", args: [false, false, false]}],
    },
    {
        name: "extract_emails",
        input: {
            kind: "text",
            value: "Contact admin@example.com or visit https://evil.example/path",
        },
        recipe: [{op: "Extract email addresses", args: [false, false, false]}],
    },
    {
        name: "defang_and_fang_url",
        input: {kind: "text", value: "https://evil.example/path"},
        recipe: [
            {
                op: "Defang URL",
                args: [true, true, true, "Only full URLs"],
            },
            {op: "Fang URL", args: [true, true, true]},
        ],
    },
    {
        name: "defang_ip_addresses",
        input: {kind: "text", value: "8.8.8.8 and 1.2.3.4"},
        recipe: [{op: "Defang IP Addresses", args: []}],
    },
    {
        name: "strings_ascii_printable",
        input: {kind: "text", value: "\u0000\u0000Hello World\u0000\u0000test\u0000AB"},
        recipe: [
            {
                op: "Strings",
                args: ["Single byte", 4, "All printable chars (A)", false, false, false],
            },
        ],
    },
];
