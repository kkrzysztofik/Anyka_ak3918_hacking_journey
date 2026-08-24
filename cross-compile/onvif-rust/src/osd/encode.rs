//! Glyph encoding for the vendor OSD library.
//!
//! `ak_osd_draw_str` takes `unsigned short` codes, not bytes.  The vendor's own
//! `asc_to_short` (platform/libapp/src/osd_ex/ak_osd_ex.c) defines the mapping:
//! a byte below 0x80 becomes its own value, and a GBK pair packs as
//! `(hi << 8) | lo`.  We only ever emit the first case — see `encode_glyphs`.

/// Maximum glyphs the daemon will accept in one `CMD_OSD_DRAW_STR`.
pub const MAX_GLYPHS: usize = 128;

/// ASCII space, used to erase the tail of a previously longer string.
const GLYPH_SPACE: u16 = 0x20;

/// Encode text into vendor glyph codes.
///
/// ASCII only. `/usr/local/ak_font_16.bin` is a GB2312 font with no Latin
/// diacritic glyphs, so accepting non-ASCII would render garbage rather than
/// the user's text. Rejecting it here is the honest behaviour.
pub fn encode_glyphs(text: &str) -> Result<Vec<u16>, String> {
    if text.is_empty() {
        return Err("OSD text must not be empty".to_string());
    }
    if !text.is_ascii() {
        return Err(format!(
            "OSD text must be ASCII: the camera font is GB2312 and has no glyph \
             for the non-ASCII characters in {text:?}"
        ));
    }
    if text.len() > MAX_GLYPHS {
        return Err(format!(
            "OSD text is {} characters, maximum is {MAX_GLYPHS}",
            text.len()
        ));
    }
    Ok(text.bytes().map(u16::from).collect())
}

/// Pad `glyphs` with spaces up to `previous_len`.
///
/// The daemon has no clean_str command, so a shrinking string would otherwise
/// leave the tail of its predecessor on screen. This mirrors what the vendor's
/// `osd_disp_stat` does.
pub fn pad_to_erase(mut glyphs: Vec<u16>, previous_len: usize) -> Vec<u16> {
    while glyphs.len() < previous_len.min(MAX_GLYPHS) {
        glyphs.push(GLYPH_SPACE);
    }
    glyphs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_glyphs_ascii_maps_to_char_codes() {
        assert_eq!(encode_glyphs("AB").unwrap(), vec![0x41, 0x42]);
    }

    #[test]
    fn test_encode_glyphs_rejects_non_ascii() {
        // The vendor font is GB2312 and has no Latin diacritics, so this is a
        // hardware limit, not a policy choice.
        let err = encode_glyphs("Ogród").unwrap_err();
        assert!(err.contains("ASCII"), "error should explain why: {err}");
    }

    #[test]
    fn test_encode_glyphs_rejects_empty() {
        assert!(encode_glyphs("").is_err());
    }

    #[test]
    fn test_pad_to_erase_appends_spaces() {
        // A shrinking string must overwrite the tail of the previous one,
        // because the daemon has no clean_str command.
        assert_eq!(pad_to_erase(vec![0x41], 3), vec![0x41, 0x20, 0x20]);
    }

    #[test]
    fn test_pad_to_erase_leaves_longer_string_alone() {
        assert_eq!(pad_to_erase(vec![0x41, 0x42], 1), vec![0x41, 0x42]);
    }
}
