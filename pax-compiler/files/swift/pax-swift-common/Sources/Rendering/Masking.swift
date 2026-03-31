import SwiftUI
import Messages

private enum SVGPathCache {
    static var paths: [String: Path] = [:]
    static var insertionOrder: [String] = []
    static let maxEntries = 512

    static func cachedPath(for key: String) -> Path? {
        paths[key]
    }

    static func store(_ path: Path, for key: String) {
        if paths[key] == nil {
            insertionOrder.append(key)
            if insertionOrder.count > maxEntries, let oldestKey = insertionOrder.first {
                insertionOrder.removeFirst()
                paths.removeValue(forKey: oldestKey)
            }
        }
        paths[key] = path
    }
}

private enum ResolvedNativeMaskCache {
    static var masks: [PaxNodeId: ResolvedNativeMask] = [:]
}

public struct ResolvedMaskHole {
    public let path: Path
    public let clips: [Path]

    public init(path: Path, clips: [Path]) {
        self.path = path
        self.clips = clips
    }
}

public struct ResolvedNativeMask {
    public let size: CGSize
    public let frameClips: [Path]
    public let holes: [ResolvedMaskHole]

    public init(size: CGSize, frameClips: [Path], holes: [ResolvedMaskHole]) {
        self.size = size
        self.frameClips = frameClips
        self.holes = holes
    }
}

private struct ResolvedPathShape: Shape {
    let resolvedPath: Path

    func path(in _: CGRect) -> Path {
        resolvedPath
    }
}

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

private func invertedFrameAffineTransform(from coeffs: [Float]) -> CGAffineTransform {
    let transform = frameAffineTransform(from: coeffs)
    let determinant = (transform.a * transform.d) - (transform.b * transform.c)
    guard abs(determinant) > .ulpOfOne else {
        return .identity
    }
    return transform.inverted()
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
    if let cached = SVGPathCache.cachedPath(for: pathData) {
        return cached
    }
    var parser = SVGPathParser(pathData: pathData)
    let path = parser.parse()
    if let path {
        SVGPathCache.store(path, for: pathData)
    }
    return path
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

private func collectFrameClipDescriptors(startingAt parentFrame: PaxNodeId?, frames: [PaxNodeId: FrameElement]) -> [FrameClipDescriptor] {
    var descriptors: [FrameClipDescriptor] = []
    var currentFrame = parentFrame

    while let frameId = currentFrame, let frame = frames[frameId] {
        if frame.clipContent {
            descriptors.insert(
                FrameClipDescriptor(
                    clipPath: frame.clipPath,
                    transform: frame.transform,
                    size: CGSize(width: CGFloat(frame.size_x), height: CGFloat(frame.size_y))
                ),
                at: 0
            )
        }
        currentFrame = frame.parentFrame
    }

    return descriptors
}

private func resolvedMaskSize(patch: NativeMaskPatch?, fallbackSize: CGSize) -> CGSize {
    let width = (patch?.size_x ?? 0) > 0 ? CGFloat(patch?.size_x ?? 0) : fallbackSize.width
    let height = (patch?.size_y ?? 0) > 0 ? CGFloat(patch?.size_y ?? 0) : fallbackSize.height
    return CGSize(width: max(0, width), height: max(0, height))
}

public func setResolvedNativeMask(id: PaxNodeId, mask: ResolvedNativeMask?) {
    if let mask {
        ResolvedNativeMaskCache.masks[id] = mask
    } else {
        ResolvedNativeMaskCache.masks.removeValue(forKey: id)
    }
}

public func resolvedNativeMask(for id: PaxNodeId) -> ResolvedNativeMask? {
    ResolvedNativeMaskCache.masks[id]
}

public func removeResolvedNativeMask(id: PaxNodeId) {
    ResolvedNativeMaskCache.masks.removeValue(forKey: id)
}

public func resolveNativeMask(
    elementTransform: [Float],
    parentFrame: PaxNodeId?,
    patch: NativeMaskPatch?,
    fallbackSize: CGSize,
    frames: [PaxNodeId: FrameElement]
) -> ResolvedNativeMask? {
    let size = resolvedMaskSize(patch: patch, fallbackSize: fallbackSize)
    let localFromWorld = invertedFrameAffineTransform(from: elementTransform)

    let frameClips = collectFrameClipDescriptors(startingAt: parentFrame, frames: frames).compactMap { descriptor -> Path? in
        let localPath = frameWorldPath(for: descriptor).applying(localFromWorld)
        return localPath.isEmpty ? nil : localPath
    }

    let holes = patch?.entries.compactMap { entry -> ResolvedMaskHole? in
        guard let holePath = parseSVGPath(entry.path) else {
            return nil
        }
        if holePath.isEmpty {
            return nil
        }
        let localClips = entry.clips.compactMap { clip -> Path? in
            guard let clipPath = parseSVGPath(clip) else {
                return nil
            }
            return clipPath.isEmpty ? nil : clipPath
        }
        return ResolvedMaskHole(path: holePath, clips: localClips)
    } ?? []

    if frameClips.isEmpty && holes.isEmpty {
        return nil
    }

    return ResolvedNativeMask(size: size, frameClips: frameClips, holes: holes)
}

public struct CombinedMaskView: View {
    let mask: ResolvedNativeMask

    public init(mask: ResolvedNativeMask) {
        self.mask = mask
    }

    public var body: some View {
        ZStack {
            clippedView(
                ResolvedPathShape(resolvedPath: Path(CGRect(origin: .zero, size: mask.size)))
                    .fill(Color.white),
                paths: mask.frameClips
            )

            ForEach(Array(mask.holes.enumerated()), id: \.offset) { _, hole in
                clippedView(
                    ResolvedPathShape(resolvedPath: hole.path)
                        .fill(Color.black),
                    paths: hole.clips
                )
            }
        }
        .compositingGroup()
        .luminanceToAlpha()
        .frame(width: mask.size.width, height: mask.size.height)
    }

    private func clippedView<V: View>(_ view: V, paths: [Path]) -> AnyView {
        paths.reduce(AnyView(view)) { current, path in
            AnyView(current.clipShape(ResolvedPathShape(resolvedPath: path)))
        }
    }
}
