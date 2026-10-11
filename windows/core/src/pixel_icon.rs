fn glyph(c: char) -> [u8; 5] {
    match c {
        'A' => [2, 5, 7, 5, 5],
        'C' => [7, 4, 4, 4, 7],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'G' => [7, 4, 5, 5, 7],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [5, 7, 7, 7, 5],
        'O' => [7, 5, 5, 5, 7],
        'P' => [6, 5, 6, 4, 4],
        'R' => [6, 5, 6, 5, 5],
        'S' => [7, 4, 7, 1, 7],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 7, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        '%' => [5, 1, 2, 4, 5],
        '-' => [0, 0, 7, 0, 0],
        _ => [0; 5],
    }
}
/// Top-down BGRA pixels for a Windows 32-bit DIB.
pub fn render(label: &str, value: &str, brand: [u8; 3], tone: [u8; 3], size: usize) -> Vec<u8> {
    let width = label
        .chars()
        .count()
        .max(value.chars().count())
        .saturating_mul(4)
        .saturating_sub(1)
        .max(1);
    let base = width.max(11) + 2;
    let grid = render_grid(label, value, brand, tone, base);
    let mut pixels = vec![0; size * size * 4];
    let left = (base - width) / 2;
    let top = (base - 11) / 2;
    for y in 0..size {
        for x in 0..size {
            let source = ((top + y * 11 / size) * base + left + x * width / size) * 4;
            let destination = (y * size + x) * 4;
            pixels[destination..destination + 4].copy_from_slice(&grid[source..source + 4]);
        }
    }
    pixels
}
fn render_grid(label: &str, value: &str, brand: [u8; 3], tone: [u8; 3], size: usize) -> Vec<u8> {
    let mut pixels = vec![0; size * size * 4];
    if size < 3 {
        return pixels;
    }
    let width = label
        .chars()
        .count()
        .max(value.chars().count())
        .saturating_mul(4)
        .saturating_sub(1);
    let scale = ((size - 2) / width.max(1)).min((size - 2) / 11).max(1);
    let top = (size.saturating_sub(11 * scale)) / 2;
    for (text, y, color) in [(label, top, brand), (value, top + 6 * scale, tone)] {
        let x = (size.saturating_sub((text.chars().count() * 4 - 1) * scale)) / 2;
        for (n, c) in text.chars().enumerate() {
            for (j, row) in glyph(c).iter().enumerate() {
                for k in 0..3 {
                    if row & (1 << (2 - k)) != 0 {
                        for dy in 0..scale {
                            for dx in 0..scale {
                                let xx = x + (n * 4 + k) * scale + dx;
                                let yy = y + j * scale + dy;
                                if xx < size && yy < size {
                                    let i = (yy * size + xx) * 4;
                                    pixels[i..i + 4]
                                        .copy_from_slice(&[color[2], color[1], color[0], 255]);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    pixels
}
