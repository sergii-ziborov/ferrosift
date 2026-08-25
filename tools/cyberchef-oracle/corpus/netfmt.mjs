// Parity bits and MAC address reformatting.

// Parity is only interesting at the edges: an empty input short-circuits
// before the delimiter is consulted, a delimiter makes each field carry its
// own bit, and any character that is not 0, 1, space, or the delimiter is an
// error. The reference compares against the delimiter *argument*, so a
// multi-character delimiter only excuses its own first character — a quirk
// worth pinning so nobody "fixes" it later.
const PARITY_INPUTS = [
    "1010",
    "1011",
    "0000",
    "1111",
    "1",
    "0",
    "",
    "1010 1100",
];

const MAC_INPUTS = [
    "00:11:22:33:44:55",
    "00-11-22-33-44-55",
    "001122334455",
    "0011.2233.4455",
    // Several at once, comma and whitespace separated, which the reference
    // splits with one regex.
    "00:11:22:33:44:55, aa-bb-cc-dd-ee-ff",
    "00:11:22:33:44:55\naabbccddeeff",
    "AA:BB:CC:DD:EE:FF",
    "",
    // Not a MAC at all: the reference reformats whatever it is given.
    "hello",
];

export async function add({addCase}) {
    for (const [index, value] of PARITY_INPUTS.entries()) {
        for (const mode of ["Even Parity", "Odd Parity"]) {
            for (const position of ["Start", "End"]) {
                const label = `${mode[0]}${position[0]}`.toLowerCase();
                addCase(`parity_${label}_${index}`, value, [
                    {op: "Parity Bit", args: [mode, position, "Encode", ""]},
                ]);
            }
        }
        // Round trip: encoding then decoding must give the input back.
        addCase(`parity_round_trip_${index}`, value, [
            {op: "Parity Bit", args: ["Even Parity", "End", "Encode", ""]},
            {op: "Parity Bit", args: ["Even Parity", "End", "Decode", ""]},
        ]);
    }

    // Delimited fields, each carrying its own bit.
    for (const [index, value] of ["1010 1100 0011", "1,0,1", "11-00-11"].entries()) {
        const delimiter = [" ", ",", "-"][index];
        addCase(`parity_delimited_${index}`, value, [
            {op: "Parity Bit", args: ["Even Parity", "End", "Encode", delimiter]},
        ]);
        addCase(`parity_delimited_decode_${index}`, value, [
            {op: "Parity Bit", args: ["Even Parity", "End", "Decode", delimiter]},
        ]);
    }

    for (const [index, value] of MAC_INPUTS.entries()) {
        // Defaults: both cases, three delimiter styles.
        addCase(`mac_default_${index}`, value, [
            {op: "Format MAC addresses", args: ["Both", true, true, true, false, false]},
        ]);
        for (const outputCase of ["Upper only", "Lower only"]) {
            addCase(`mac_${outputCase.split(" ")[0].toLowerCase()}_${index}`, value, [
                {op: "Format MAC addresses", args: [outputCase, true, true, true, false, false]},
            ]);
        }
        // Cisco style and the EUI-64 interface identifier, which flips a bit
        // in the first octet and is the easiest part to get wrong.
        addCase(`mac_cisco_${index}`, value, [
            {op: "Format MAC addresses", args: ["Lower only", false, false, false, true, false]},
        ]);
        addCase(`mac_ipv6_${index}`, value, [
            {op: "Format MAC addresses", args: ["Lower only", false, false, false, false, true]},
        ]);
        // Everything on at once.
        addCase(`mac_all_${index}`, value, [
            {op: "Format MAC addresses", args: ["Both", true, true, true, true, true]},
        ]);
        // Nothing on: the group separator is all that survives.
        addCase(`mac_none_${index}`, value, [
            {op: "Format MAC addresses", args: ["Both", false, false, false, false, false]},
        ]);
    }
}
