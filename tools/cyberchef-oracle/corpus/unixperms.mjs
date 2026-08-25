// Explaining UNIX permission strings.
//
// Two input forms reach the same report, and they disagree in one visible way:
// the file-type line appears for textual input only, because octal carries no
// type information. Both forms are sampled so that asymmetry is pinned rather
// than assumed.
//
// The special bits are sampled in both cases. `s` and `t` mean the flag is set
// and execute is granted; `S` and `T` mean the flag is set while execute is
// not, which is how a flag that cannot take effect is reported. A port that
// treated the pair as one bit agrees on every ordinary mode and diverges on
// exactly these.
//
// `75551` is here for the truncation: the octal form reads at most four
// digits, so the trailing `1` is not consumed rather than making the input
// invalid.

const SAMPLES = [
    "755",
    "644",
    "777",
    "000",
    "0755",
    "4755",
    "2755",
    "1755",
    "7777",
    "7",
    "75",
    "75551",
    "  644  ",
    "drwxr-xr-x",
    "-rw-r--r--",
    "lrwxrwxrwx",
    "srwxrwxrwx",
    "prw-rw----",
    "crw-rw-rw-",
    "brw-rw----",
    "Drwxr-xr-x",
    "-rwsr-xr-x",
    "-rwSr-xr-x",
    "-rwxr-sr-x",
    "-rwxr-Sr-x",
    "drwxrwxrwt",
    "drwxrwxrwT",
    "-rwsr-sr-t",
    "-rwSr-Sr-T",
    "drwx",
    "d",
    "----------",
];

export function add({addCase}) {
    for (const [index, sample] of SAMPLES.entries()) {
        addCase(`unix_perms_${index}`, sample, [
            {op: "Parse UNIX file permissions", args: []},
        ]);
    }
}
