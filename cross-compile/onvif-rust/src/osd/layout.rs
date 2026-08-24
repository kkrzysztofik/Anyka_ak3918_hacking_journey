//! OSD placement math.
//!
//! Ports the vendor layout idea from `osd_disp_name`, corrected by Stage B on
//! this camera: `ak_osd_init` doubles the font-file size on the main channel
//! (16 → 32), and ASCII advance is half of the per-channel font height.
//! Pure functions only — fully testable without hardware.

use serde::{Deserialize, Serialize};

/// Font-file size on disk (`/usr/local/ak_font_16.bin`).
pub const FONT_FILE_SIZE: i32 = 16;

/// Which corner an OSD sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Corner {
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight,
}

/// Per-channel glyph metrics after `ak_osd_init`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontMetrics {
    /// Glyph cell height in pixels (also the left-edge inset).
    pub height: i32,
    /// Horizontal advance per ASCII glyph (`height / 2`).
    pub advance: i32,
}

impl FontMetrics {
    /// Metrics for a video channel after vendor init.
    ///
    /// Channel 0 (main) uses `font_file_size * 2`; channel 1 (sub) uses the
    /// file size. Verified on hardware 2026-08-24 (Stage B).
    pub fn for_channel(channel: u8) -> Self {
        let height = if channel == 0 {
            FONT_FILE_SIZE * 2
        } else {
            FONT_FILE_SIZE
        };
        Self {
            height,
            advance: height / 2,
        }
    }
}

/// Usable dimensions of one video channel, from `CMD_OSD_INIT`'s max-rect reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelDims {
    pub width: i32,
    pub height: i32,
}

/// Where to start drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
}

/// Compute the draw origin for `glyph_count` ASCII glyphs in `corner`.
///
/// Clamps to zero rather than returning a negative origin: the vendor library
/// does not bounds-check `ak_osd_draw_str`, so an overlong string would
/// otherwise index outside the OSD buffer.
pub fn place(
    corner: Corner,
    glyph_count: usize,
    dims: ChannelDims,
    font: FontMetrics,
) -> Placement {
    let text_width = font.advance * glyph_count as i32;

    let x = match corner {
        Corner::UpperLeft | Corner::LowerLeft => font.height,
        Corner::UpperRight | Corner::LowerRight => (dims.width - text_width).max(0),
    };
    let y = match corner {
        Corner::UpperLeft | Corner::UpperRight => 0,
        Corner::LowerLeft | Corner::LowerRight => (dims.height - font.height).max(0),
    };

    Placement { x, y }
}

/// Pixel size of the OSD rect that must hold `glyph_count` glyphs.
pub fn rect_size(glyph_count: usize, font: FontMetrics) -> (i32, i32) {
    (font.advance * glyph_count as i32, font.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: ChannelDims = ChannelDims {
        width: 1280,
        height: 720,
    };
    const MAIN_FONT: FontMetrics = FontMetrics {
        height: 32,
        advance: 16,
    };
    const SUB_FONT: FontMetrics = FontMetrics {
        height: 16,
        advance: 8,
    };

    #[test]
    fn test_font_metrics_main_is_double_file_size() {
        assert_eq!(FontMetrics::for_channel(0), MAIN_FONT);
    }

    #[test]
    fn test_font_metrics_sub_matches_file_size() {
        assert_eq!(FontMetrics::for_channel(1), SUB_FONT);
    }

    #[test]
    fn test_place_upper_left_insets_by_font_height() {
        let p = place(Corner::UpperLeft, 9, MAIN, MAIN_FONT);
        assert_eq!((p.x, p.y), (32, 0));
    }

    #[test]
    fn test_place_upper_right_is_right_aligned_by_glyph_width() {
        // 9 ASCII glyphs at 16px each = 144px wide on main.
        let p = place(Corner::UpperRight, 9, MAIN, MAIN_FONT);
        assert_eq!((p.x, p.y), (1280 - 144, 0));
    }

    #[test]
    fn test_place_lower_left_sits_one_line_above_the_bottom() {
        let p = place(Corner::LowerLeft, 9, MAIN, MAIN_FONT);
        assert_eq!((p.x, p.y), (32, 720 - 32));
    }

    #[test]
    fn test_place_lower_right_combines_both_edges() {
        let p = place(Corner::LowerRight, 9, MAIN, MAIN_FONT);
        assert_eq!((p.x, p.y), (1280 - 144, 720 - 32));
    }

    #[test]
    fn test_place_clamps_overlong_text_to_zero_rather_than_negative() {
        let p = place(Corner::UpperRight, 400, MAIN, MAIN_FONT);
        assert_eq!(p.x, 0);
    }

    #[test]
    fn test_place_scales_to_the_sub_channel() {
        let sub = ChannelDims {
            width: 640,
            height: 360,
        };
        let p = place(Corner::LowerRight, 9, sub, SUB_FONT);
        assert_eq!((p.x, p.y), (640 - 72, 360 - 16));
    }

    #[test]
    fn test_rect_size_for_hello_osd_on_main() {
        assert_eq!(rect_size(9, MAIN_FONT), (144, 32));
    }
}
