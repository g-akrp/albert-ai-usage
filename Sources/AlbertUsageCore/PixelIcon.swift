import Foundation

/// The two-row pixel icon: provider label on top in its brand color, value below in a severity
/// color. A hand-made 3×5 font, uppercase only.
public struct IconSpec: Equatable, Hashable {
    public let top: String
    public let bottom: String
    public let topColor: RGB
    public let bottomColor: RGB
    /// Spoken by VoiceOver and shown as the tooltip, never drawn.
    public let accessibility: String

    public init(top: String, bottom: String, topColor: RGB, bottomColor: RGB, accessibility: String) {
        (self.top, self.bottom, self.topColor, self.bottomColor, self.accessibility) =
            (top, bottom, topColor, bottomColor, accessibility)
    }
}

public enum PixelFont {
    public static let glyphWidth = 3, glyphHeight = 5, spacing = 1

    /// Each row is the low 3 bits, most significant bit leftmost. Unknown characters are blank.
    static func glyph(_ c: Character) -> [UInt8] {
        switch c {
        case "0": return [0b111, 0b101, 0b101, 0b101, 0b111]
        case "1": return [0b010, 0b110, 0b010, 0b010, 0b111]
        case "2": return [0b111, 0b001, 0b111, 0b100, 0b111]
        case "3": return [0b111, 0b001, 0b111, 0b001, 0b111]
        case "4": return [0b101, 0b101, 0b111, 0b001, 0b001]
        case "5": return [0b111, 0b100, 0b111, 0b001, 0b111]
        case "6": return [0b111, 0b100, 0b111, 0b101, 0b111]
        case "7": return [0b111, 0b001, 0b001, 0b001, 0b001]
        case "8": return [0b111, 0b101, 0b111, 0b101, 0b111]
        case "9": return [0b111, 0b101, 0b111, 0b001, 0b111]
        case "%": return [0b101, 0b001, 0b010, 0b100, 0b101]
        case "-": return [0b000, 0b000, 0b111, 0b000, 0b000]
        case ".": return [0b000, 0b000, 0b000, 0b000, 0b010]
        case "?": return [0b111, 0b001, 0b011, 0b000, 0b010]
        case "A": return [0b010, 0b101, 0b111, 0b101, 0b101]
        case "B": return [0b110, 0b101, 0b110, 0b101, 0b110]
        case "C": return [0b111, 0b100, 0b100, 0b100, 0b111]
        case "D": return [0b110, 0b101, 0b101, 0b101, 0b110]
        case "E": return [0b111, 0b100, 0b111, 0b100, 0b111]
        case "F": return [0b111, 0b100, 0b110, 0b100, 0b100]
        case "G": return [0b111, 0b100, 0b101, 0b101, 0b111]
        case "H": return [0b101, 0b101, 0b111, 0b101, 0b101]
        case "I": return [0b111, 0b010, 0b010, 0b010, 0b111]
        case "J": return [0b001, 0b001, 0b001, 0b101, 0b111]
        case "K": return [0b101, 0b101, 0b110, 0b101, 0b101]
        case "L": return [0b100, 0b100, 0b100, 0b100, 0b111]
        case "M": return [0b101, 0b111, 0b101, 0b101, 0b101]
        case "N": return [0b101, 0b111, 0b111, 0b111, 0b101]
        case "O": return [0b111, 0b101, 0b101, 0b101, 0b111]
        case "P": return [0b110, 0b101, 0b110, 0b100, 0b100]
        case "Q": return [0b111, 0b101, 0b101, 0b111, 0b001]
        case "R": return [0b110, 0b101, 0b110, 0b101, 0b101]
        case "S": return [0b111, 0b100, 0b111, 0b001, 0b111]
        case "T": return [0b111, 0b010, 0b010, 0b010, 0b010]
        case "U": return [0b101, 0b101, 0b101, 0b101, 0b111]
        case "V": return [0b101, 0b101, 0b101, 0b101, 0b010]
        case "W": return [0b101, 0b101, 0b101, 0b111, 0b101]
        case "X": return [0b101, 0b101, 0b010, 0b101, 0b101]
        case "Y": return [0b101, 0b101, 0b010, 0b010, 0b010]
        case "Z": return [0b111, 0b001, 0b010, 0b100, 0b111]
        default: return [0, 0, 0, 0, 0]
        }
    }

    public static func lineWidth(_ text: String) -> Int {
        text.isEmpty ? 0 : text.count * glyphWidth + (text.count - 1) * spacing
    }

    /// One filled square, in font pixels before scaling, origin top left.
    public struct Pixel: Equatable { public let x: Int, y: Int, bottomRow: Bool }

