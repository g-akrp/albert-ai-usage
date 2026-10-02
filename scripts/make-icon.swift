// Draws the app icon and writes Resources/AppIcon.icns.
// Run from the repo root: swift scripts/make-icon.swift
//
// Same pixel style as the menu bar icon: "AI" in the 3×5 font on top, a green/orange/red
// usage bar below, on a dark rounded square.
import AppKit

let background = NSColor(srgbRed: 0.11, green: 0.13, blue: 0.17, alpha: 1)
let glyphs: [[UInt8]] = [
    [0b010, 0b101, 0b111, 0b101, 0b101],  // A
    [0b111, 0b010, 0b010, 0b010, 0b111],  // I
]
let bar: [NSColor] = [
    NSColor(srgbRed: 52 / 255, green: 199 / 255, blue: 89 / 255, alpha: 1),
    NSColor(srgbRed: 52 / 255, green: 199 / 255, blue: 89 / 255, alpha: 1),
    NSColor(srgbRed: 52 / 255, green: 199 / 255, blue: 89 / 255, alpha: 1),
    NSColor(srgbRed: 1, green: 149 / 255, blue: 0, alpha: 1),
    NSColor(srgbRed: 1, green: 149 / 255, blue: 0, alpha: 1),
    NSColor(srgbRed: 1, green: 59 / 255, blue: 48 / 255, alpha: 1),
    NSColor.white.withAlphaComponent(0.15),
]

/// Renders the icon at `pixels` × `pixels`, following the macOS grid:
/// an 824/1024 rounded square, centred, with a soft drop shadow.
func render(pixels: Int) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = NSSize(width: pixels, height: pixels)

    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    NSGraphicsContext.current?.shouldAntialias = false
    let scale = CGFloat(pixels) / 1024
    let tile = NSRect(x: 100 * scale, y: 100 * scale, width: 824 * scale, height: 824 * scale)
    let shape = NSBezierPath(roundedRect: tile, xRadius: 185 * scale, yRadius: 185 * scale)

    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current?.shouldAntialias = true
    let shadow = NSShadow()
    shadow.shadowColor = NSColor.black.withAlphaComponent(0.3)
    shadow.shadowOffset = NSSize(width: 0, height: -10 * scale)
    shadow.shadowBlurRadius = 20 * scale
    shadow.set()
    background.setFill()
    shape.fill()
    NSGraphicsContext.restoreGraphicsState()

    // One font pixel is 72 units; "AI" is 7 × 5 font pixels, the bar 7 × 1, one blank row between.
    let unit = 72 * scale
    let left = tile.midX - 3.5 * unit
    let top = tile.midY + 3.5 * unit  // top edge of the text, in flipped-up coordinates
    func fill(_ column: Int, _ row: Int, _ color: NSColor) {
        color.setFill()
        NSRect(x: left + CGFloat(column) * unit, y: top - CGFloat(row + 1) * unit, width: unit, height: unit).fill()
    }
    for (index, glyph) in glyphs.enumerated() {
        for (row, bits) in glyph.enumerated() {
            for column in 0..<3 where bits >> (2 - column) & 1 == 1 {
                fill(index * 4 + column, row, .white)
            }
        }
    }
    for (column, color) in bar.enumerated() {
        fill(column, 6, color)
    }
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

let iconset = URL(fileURLWithPath: "build/AppIcon.iconset")
try? FileManager.default.removeItem(at: iconset)
try FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)

for points in [16, 32, 128, 256, 512] {
    for factor in [1, 2] {
        let name = factor == 1 ? "icon_\(points)x\(points).png" : "icon_\(points)x\(points)@2x.png"
        let data = render(pixels: points * factor).representation(using: .png, properties: [:])!
        try data.write(to: iconset.appendingPathComponent(name))
    }
}

let iconutil = Process()
iconutil.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
iconutil.arguments = ["-c", "icns", "-o", "Resources/AppIcon.icns", iconset.path]
try iconutil.run()
iconutil.waitUntilExit()
guard iconutil.terminationStatus == 0 else {
    FileHandle.standardError.write("iconutil failed\n".data(using: .utf8)!)
    exit(1)
}
print("Wrote Resources/AppIcon.icns")
