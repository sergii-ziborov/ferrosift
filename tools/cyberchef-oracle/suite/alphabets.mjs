// Alphabets and byte ranges shared by the curated cases.

/** CyberChef's default Base64 alphabet. */
export const standard = "A-Za-z0-9+/=";

/** The `crypt(3)` Base64 variant, which reorders the standard alphabet. */
export const crypt = "/128GhIoPQROSTeUbADfgHijKLM+n0pFWXY456xyzB7=39VaqrstJklmNuZvwcdEC";

/** Every byte value, hex-encoded: the round-trip cases must cover all 256. */
export const allBytes = Array.from({length: 256}, (_, value) => value)
    .map(value => value.toString(16).padStart(2, "0"))
    .join("");
