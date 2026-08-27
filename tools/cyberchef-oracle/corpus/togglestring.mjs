// `Utils.convertToByteArray`, pinned through the operation that shows it whole.
//
// Every toggleString field in the catalog -- keys, IVs, salts, passphrases,
// authenticated data -- is read by this one function, so a misreading is wrong
// in ten operations at once. XOR is the probe because its output is the key
// bytes themselves: XOR a run of zeros with the field and the ciphertext *is*
// the repeated key, so any difference in how the field was read is visible in
// full rather than diffused through a cipher.
//
// The cases are chosen where a plausible port differs from the reference, and
// each of the six branches has at least one. What they establish:
//
//   * Hex is permissive, not strict. It splits on anything that is not a hex
//     digit and reads each run two at a time, so an odd digit is a byte and
//     `zz` is empty rather than an error.
//   * Latin1 -- and any unrecognised option name -- is `strToByteArray`, which
//     falls back to UTF-8 when a character does not fit in a byte. A CJK key is
//     its UTF-8 bytes, not its code points masked to eight bits.
//   * Binary removes whitespace and then chunks the remainder, running across
//     the gaps rather than restarting at them.
//   * Decimal splits on runs of anything that is not a digit or a minus.
//   * Base64 strips what is not in the alphabet instead of refusing it.
//
// Sixteen zero bytes, so the ciphertext is the key repeated. A key longer than
// this would be cut off before it repeated, which is the one thing these cases
// are not about.
const PROBE = Buffer.alloc(16);

const KEYS = [
    // Hex: the ordinary reading, then the four that are not.
    {option: "Hex", string: "00112233"},
    {option: "Hex", string: "0 1 2 3"},
    {option: "Hex", string: "abc"},
    {option: "Hex", string: "0x41 0x42"},
    {option: "Hex", string: "de:ad:be:ef"},
    {option: "Hex", string: "41-42-43"},

    // UTF8 is UTF-8 whatever the character.
    {option: "UTF8", string: "key"},
    {option: "UTF8", string: "é"},
    {option: "UTF8", string: "日本"},

    // Latin1 is *not* the code points masked to a byte: past the byte range it
    // is the whole string's UTF-8 encoding instead.
    {option: "Latin1", string: "key"},
    {option: "Latin1", string: "é"},
    {option: "Latin1", string: "日本"},
    {option: "Latin1", string: "aÿz"},

    // Base64, padded and not, and with characters outside the alphabet.
    {option: "Base64", string: "QUJD"},
    {option: "Base64", string: "QUJD=="},
    {option: "Base64", string: "QU"},
    {option: "Base64", string: "Q U J D"},
    {option: "Base64", string: "!QUJD!"},

    // Binary, spaced and not, and with the gaps in the wrong places.
    {option: "Binary", string: "01000001 01000010"},
    {option: "Binary", string: "0100000101000010"},
    {option: "Binary", string: "0100 000101000010"},

    // Decimal, with each separator the split regex accepts.
    {option: "Decimal", string: "1 2 3"},
    {option: "Decimal", string: "1,2,3"},
    {option: "Decimal", string: "65;66;67"},
    {option: "Decimal", string: "255 0 128"},
];

/**
 * Whether the field's characters all fit in a byte.
 *
 * Only the byte-string reading cares. It hands the string to the consumer
 * untouched, and `crypto-api`'s hasher packs four characters into a
 * thirty-two bit word with `charCodeAt(i) << 24 | charCodeAt(i+1) << 16 | ...`
 * and no mask -- so a character above two hundred and fifty-five spills its
 * high byte into the *previous* byte's position in the word. The digest that
 * comes out is not the HMAC of any key at all, and no byte-oriented
 * implementation produces it. Those fields are pinned through XOR only, and
 * the divergence is recorded in docs/compatibility/cyberchef-v11.3.0.md.
 */
const fitsInBytes = key =>
    key.option !== "Latin1" || [...key.string].every(one => one.codePointAt(0) < 256);

export async function add({addCase}) {
    let index = 0;
    for (const key of KEYS) {
        addCase(`togglestring_${index}`, PROBE, [
            {op: "XOR", args: [key, "Standard", false]},
        ]);
        index += 1;
    }

    // The same fields, read the other way. HMAC calls `convertToByteString`
    // where XOR calls `convertToByteArray`, so this arm is what stops the two
    // readings being collapsed into one: the Latin1 cases below deliberately
    // disagree with the XOR cases above.
    index = 0;
    for (const key of KEYS) {
        if (fitsInBytes(key)) {
            addCase(`togglestring_hmac_${index}`, "message", [
                {op: "HMAC", args: [key, "SHA256"]},
            ]);
        }
        index += 1;
    }
}
