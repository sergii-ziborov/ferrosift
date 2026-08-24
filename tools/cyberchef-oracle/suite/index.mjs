// The curated case list, assembled from one module per operation family.
//
// Order is part of the fixture: cases are baked and written in this sequence,
// so appending to a family appends within its block rather than reshuffling
// what is already pinned.
import {archiveCases} from "./archive.mjs";
import {byteCases} from "./bytes.mjs";
import {cryptoCases} from "./crypto.mjs";
import {digestCases} from "./digest.mjs";
import {encodingCases} from "./encoding.mjs";
import {textCases} from "./text.mjs";

export const curatedCases = [
    ...encodingCases,
    ...byteCases,
    ...digestCases,
    ...textCases,
    ...cryptoCases,
    ...archiveCases,
];

/**
 * A recipe the importer must reject rather than silently accept. Kept beside
 * the supported cases so that "we do not implement this" is pinned with the
 * same rigour as "we implement this exactly".
 */
export const unsupportedCase = {
    name: "magic_is_explicitly_unsupported",
    recipe: [{op: "Magic", args: []}],
    finding: {
        code: "compat.cyberchef.unknown_operation",
        source_step: 0,
        original_operation: "Magic",
    },
};
