//! IEEE CRC-32 used by the gzip footer.

/// Computes the ISO-HDLC / gzip CRC-32 of `data`.
pub(crate) fn crc32(data: &[u8]) -> u32 {
    crc32_finish(crc32_update(0xffff_ffff_u32, data))
}

/// Continues a gzip-style CRC-32 from an intermediate state.
///
/// Start with `0xffff_ffff`, feed every uncompressed byte, then
/// [`crc32_finish`].
pub(crate) fn crc32_update(mut crc: u32, data: &[u8]) -> u32 {
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    crc
}

/// Finalises a CRC-32 started with `0xffff_ffff` and updated with
/// [`crc32_update`].
pub(crate) const fn crc32_finish(crc: u32) -> u32 {
    !crc
}
