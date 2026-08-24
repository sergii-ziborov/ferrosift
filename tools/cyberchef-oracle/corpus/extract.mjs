// Extractors and defanging, built from one fixed indicator corpus.
//
// The text is deliberately hand-written rather than sampled: extractors are
// judged on which spans they find, so the input must contain known positives
// and near-misses in known places.

const IOC_TEXT =
    "Contact admin@example.com or ops@corp.example.org, visit " +
    "https://evil.example/path?x=1 and http://a.b.example see 8.8.8.8, " +
    "192.168.1.1 and 2001:db8::1 with domain.example.org and " +
    "aa:bb:cc:dd:ee:ff plus AA-BB-CC-DD-EE-FF and path C:\\Windows\\cmd.exe " +
    "and /usr/bin/python3 and hash 0123456789abcdef0123456789abcdef01234567";

export function add({addCase}) {
    addCase("extract_ip", IOC_TEXT, [
        {op: "Extract IP addresses", args: [true, true, false, false, false, false]},
    ]);
    addCase("extract_urls", IOC_TEXT, [{op: "Extract URLs", args: [false, false, false]}]);
    addCase("extract_emails", IOC_TEXT, [
        {op: "Extract email addresses", args: [false, false, false]},
    ]);
    addCase("extract_domains", IOC_TEXT, [
        {op: "Extract domains", args: [false, false, false, false]},
    ]);
    addCase("extract_mac", IOC_TEXT, [{op: "Extract MAC addresses", args: [true, true, true]}]);
    addCase("extract_hashes", IOC_TEXT, [{op: "Extract hashes", args: [40, false, false]}]);
    addCase("extract_file_paths", IOC_TEXT, [
        {op: "Extract file paths", args: [true, true, false, false, false]},
    ]);
    addCase("defang_url", "https://evil.example/path?x=1", [
        {op: "Defang URL", args: [true, true, true, "Only full URLs"]},
    ]);
    addCase("defang_and_fang_url", "https://evil.example/a", [
        {op: "Defang URL", args: [true, true, true, "Only full URLs"]},
        {op: "Fang URL", args: [true, true, true]},
    ]);
    addCase("defang_ip", "8.8.8.8 and 1.2.3.4 and 2001:db8::1", [
        {op: "Defang IP Addresses", args: []},
    ]);
}
