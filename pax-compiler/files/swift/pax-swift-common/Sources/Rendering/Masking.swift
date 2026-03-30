import SwiftUI
import Messages

private func frameAffineTransform(from coeffs: [Float]) -> CGAffineTransform {
    guard coeffs.count >= 6 else {
        return .identity
    }
    return CGAffineTransform(
        a: CGFloat(coeffs[0]),
        b: CGFloat(coeffs[1]),
        c: CGFloat(coeffs[2]),
        d: CGFloat(coeffs[3]),
        tx: CGFloat(coeffs[4]),
        ty: CGFloat(coeffs[5])
    )
}

private extension Character {
    var isSVGCommand: Bool {
        "MmLlQqCcZz".contains(self)
    }

    var isRelativeSVGCommand: Bool {
        String(self) == String(self).lowercased()
    }
}

private struct SVGPathParser {
    let pathData: String
    var index: String.Index
    var currentPoint = CGPoint.zero
    var subpathStart = CGPoint.zero
    var currentCommand: Character?

    init(pathData: String) {
        self.pathData = pathData
        self.index = pathData.startIndex
    }

    mutating func parse() -> Path? {
        var path = Path()

        while true {
            skipSeparators()
            if index >= pathData.endIndex {
                break
            }

            if let command = readCommandIfPresent() {
                currentCommand = command
            }

            guard let command = currentCommand else {
                return nil
            }

            switch command {
            case "M", "m":
                guard let point = readPoint(relativeTo: currentPoint, isRelative: command.isRelativeSVGCommand) else {
                    return nil
                }
                path.move(to: point)
                currentPoint = point
                subpathStart = point
                currentCommand = command.isRelativeSVGCommand ? "l" : "L"

                while let extraPoint = readPoint(relativeTo: currentPoint, isRelative: currentCommand?.isRelativeSVGCommand ?? false) {
                    path.addLine(to: extraPoint)
                    currentPoint = extraPoint
                }
            case "L", "l":
                var consumedAny = false
                while let point = readPoint(relativeTo: currentPoint, isRelative: command.isRelativeSVGCommand) {
                    path.addLine(to: point)
                    currentPoint = point
                    consumedAny = true
                }
                if !consumedAny {
                    return nil
                }
            case "Q", "q":
                var consumedAny = false
                while canReadNumber() {
                    let segmentStart = currentPoint
                    guard
                        let control = readPoint(relativeTo: segmentStart, isRelative: command.isRelativeSVGCommand),
                        let point = readPoint(relativeTo: segmentStart, isRelative: command.isRelativeSVGCommand)
                    else {
                        return nil
                    }
                    path.addQuadCurve(to: point, control: control)
                    currentPoint = point
                    consumedAny = true
                }
                if !consumedAny {
                    return nil
                }
            case "C", "c":
                var consumedAny = false
                while canReadNumber() {
                    let segmentStart = currentPoint
                    guard
                        let control1 = readPoint(relativeTo: segmentStart, isRelative: command.isRelativeSVGCommand),
                        let control2 = readPoint(relativeTo: segmentStart, isRelative: command.isRelativeSVGCommand),
                        let point = readPoint(relativeTo: segmentStart, isRelative: command.isRelativeSVGCommand)
                    else {
                        return nil
                    }
                    path.addCurve(to: point, control1: control1, control2: control2)
                    currentPoint = point
                    consumedAny = true
                }
                if !consumedAny {
                    return nil
                }
            case "Z", "z":
                path.closeSubpath()
                currentPoint = subpathStart
                currentCommand = nil
            default:
                return nil
            }
        }

        return path
    }

    private mutating func readCommandIfPresent() -> Character? {
        guard index < pathData.endIndex else {
            return nil
        }
        let character = pathData[index]
        guard character.isSVGCommand else {
            return nil
        }
        index = pathData.index(after: index)
        return character
    }

    private mutating func readPoint(relativeTo origin: CGPoint, isRelative: Bool) -> CGPoint? {
        guard let x = readNumber(), let y = readNumber() else {
            return nil
        }
        if isRelative {
            return CGPoint(x: origin.x + x, y: origin.y + y)
        }
        return CGPoint(x: x, y: y)
    }

    private mutating func readNumber() -> CGFloat? {
        skipSeparators()
        guard canReadNumber() else {
            return nil
        }

        let start = index
        while index < pathData.endIndex {
            let character = pathData[index]
            if character.isNumber || character == "-" || character == "+" || character == "." || character == "e" || character == "E" {
                index = pathData.index(after: index)
            } else {
                break
            }
        }

        let token = String(pathData[start..<index])
        guard let value = Double(token) else {
            return nil
        }
        return CGFloat(value)
    }

    private mutating func skipSeparators() {
        while index < pathData.endIndex {
            let character = pathData[index]
            if character == "," || character.isWhitespace {
                index = pathData.index(after: index)
            } else {
                break
            }
        }
    }

    private mutating func canReadNumber() -> Bool {
        skipSeparators()
        guard index < pathData.endIndex else {
            return false
        }
        let character = pathData[index]
        return character.isNumber || character == "-" || character == "+" || character == "."
    }
}

func parseSVGPath(_ pathData: String) -> Path? {
    guard !pathData.isEmpty else {
        return nil
    }
    var parser = SVGPathParser(pathData: pathData)
    return parser.parse()
}

struct SVGPathShape: Shape {
    let pathData: String

    func path(in _: CGRect) -> Path {
        parseSVGPath(pathData) ?? Path()
    }
}

struct FrameClipDescriptor {
    let clipPath: String?
    let transform: [Float]
    let size: CGSize
}

private func frameWorldPath(for descriptor: FrameClipDescriptor) -> Path {
    if let clipPath = descriptor.clipPath,
       !clipPath.isEmpty,
       let path = parseSVGPath(clipPath) {
        return path
    }

    let rect = CGRect(
        x: 0,
        y: 0,
        width: descriptor.size.width,
        height: descriptor.size.height
    )
    return Path(rect).applying(frameAffineTransform(from: descriptor.transform))
}

struct FrameClipShape: Shape {
    let descriptor: FrameClipDescriptor
    let localFromWorld: CGAffineTransform

    func path(in _: CGRect) -> Path {
        frameWorldPath(for: descriptor).applying(localFromWorld)
    }
}

struct NativeMaskView: View {
    let patch: NativeMaskPatch
    let fallbackSize: CGSize

    private var resolvedSize: CGSize {
        let width = patch.size_x > 0 ? CGFloat(patch.size_x) : fallbackSize.width
        let height = patch.size_y > 0 ? CGFloat(patch.size_y) : fallbackSize.height
        return CGSize(width: max(0, width), height: max(0, height))
    }

    var body: some View {
        Canvas { context, _ in
            let bounds = CGRect(origin: .zero, size: resolvedSize)
            context.fill(Path(bounds), with: .color(.white))

            for entry in patch.entries {
                guard let holePath = parseSVGPath(entry.path) else {
                    continue
                }

                var holeContext = context
                for clip in entry.clips {
                    if let clipPath = parseSVGPath(clip) {
                        holeContext.clip(to: clipPath)
                    }
                }
                holeContext.blendMode = .destinationOut
                holeContext.fill(holePath, with: .color(.white))
            }
        }
        .frame(width: resolvedSize.width, height: resolvedSize.height)
    }
}