    /// Both rows centered; the second row starts one blank row below the first.
    public static func layout(top: String, bottom: String) -> (width: Int, height: Int, pixels: [Pixel]) {
        let top = top.uppercased(), bottom = bottom.uppercased()
        let width = max(lineWidth(top), lineWidth(bottom), 1)
        var pixels: [Pixel] = []
        for (text, y0, isBottom) in [(top, 0, false), (bottom, glyphHeight + 1, true)] {
            var x0 = (width - lineWidth(text)) / 2
            for c in text {
                for (row, bits) in glyph(c).enumerated() {
                    for col in 0..<glyphWidth where bits >> (glyphWidth - 1 - col) & 1 == 1 {
                        pixels.append(Pixel(x: x0 + col, y: y0 + row, bottomRow: isBottom))
                    }
                }
                x0 += glyphWidth + spacing
            }
        }
        return (width, glyphHeight * 2 + 1, pixels)
    }
}

/// Which icons the menu bar shows.
public enum StatusIcons {
    public static let green = RGB(52, 199, 89), orange = RGB(255, 149, 0), red = RGB(255, 59, 48)
    public static let placeholder = IconSpec(top: "AI", bottom: "--", topColor: .gray, bottomColor: .gray,
                                             accessibility: "Albert AI Usage")

    /// Green below 70%, orange from 70%, red from 90%.
    public static func severity(_ percent: Double) -> RGB {
        percent >= 90 ? red : percent >= 70 ? orange : green
    }

    public static func percentText(_ percent: Double) -> String {
        String(format: "%.0f%%", percent)
    }

    /// The pinned provider's icon if the pin is valid; otherwise one icon per provider, which the
    /// menu bar cycles through. Providers still loading are left out of the cycle.
    public static func icons(runs: [ProviderRun], pinned: String?) -> [IconSpec] {
        if let (run, meterId) = Pin.resolve(pinned, in: runs) {
            if let meterId, case .success(let report)? = run.result,
               let meter = report.meters.first(where: { $0.id == meterId }), let percent = meter.maxPercent {
                let text = percentText(percent)
                return [IconSpec(top: label(for: meter.label), bottom: text, topColor: run.iconColor,
                                 bottomColor: severity(percent), accessibility: "\(run.name) \(meter.label) \(text)")]
            }
            return [icon(for: run) ?? IconSpec(top: run.iconLabel, bottom: "--", topColor: run.iconColor,
                                               bottomColor: .gray, accessibility: "\(run.name) no usage data")]
        }
        let icons = runs.compactMap(icon(for:))
        return icons.isEmpty ? [placeholder] : icons
    }

    /// Top row for a pinned meter: the first three letters or digits of its label, for example GEM.
    public static func label(for meterLabel: String) -> String {
        let letters = meterLabel.uppercased().filter { $0.isASCII && ($0.isLetter || $0.isNumber) }
        return letters.isEmpty ? "?" : String(letters.prefix(3))
    }

    static func icon(for run: ProviderRun) -> IconSpec? {
        switch run.result {
        case .success(let report)?:
            guard let percent = report.maxPercent else { return nil }
            let text = percentText(percent)
            return IconSpec(top: run.iconLabel, bottom: text, topColor: run.iconColor,
                            bottomColor: severity(percent), accessibility: "\(run.name) \(text)")
        case .failure(let failure)?:
            return IconSpec(top: run.iconLabel, bottom: "ERR", topColor: run.iconColor, bottomColor: red,
                            accessibility: "\(run.name) error: \(failure.message)")
        case nil:
            return nil
        }
    }
}

/// One provider's latest state, as the menu shows it.
public struct ProviderRun: Equatable {
    public let id: String
    public let name: String
    public let iconLabel: String
    public let iconColor: RGB
    /// `nil` until the first run finishes.
    public var result: Result<Report, ProviderFailure>?
    /// The provider file this run came from; differs from `id` for one account of several.
    public let configId: String

    public init(id: String, name: String, iconLabel: String, iconColor: RGB,
                result: Result<Report, ProviderFailure>? = nil, configId: String? = nil) {
        (self.id, self.name, self.iconLabel, self.iconColor, self.result) = (id, name, iconLabel, iconColor, result)
        self.configId = configId ?? id
    }

    public init(config: ProviderConfig, result: Result<Report, ProviderFailure>? = nil) {
        self.init(id: config.id, name: config.name, iconLabel: config.iconLabel, iconColor: config.iconColor,
                  result: result)
    }
}

/// What the menu bar is pinned to: a run id (`codex`, `copilot:octocat`), or a run id and a meter
/// id joined by `|` (`antigravity|Gemini Models`).
public enum Pin {
    public static func key(run: String, meter: String) -> String { "\(run)|\(meter)" }

    /// The pinned run and meter id, if the run exists. A pin on a provider file whose runs are now
    /// per account (`copilot`) falls back to its first account.
    public static func resolve(_ pin: String?, in runs: [ProviderRun]) -> (run: ProviderRun, meter: String?)? {
        guard let pin else { return nil }
        let parts = pin.split(separator: "|", maxSplits: 1).map(String.init)
        let meter = parts.count > 1 ? parts[1] : nil
        guard let runId = parts.first else { return nil }
        if let run = runs.first(where: { $0.id == runId }) { return (run, meter) }
        if meter == nil, let run = runs.first(where: { $0.configId == runId }) { return (run, nil) }
        return nil
    }
}
