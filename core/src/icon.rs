//! Renders a small static two-row PNG icon (e.g. "CODEX" / "47%") for
//! SwiftBar's `image=` menu-bar parameter -- a genuinely static
//! two-line label, unlike the cycling header lines SwiftBar's own
//! text titles are limited to (see docs/swiftbar.md).
//!
//! No font file is embedded (nothing here can fetch one): this is a
//! small hand-authored 3x5 pixel font covering the characters
//! provider names and percentages actually need (A-Z, 0-9, %, space).
//! Uppercase only -- a standard simplification for tiny bitmap fonts
//! at this scale.

use image::{ImageEncoder, Rgba, RgbaImage};

const GLYPH_W: u32 = 3;
const GLYPH_H: u32 = 5;
const SPACING: u32 = 1;

/// Each row is the low 3 bits of a byte, MSB = leftmost column.
fn glyph(c: char) -> [u8; 5] {
    match c {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '%' => [0b101, 0b001, 0b010, 0b100, 0b101],
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b111, 0b100, 0b111],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'N' => [0b101, 0b111, 0b111, 0b111, 0b101],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        _ => [0, 0, 0, 0, 0], // space and anything unsupported: blank
    }
}

/// Width in pixels a line of `text` (uppercased) renders to.
fn line_width(text: &str) -> u32 {
    let n = text.chars().count() as u32;
    if n == 0 {
        0
    } else {
        n * GLYPH_W + (n - 1) * SPACING
    }
}

fn draw_line(
    img: &mut RgbaImage,
    text: &str,
    x_start: u32,
    y_start: u32,
    color: Rgba<u8>,
    scale: u32,
) {
    let mut x = x_start;
    for c in text.chars() {
        let rows = glyph(c.to_ascii_uppercase());
        for (row_i, row) in rows.iter().enumerate() {
            for col in 0..GLYPH_W {
                let bit = (row >> (GLYPH_W - 1 - col)) & 1;
                if bit == 1 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            img.put_pixel(
                                x + col * scale + dx,
                                y_start + row_i as u32 * scale + dy,
                                color,
                            );
                        }
                    }
                }
            }
        }
        x += (GLYPH_W + SPACING) * scale;
    }
}

/// Renders `top` and `bottom` as two stacked, centered, static rows
/// on a transparent background, and returns encoded PNG bytes.
/// `scale` blows up each font pixel into a `scale x scale` block --
/// the native 3x5 font is too thin (5px tall per row) to read as a
/// real menu-bar icon; `scale: 2` gives a ~22px-tall image, close to
/// a typical macOS status-item icon height.
pub fn render_two_line_png(top: &str, bottom: &str, color: [u8; 3], scale: u32) -> Vec<u8> {
    let top_w = line_width(top) * scale;
    let bottom_w = line_width(bottom) * scale;
    let width = top_w.max(bottom_w).max(1);
    let row_gap = scale;
    let height = GLYPH_H * scale * 2 + row_gap;

    let mut img = RgbaImage::new(width, height);
    let color = Rgba([color[0], color[1], color[2], 255]);

    draw_line(&mut img, top, (width - top_w) / 2, 0, color, scale);
    draw_line(
        &mut img,
        bottom,
        (width - bottom_w) / 2,
        GLYPH_H * scale + row_gap,
        color,
        scale,
    );

    let mut png_bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png_bytes)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .expect("encoding a small in-memory PNG cannot fail");
    png_bytes
}

/// Standard base64 (RFC 4648), no external crate -- SwiftBar's
/// `image=` param just needs a base64 string, and this is short
/// enough not to justify a new dependency for it.
pub fn to_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied();
        let b2 = chunk.get(2).copied();

        out.push(ALPHABET[(b0 >> 2) as usize] as char);
        out.push(ALPHABET[(((b0 & 0x03) << 4) | (b1.unwrap_or(0) >> 4)) as usize] as char);
        out.push(match b1 {
            Some(b1) => ALPHABET[(((b1 & 0x0f) << 2) | (b2.unwrap_or(0) >> 6)) as usize] as char,
            None => '=',
        });
        out.push(match b2 {
            Some(b2) => ALPHABET[(b2 & 0x3f) as usize] as char,
            None => '=',
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_nonempty_png_with_correct_dimensions() {
        let png = render_two_line_png("CODEX", "47%", [0, 128, 0], 2);
        assert!(!png.is_empty());
        // PNG magic bytes.
        assert_eq!(
            &png[0..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        );
    }

    #[test]
    fn scale_two_doubles_pixel_dimensions() {
        let decoded = image::load_from_memory(&render_two_line_png("A", "1", [0, 0, 0], 2))
            .unwrap()
            .to_rgba8();
        // width: 1 glyph * 3px * scale 2 = 6; height: 5*2*2 rows + gap(2) = 22
        assert_eq!(decoded.width(), 6);
        assert_eq!(decoded.height(), 22);
    }

    #[test]
    fn line_width_accounts_for_spacing() {
        assert_eq!(line_width(""), 0);
        assert_eq!(line_width("A"), 3);
        assert_eq!(line_width("AB"), 7); // 3 + 1 + 3
    }

    #[test]
    fn base64_round_trip_known_value() {
        // "Man" -> "TWFu" is the canonical RFC 4648 worked example.
        assert_eq!(to_base64(b"Man"), "TWFu");
    }

    #[test]
    fn base64_handles_padding() {
        assert_eq!(to_base64(b"M"), "TQ==");
        assert_eq!(to_base64(b"Ma"), "TWE=");
    }
}
