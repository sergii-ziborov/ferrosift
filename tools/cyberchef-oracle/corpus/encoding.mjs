// Encoding families: round trips through each base-N and radix codec, with the
// decode side pinned on the reference encoder's own canonical output.
import {LENGTHS} from "./builder.mjs";

const B32 = "A-Z2-7=";
const B32HEX = "0-9A-V=";
const B45 = "0-9A-Z $%*+\\-./:";
const B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const B58R = "rpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2bcdeCg65jkm8oFqi1tuvAxyz";
const B64 = "A-Za-z0-9+/=";
const B64URL = "A-Za-z0-9-_";
const B85 = "!-u";
const B85Z = "0-9a-zA-Z.\\-:+=^!/*?&<>()[]{}@%$#";

export async function add({addCase, bakeString, encodeDecodePair, randomAscii}) {
    await encodeDecodePair(
        "base32",
        {op: "To Base32", args: [B32]},
        {op: "From Base32", args: [B32, true]},
    );
    await encodeDecodePair(
        "base32hex",
        {op: "To Base32", args: [B32HEX]},
        {op: "From Base32", args: [B32HEX, true]},
    );
    await encodeDecodePair(
        "base45",
        {op: "To Base45", args: [B45]},
        {op: "From Base45", args: [B45, true]},
    );
    await encodeDecodePair(
        "base58",
        {op: "To Base58", args: [B58]},
        {op: "From Base58", args: [B58, true]},
    );
    await encodeDecodePair(
        "base58ripple",
        {op: "To Base58", args: [B58R]},
        {op: "From Base58", args: [B58R, true]},
    );
    await encodeDecodePair(
        "base64",
        {op: "To Base64", args: [B64]},
        {op: "From Base64", args: [B64, true, false]},
    );
    await encodeDecodePair(
        "base64url",
        {op: "To Base64", args: [B64URL]},
        {op: "From Base64", args: [B64URL, true, false]},
    );
    await encodeDecodePair(
        "base85",
        {op: "To Base85", args: [B85, false]},
        {op: "From Base85", args: [B85, true, "z"]},
    );
    await encodeDecodePair(
        "base85z85",
        {op: "To Base85", args: [B85Z, false]},
        // Z85 contains "z", so the zero-group character must be empty to avoid the
        // alphabet-conflict rejection (which FerroSift reproduces identically).
        {op: "From Base85", args: [B85Z, true, ""]},
    );
    await encodeDecodePair(
        "hex",
        {op: "To Hex", args: ["Space", 0]},
        {op: "From Hex", args: ["Auto"]},
    );
    await encodeDecodePair(
        "hex0x",
        {op: "To Hex", args: ["0x with comma", 0]},
        {op: "From Hex", args: ["0x with comma"]},
    );
    await encodeDecodePair(
        "binary",
        {op: "To Binary", args: ["Space", 8]},
        {op: "From Binary", args: ["Space", 8]},
    );
    await encodeDecodePair(
        "decimal",
        {op: "To Decimal", args: ["Space", false]},
        {op: "From Decimal", args: ["Space", false]},
    );
    await encodeDecodePair(
        "octal",
        {op: "To Octal", args: ["Space"]},
        {op: "From Octal", args: ["Space"]},
    );
    // To Charcode is text-to-text (not byte input), so sample printable text.
    for (const length of LENGTHS) {
        const raw = randomAscii(length).toString("latin1");
        const encoder = {op: "To Charcode", args: ["Space", 16]};
        addCase(`charcode_encode_${length}`, raw, [encoder]);
        const encoded = await bakeString(raw, [encoder]);
        addCase(`charcode_decode_${length}`, encoded, [
            {op: "From Charcode", args: ["Space", 16]},
        ]);
    }
}
