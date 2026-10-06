// Draws docs/images/showcase.png: the menu bar icon in each state, on a dark and a light menu bar.
// Run from the repo root:
//   swiftc -parse-as-library -o build/make-showcase Sources/AIUsageCore/*.swift scripts/make-showcase.swift
//   build/make-showcase
// Uses the app's own font and colors, so the picture cannot drift from the app.
import AppKit

struct Sample {
    let icon: IconSpec
    let caption: String
}

func rgb(_ hex: String) -> RGB { RGB(hex: hex)! }
func icon(_ top: String, _ bottom: String, _ topColor: String, _ bottomColor: RGB) -> IconSpec {
    IconSpec(top: top, bottom: bottom, topColor: rgb(topColor), bottomColor: bottomColor, accessibility: "")
}

let claude = "D97757", codex = "10A37F", antigravity = "4285F4"

let states: [Sample] = [
    Sample(icon: icon("CLD", "14%", claude, StatusIcons.green), caption: "under 70%"),
    Sample(icon: icon("CLD", "72%", claude, StatusIcons.orange), caption: "from 70%"),
    Sample(icon: icon("CLD", "95%", claude, StatusIcons.red), caption: "from 90%"),
    Sample(icon: icon("CLD", "ERR", claude, StatusIcons.red), caption: "provider failed"),
    Sample(icon: icon("CLD", "--", claude, .gray), caption: "no data yet"),
]
let providers: [Sample] = [
    Sample(icon: icon("CLD", "14%", claude, StatusIcons.green), caption: "Claude Code"),
    Sample(icon: icon("CDX", "72%", codex, StatusIcons.orange), caption: "Codex"),
    Sample(icon: icon("AGY", "95%", antigravity, StatusIcons.red), caption: "Antigravity"),
    Sample(icon: icon("GEM", "7%", antigravity, StatusIcons.green), caption: "pinned limit"),
]

let scale: CGFloat = 10, cell: CGFloat = 200, margin: CGFloat = 40
let rowHeight: CGFloat = 190, titleHeight: CGFloat = 50
let columns = CGFloat(states.count)
let bandHeight = margin + titleHeight + rowHeight * 2 + margin
let width = Int(cell * columns + margin * 2), height = Int(bandHeight * 2)

func color(_ c: RGB) -> NSColor {
    NSColor(srgbRed: CGFloat(c.r) / 255, green: CGFloat(c.g) / 255, blue: CGFloat(c.b) / 255, alpha: 1)
}

func draw(_ text: String, at point: NSPoint, size: CGFloat, weight: NSFont.Weight, color: NSColor, centeredIn w: CGFloat? = nil) {
    let attributes: [NSAttributedString.Key: Any] = [.font: NSFont.systemFont(ofSize: size, weight: weight), .foregroundColor: color]
    let string = NSAttributedString(string: text, attributes: attributes)
    let x = w.map { point.x + ($0 - string.size().width) / 2 } ?? point.x
    string.draw(at: NSPoint(x: x, y: point.y))
}

func drawIcon(_ spec: IconSpec, centerX: CGFloat, topY: CGFloat) {
    let layout = PixelFont.layout(top: spec.top, bottom: spec.bottom)
    let originX = centerX - CGFloat(layout.width) * scale / 2
    for pixel in layout.pixels {
        color(pixel.bottomRow ? spec.bottomColor : spec.topColor).setFill()
        // Flipped y: the bitmap's origin is bottom-left, the layout's is top-left.
        NSRect(x: originX + CGFloat(pixel.x) * scale, y: topY - CGFloat(pixel.y + 1) * scale, width: scale, height: scale).fill()
    }
}

func drawBand(top bandTop: CGFloat, background: NSColor, text: NSColor, secondary: NSColor, title: String) {
    background.setFill()
    NSRect(x: 0, y: bandTop - bandHeight, width: CGFloat(width), height: bandHeight).fill()
    draw(title, at: NSPoint(x: margin, y: bandTop - margin - 30), size: 26, weight: .semibold, color: text)
    for (index, row) in [states, providers].enumerated() {
        let rowTop = bandTop - margin - titleHeight - rowHeight * CGFloat(index)
        for (column, sample) in row.enumerated() {
            let x = margin + cell * CGFloat(column)
            drawIcon(sample.icon, centerX: x + cell / 2, topY: rowTop - 20)
            draw(sample.caption, at: NSPoint(x: x, y: rowTop - 160), size: 20, weight: .regular, color: secondary, centeredIn: cell)
        }
    }
}

@main
struct Showcase {
    static func main() throws {
        let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height, bitsPerSample: 8,
                                      samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
                                      bytesPerRow: 0, bitsPerPixel: 0)!
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
        NSGraphicsContext.current?.imageInterpolation = .none
        drawBand(top: CGFloat(height), background: NSColor(srgbRed: 0.11, green: 0.12, blue: 0.14, alpha: 1),
                 text: .white, secondary: NSColor(white: 1, alpha: 0.6), title: "Dark menu bar")
        drawBand(top: bandHeight, background: NSColor(srgbRed: 0.93, green: 0.93, blue: 0.94, alpha: 1),
                 text: NSColor(white: 0.1, alpha: 1), secondary: NSColor(white: 0, alpha: 0.55), title: "Light menu bar")
        NSGraphicsContext.restoreGraphicsState()
        try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "docs/images/showcase.png"))
        print("Wrote docs/images/showcase.png")
    }
}
