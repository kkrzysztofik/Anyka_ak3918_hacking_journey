//! OSD placement math.
//!
//! Ports the vendor layout idea from `osd_disp_name`, corrected by Stage B on
//! this camera: `ak_osd_init` doubles the font-file size on the main channel
//! (16 → 32), and ASCII advance is half of the per-channel font height.
//!
//! This camera's ISP path (via `osd_vpss_wrap.c`) only drives **one** OSD DMA
//! plane per video channel — the wrap drops the rect index. Name and datetime
//! therefore share a single full-frame canvas; `place` returns draw offsets
//! inside that canvas.
//! Pure functions only — fully testable without hardware.

use serde::{Deserialize, Serialize};

/// Font-file size on disk (`/usr/local/ak_font_16.bin`).
pub const FONT_FILE_SIZE: i32 = 16;

/// Silicon rect index used for the shared per-channel canvas.
pub const CANVAS_RECT: i32 = 0;

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

/// Draw offsets inside the shared full-frame canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub draw_x: i32,
    pub draw_y: i32,
}

/// Full-frame canvas geometry for `ak_osd_set_rect` (once per channel).
pub fn canvas_rect(dims: ChannelDims) -> (i32, i32, i32, i32) {
    (0, 0, dims.width, dims.height)
}

/// Compute `draw_str` offsets for `glyph_count` ASCII glyphs in `corner`.
///
/// Clamps draw_x to zero rather than negative: the vendor library does not
/// bounds-check `ak_osd_draw_str`.
pub fn place(
    corner: Corner,
    glyph_count: usize,
    dims: ChannelDims,
    font: FontMetrics,
) -> Placement {
    let text_width = font.advance * glyph_count as i32;
    let draw_x = match corner {
        Corner::UpperLeft | Corner::LowerLeft => font.height,
        Corner::UpperRight | Corner::LowerRight => (dims.width - text_width).max(0),
    };
    let draw_y = match corner {
        Corner::UpperLeft | Corner::UpperRight => 0,
        Corner::LowerLeft | Corner::LowerRight => (dims.height - font.height).max(0),
    };

    Placement { draw_x, draw_y }
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
    fn test_canvas_rect_is_full_frame() {
        assert_eq!(canvas_rect(MAIN), (0, 0, 1280, 720));
    }

    #[test]
    fn test_place_upper_left_insets_by_font_height() {
        let p = place(Corner::UpperLeft, 9, MAIN, MAIN_FONT);
        assert_eq!((p.draw_x, p.draw_y), (32, 0));
    }

    #[test]
    fn test_place_upper_right_is_right_aligned() {
        let p = place(Corner::UpperRight, 9, MAIN, MAIN_FONT);
        assert_eq!((p.draw_x, p.draw_y), (1280 - 144, 0));
    }

    #[test]
    fn test_place_lower_left_sits_one_line_above_bottom() {
        let p = place(Corner::LowerLeft, 9, MAIN, MAIN_FONT);
        assert_eq!((p.draw_x, p.draw_y), (32, 720 - 32));
    }

    #[test]
    fn test_place_lower_right_combines_both_edges() {
        let p = place(Corner::LowerRight, 9, MAIN, MAIN_FONT);
        assert_eq!((p.draw_x, p.draw_y), (1280 - 144, 720 - 32));
    }

    #[test]
    fn test_place_clamps_overlong_text_to_zero_rather_than_negative() {
        let p = place(Corner::UpperRight, 400, MAIN, MAIN_FONT);
        assert_eq!(p.draw_x, 0);
    }

    #[test]
    fn test_place_scales_to_the_sub_channel() {
        let sub = ChannelDims {
            width: 640,
            height: 360,
        };
        let p = place(Corner::LowerRight, 9, sub, SUB_FONT);
        assert_eq!((p.draw_x, p.draw_y), (640 - 72, 360 - 16));
    }
}
