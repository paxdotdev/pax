import SwiftUI
import Messages
#if os(iOS) || os(tvOS) || os(watchOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

private func affineTransform(from coeffs: [Float]) -> CGAffineTransform {
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

private func safeInverseTransform(_ transform: CGAffineTransform) -> CGAffineTransform {
    let determinant = (transform.a * transform.d) - (transform.b * transform.c)
    guard abs(determinant) > .ulpOfOne else {
        return .identity
    }
    return transform.inverted()
}

private func resolvedDimension(_ value: Float) -> CGFloat? {
    guard value >= 0 else {
        return nil
    }
    return CGFloat(value)
}

private func resolvedSize(_ element: NativePositionElement) -> CGSize {
    CGSize(width: max(0, CGFloat(element.size_x)), height: max(0, CGFloat(element.size_y)))
}

private func resolvedTextSize(_ element: TextElement) -> CGSize {
    CGSize(width: max(0, CGFloat(element.size_x)), height: max(0, CGFloat(element.size_y)))
}

private func combineCGSize(_ size: CGSize, into hasher: inout Hasher) {
    hasher.combine(size.width.bitPattern)
    hasher.combine(size.height.bitPattern)
}

private func combineCGFloat(_ value: CGFloat, into hasher: inout Hasher) {
    hasher.combine(value.bitPattern)
}

private func combinePath(_ path: CGPath, into hasher: inout Hasher) {
    path.applyWithBlock { elementPointer in
        let element = elementPointer.pointee
        let pointCount: Int
        switch element.type {
        case .moveToPoint:
            hasher.combine(0)
            pointCount = 1
        case .addLineToPoint:
            hasher.combine(1)
            pointCount = 1
        case .addQuadCurveToPoint:
            hasher.combine(2)
            pointCount = 2
        case .addCurveToPoint:
            hasher.combine(3)
            pointCount = 3
        case .closeSubpath:
            hasher.combine(4)
            pointCount = 0
        @unknown default:
            hasher.combine(99)
            pointCount = 0
        }
        for index in 0..<pointCount {
            combineCGFloat(element.points[index].x, into: &hasher)
            combineCGFloat(element.points[index].y, into: &hasher)
        }
    }
}

private func clipPathSignature(_ paths: [CGPath], size: CGSize) -> Int {
    var hasher = Hasher()
    combineCGSize(size, into: &hasher)
    hasher.combine(paths.count)
    for path in paths {
        combinePath(path, into: &hasher)
    }
    return hasher.finalize()
}

private func transformedClipPaths(_ paths: [CGPath], by transform: CGAffineTransform) -> [CGPath] {
    paths.map { path in
        var transform = transform
        return path.copy(using: &transform) ?? path
    }
}

private func combineDouble(_ value: Double, into hasher: inout Hasher) {
    hasher.combine(value.bitPattern)
}

private func combineFloat(_ value: Float, into hasher: inout Hasher) {
    hasher.combine(value.bitPattern)
}

private func combineColor(_ color: Color, into hasher: inout Hasher) {
#if os(iOS) || os(tvOS) || os(watchOS)
    let platform = platformColor(color).cgColor
#elseif os(macOS)
    let platform = (platformColor(color).usingColorSpace(.deviceRGB) ?? platformColor(color)).cgColor
#endif
    if let converted = platform.converted(to: CGColorSpaceCreateDeviceRGB(), intent: .defaultIntent, options: nil),
       let components = converted.components {
        for component in components {
            combineCGFloat(component, into: &hasher)
        }
    } else if let components = platform.components {
        for component in components {
            combineCGFloat(component, into: &hasher)
        }
    } else {
        hasher.combine(String(describing: platform))
    }
}

private func combineFont(_ font: PaxFont, into hasher: inout Hasher) {
    hasher.combine(String(describing: font.type))
}

private func combineTextStyle(_ style: TextStyle, into hasher: inout Hasher) {
    combineFont(style.font, into: &hasher)
    combineColor(style.fill, into: &hasher)
    hasher.combine(String(describing: style.alignmentMultiline))
    hasher.combine(style.font_size.bitPattern)
    hasher.combine(style.underline)
}

private func textAlignment(from alignment: TextAlignment) -> TextAlignment {
    alignment
}

private struct RasterizedMaskHolePayload {
    let cgPath: CGPath
    let clipCGPaths: [CGPath]
    let opacity: CGFloat
}

private struct RasterizedNativeMaskPayload {
    let signature: UInt64
    let size: CGSize
    let holes: [RasterizedMaskHolePayload]
}

private enum NativeMaskDebug {
    static let enabled = ProcessInfo.processInfo.environment["PAX_DEBUG_NATIVE_MASKS"] == "1"

    static func log(_ message: @autoclosure () -> String) {
        guard enabled else {
            return
        }
        fputs("[pax-native-mask] \(message())\n", stderr)
    }
}

private struct PendingMaskRender {
    let generation: UInt64
    let scale: CGFloat
    let payload: RasterizedNativeMaskPayload
}

private func currentNativeMaskScale() -> CGFloat {
#if os(iOS) || os(tvOS) || os(watchOS)
    #if targetEnvironment(simulator)
    return 1.0
    #else
    return UIScreen.main.scale
    #endif
#elseif os(macOS)
    return NSScreen.main?.backingScaleFactor ?? 1.0
#endif
}

private func configureNativeTransformLayer(_ layer: CALayer) {
    layer.allowsEdgeAntialiasing = true
    layer.edgeAntialiasingMask = [
        .layerLeftEdge,
        .layerRightEdge,
        .layerTopEdge,
        .layerBottomEdge
    ]
}

private struct RasterizedNativeMaskCacheKey: Hashable {
    let signature: UInt64
    let width: UInt64
    let height: UInt64
    let scale: UInt64
}

private enum RasterizedNativeMaskImageCache {
    static var images: [RasterizedNativeMaskCacheKey: CGImage] = [:]
    static var insertionOrder: [RasterizedNativeMaskCacheKey] = []
    static let maxEntries = 256

    static func image(for key: RasterizedNativeMaskCacheKey) -> CGImage? {
        images[key]
    }

    static func store(_ image: CGImage, for key: RasterizedNativeMaskCacheKey) {
        if images[key] == nil {
            insertionOrder.append(key)
            if insertionOrder.count > maxEntries, let oldest = insertionOrder.first {
                insertionOrder.removeFirst()
                images.removeValue(forKey: oldest)
            }
        }
        images[key] = image
    }
}

private func rasterPayload(from mask: ResolvedNativeMask) -> RasterizedNativeMaskPayload {
    RasterizedNativeMaskPayload(
        signature: mask.signature,
        size: mask.size,
        holes: mask.holes.map { hole in
            RasterizedMaskHolePayload(
                cgPath: hole.cgPath,
                clipCGPaths: hole.clipCGPaths,
                opacity: CGFloat(hole.opacity)
            )
        }
    )
}

private func rasterizedMaskImage(
    payload: RasterizedNativeMaskPayload,
    scale: CGFloat
) -> CGImage? {
    let pixelWidth = max(Int(ceil(payload.size.width * scale)), 1)
    let pixelHeight = max(Int(ceil(payload.size.height * scale)), 1)
    let bytesPerRow = pixelWidth * 4
    let bitmapInfo = CGImageAlphaInfo.premultipliedLast.rawValue
        | CGBitmapInfo.byteOrder32Big.rawValue

    guard let context = CGContext(
        data: nil,
        width: pixelWidth,
        height: pixelHeight,
        bitsPerComponent: 8,
        bytesPerRow: bytesPerRow,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: bitmapInfo
    ) else {
        return nil
    }

    // Pax/native leaf geometry is expressed in a top-left, Y-down space.
    // Raw CoreGraphics bitmap contexts default to a bottom-left, Y-up space,
    // so normalize the context before rasterizing hole geometry into the mask.
    // Frame clip paths are expressed in Pax's top-left, Y-down coordinate space.
    context.scaleBy(x: scale, y: scale)
    context.translateBy(x: 0, y: payload.size.height)
    context.scaleBy(x: 1, y: -1)
    let bounds = CGRect(origin: .zero, size: payload.size)
    context.setBlendMode(.normal)
    context.setFillColor(gray: 1.0, alpha: 1.0)
    context.fill(bounds)

    for hole in payload.holes {
        context.saveGState()
        for clip in hole.clipCGPaths {
            context.addPath(clip)
            context.clip()
        }
        // CALayer.mask consumes alpha, not luminance. Remove coverage from the
        // opaque backdrop directly so partial-opacity vector punch-through
        // attenuates the native leaf instead of acting like a color-only tint.
        context.setBlendMode(.destinationOut)
        context.setFillColor(gray: 1.0, alpha: hole.opacity)
        context.addPath(hole.cgPath)
        context.fillPath()
        context.restoreGState()
    }

    return context.makeImage()
}

private func rasterizedPositiveClipMaskImage(
    paths: [CGPath],
    size: CGSize,
    scale: CGFloat
) -> CGImage? {
    guard size.width > 0, size.height > 0, !paths.isEmpty else {
        return nil
    }
    let pixelWidth = max(Int(ceil(size.width * scale)), 1)
    let pixelHeight = max(Int(ceil(size.height * scale)), 1)
    let bytesPerRow = pixelWidth * 4
    let bitmapInfo = CGImageAlphaInfo.premultipliedLast.rawValue
        | CGBitmapInfo.byteOrder32Big.rawValue

    guard let context = CGContext(
        data: nil,
        width: pixelWidth,
        height: pixelHeight,
        bitsPerComponent: 8,
        bytesPerRow: bytesPerRow,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: bitmapInfo
    ) else {
        return nil
    }

    context.scaleBy(x: scale, y: scale)
    context.translateBy(x: 0, y: size.height)
    context.scaleBy(x: 1, y: -1)
    context.setFillColor(gray: 1.0, alpha: 1.0)
    for path in paths {
        context.addPath(path)
    }
    context.fillPath()

    return context.makeImage()
}

private func rasterizedMaskCacheKey(
    payload: RasterizedNativeMaskPayload,
    scale: CGFloat
) -> RasterizedNativeMaskCacheKey {
    RasterizedNativeMaskCacheKey(
        signature: payload.signature,
        width: UInt64(Double(payload.size.width).bitPattern),
        height: UInt64(Double(payload.size.height).bitPattern),
        scale: UInt64(Double(scale).bitPattern)
    )
}

private func cachedRasterizedMaskImage(
    payload: RasterizedNativeMaskPayload,
    scale: CGFloat
) -> CGImage? {
    let key = rasterizedMaskCacheKey(payload: payload, scale: scale)
    if let cached = RasterizedNativeMaskImageCache.image(for: key) {
        return cached
    }
    guard let image = rasterizedMaskImage(payload: payload, scale: scale) else {
        return nil
    }
    RasterizedNativeMaskImageCache.store(image, for: key)
    return image
}

#if os(macOS)
private func compositedMaskedSnapshotImage(
    snapshot: CGImage,
    mask: CGImage,
    size: CGSize,
    scale: CGFloat
) -> CGImage? {
    let pixelWidth = max(Int(ceil(size.width * scale)), 1)
    let pixelHeight = max(Int(ceil(size.height * scale)), 1)
    let bytesPerRow = pixelWidth * 4
    let bitmapInfo = CGImageAlphaInfo.premultipliedLast.rawValue
        | CGBitmapInfo.byteOrder32Big.rawValue

    guard let context = CGContext(
        data: nil,
        width: pixelWidth,
        height: pixelHeight,
        bitsPerComponent: 8,
        bytesPerRow: bytesPerRow,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: bitmapInfo
    ) else {
        return nil
    }

    let rect = CGRect(origin: .zero, size: CGSize(width: pixelWidth, height: pixelHeight))
    context.interpolationQuality = .high
    context.draw(snapshot, in: rect)
    context.setBlendMode(.destinationIn)
    context.draw(mask, in: rect)
    return context.makeImage()
}
#endif

#if os(iOS) || os(tvOS) || os(watchOS)
private func platformColor(_ color: Color) -> UIColor {
    UIColor(color)
}

private func alignedTextLayerFrame(containerSize: CGSize, measuredTextSize: CGSize, alignment: Alignment, clip: Bool) -> CGRect {
    let width = max(containerSize.width, 1)
    let clampedTextHeight = max(measuredTextSize.height, 1)
    let y: CGFloat
    switch alignment.vertical {
    case .center:
        y = clip ? max((containerSize.height - clampedTextHeight) * 0.5, 0) : (containerSize.height - clampedTextHeight) * 0.5
    case .bottom:
        y = clip ? max(containerSize.height - clampedTextHeight, 0) : containerSize.height - clampedTextHeight
    default:
        y = 0
    }
    let height = clip ? min(clampedTextHeight, max(containerSize.height - y, 1)) : clampedTextHeight
    return CGRect(x: 0, y: y, width: width, height: height)
}

private func platformLayerTextAlignment(_ alignment: Alignment) -> CATextLayerAlignmentMode {
    switch alignment.horizontal {
    case .center:
        return .center
    case .trailing:
        return .right
    default:
        return .left
    }
}

private func platformHorizontalTextAlignment(_ alignment: Alignment) -> NSTextAlignment {
    switch alignment.horizontal {
    case .center:
        return .center
    case .trailing:
        return .right
    default:
        return .left
    }
}

private func platformTextAlignment(_ alignment: TextAlignment) -> NSTextAlignment {
    switch alignment {
    case .center:
        return .center
    case .trailing:
        return .right
    default:
        return .left
    }
}
#elseif os(macOS)
private func platformColor(_ color: Color) -> NSColor {
    NSColor(color)
}

private func alignedTextLayerFrame(containerSize: CGSize, measuredTextSize: CGSize, alignment: Alignment, clip: Bool) -> CGRect {
    let width = max(containerSize.width, 1)
    let clampedTextHeight = max(measuredTextSize.height, 1)
    let y: CGFloat
    switch alignment.vertical {
    case .center:
        y = clip ? max((containerSize.height - clampedTextHeight) * 0.5, 0) : (containerSize.height - clampedTextHeight) * 0.5
    case .bottom:
        y = clip ? max(containerSize.height - clampedTextHeight, 0) : containerSize.height - clampedTextHeight
    default:
        y = 0
    }
    let height = clip ? min(clampedTextHeight, max(containerSize.height - y, 1)) : clampedTextHeight
    return CGRect(x: 0, y: y, width: width, height: height)
}

private func platformTextAlignment(_ alignment: TextAlignment) -> NSTextAlignment {
    switch alignment {
    case .center:
        return .center
    case .trailing:
        return .right
    default:
        return .left
    }
}

private func platformHorizontalTextAlignment(_ alignment: Alignment) -> NSTextAlignment {
    switch alignment.horizontal {
    case .center:
        return .center
    case .trailing:
        return .right
    default:
        return .left
    }
}

private func platformLayerTextAlignment(_ alignment: Alignment) -> CATextLayerAlignmentMode {
    switch alignment.horizontal {
    case .center:
        return .center
    case .trailing:
        return .right
    default:
        return .left
    }
}
#endif

public final class NativeLayerCountTracker {
    public static let shared = NativeLayerCountTracker()
    public private(set) var layerCount: Int = 1

    private init() {}

    public func update(_ count: Int) {
        layerCount = max(count, 1)
    }
}

public final class NativeScrollerHostRegistry {
#if os(iOS) || os(tvOS) || os(watchOS)
    public typealias HostView = UIView
#elseif os(macOS)
    public typealias HostView = NSView
#endif

    public struct Hosts {
        public let canvasHost: HostView
        public let contentHost: HostView
    }

    public static let shared = NativeScrollerHostRegistry()
    private var hosts: [PaxNodeId: Hosts] = [:]

    private init() {}

    public func register(id: PaxNodeId, canvasHost: HostView, contentHost: HostView) {
        hosts[id] = Hosts(canvasHost: canvasHost, contentHost: contentHost)
    }

    public func unregister(id: PaxNodeId) {
        hosts.removeValue(forKey: id)
    }

    public func canvasHost(for id: PaxNodeId) -> HostView? {
        hosts[id]?.canvasHost
    }

    public func contentHost(for id: PaxNodeId) -> HostView? {
        hosts[id]?.contentHost
    }
}

public struct NativeRenderingLayer: View {
    public init() {}

    private let fontObserver = FontRegistrationObserver.shared
    @ObservedObject var nativeSceneInvalidation = NativeSceneInvalidation.singleton
    let textElements = TextElements.singleton
    let frameElements = FrameElements.singleton
    let scrollerElements = ScrollerElements.singleton
    let buttonElements = ButtonElements.singleton
    let checkboxElements = CheckboxElements.singleton
    let nativeImageElements = NativeImageElements.singleton
    let youtubeVideoElements = YoutubeVideoElements.singleton
    let dropdownElements = DropdownElements.singleton
    let radioListElements = RadioListElements.singleton
    let sliderElements = SliderElements.singleton
    let textboxElements = TextboxElements.singleton
    let eventBlockerElements = EventBlockerElements.singleton

    fileprivate enum NativeLeafKind {
        case text(TextElement)
        case button(ButtonElement)
        case checkbox(CheckboxElement)
        case slider(SliderElement)
        case dropdown(DropdownElement)
        case radioList(RadioListElement)
        case textbox(TextboxElement)
        case nativeImage(NativeImageElement)
        case youtubeVideo(YoutubeVideoElement)
        case eventBlocker(EventBlockerElement)

        var contentKey: String {
            switch self {
            case .text(let element):
                if element.editable {
                    return "text-editable"
                }
                // Unclipped selectable text stays on the static text layer. Native text views
                // impose their own clip region, so we only switch view types when clipping or
                // editing semantics require it.
                if element.selectable && element.clip {
                    return "text-selectable"
                }
                return "text-static"
            case .button:
                return "button"
            case .checkbox:
                return "checkbox"
            case .slider:
                return "slider"
            case .dropdown:
                return "dropdown"
            case .radioList:
                return "radio-list"
            case .textbox(let element):
                return element.isTextArea ? "textbox-area" : "textbox-field"
            case .nativeImage:
                return "native-image"
            case .youtubeVideo:
                return "youtube-video"
            case .eventBlocker:
                return "event-blocker"
            }
        }

        func contentSignature(size: CGSize) -> Int {
            var hasher = Hasher()
            hasher.combine(contentKey)
            combineCGSize(size, into: &hasher)
            switch self {
            case .text(let element):
                hasher.combine(element.content)
                hasher.combine(element.editable)
                hasher.combine(element.selectable)
                hasher.combine(element.clip)
                hasher.combine(element.markdown)
                combineTextStyle(element.textStyle, into: &hasher)
                if let styleLink = element.style_link {
                    combineTextStyle(styleLink, into: &hasher)
                } else {
                    hasher.combine(0)
                }
            case .button(let element):
                hasher.combine(element.content)
                combineColor(element.color, into: &hasher)
                combineColor(element.hoverColor, into: &hasher)
                combineColor(element.outlineStrokeColor, into: &hasher)
                combineDouble(element.outlineStrokeWidth, into: &hasher)
                combineDouble(element.borderRadius, into: &hasher)
                combineTextStyle(element.style, into: &hasher)
            case .checkbox(let element):
                hasher.combine(element.checked)
                combineColor(element.background, into: &hasher)
                combineColor(element.backgroundChecked, into: &hasher)
                combineColor(element.outlineColor, into: &hasher)
                combineDouble(element.outlineWidth, into: &hasher)
                combineDouble(element.borderRadius, into: &hasher)
            case .slider(let element):
                combineDouble(element.value, into: &hasher)
                combineDouble(element.step, into: &hasher)
                combineDouble(element.min, into: &hasher)
                combineDouble(element.max, into: &hasher)
                combineColor(element.accent, into: &hasher)
                combineColor(element.background, into: &hasher)
                combineDouble(element.borderRadius, into: &hasher)
            case .dropdown(let element):
                hasher.combine(element.selectedId)
                hasher.combine(element.options)
                combineColor(element.background, into: &hasher)
                combineColor(element.strokeColor, into: &hasher)
                combineDouble(element.strokeWidth, into: &hasher)
                combineDouble(element.borderRadius, into: &hasher)
                combineTextStyle(element.style, into: &hasher)
            case .radioList(let element):
                hasher.combine(element.selectedId)
                hasher.combine(element.options)
                combineTextStyle(element.style, into: &hasher)
                combineColor(element.backgroundChecked, into: &hasher)
                combineColor(element.outlineColor, into: &hasher)
                combineDouble(element.outlineWidth, into: &hasher)
                combineColor(element.background, into: &hasher)
            case .textbox(let element):
                hasher.combine(element.text)
                hasher.combine(element.focusOnMount)
                hasher.combine(element.placeholder)
                hasher.combine(element.isTextArea)
                combineColor(element.background, into: &hasher)
                combineColor(element.strokeColor, into: &hasher)
                combineDouble(element.strokeWidth, into: &hasher)
                combineDouble(element.borderRadius, into: &hasher)
                combineTextStyle(element.style, into: &hasher)
                combineColor(element.outlineColor, into: &hasher)
                combineDouble(element.outlineWidth, into: &hasher)
            case .nativeImage(let element):
                hasher.combine(element.url)
                hasher.combine(element.fit)
            case .youtubeVideo(let element):
                hasher.combine(element.url)
            case .eventBlocker:
                break
            }
            return hasher.finalize()
        }
    }

    private struct NativeRenderItem: Identifiable {
        let id: PaxNodeId
        let zIndex: Int
        let parentFrame: PaxNodeId?
        let localTransform: CGAffineTransform
        let size: CGSize
        let opacity: Double
        let kind: NativeLeafKind
        let mask: ResolvedNativeMask?
    }

    private struct FrameRenderNode: Identifiable {
        let id: PaxNodeId
        let zIndex: Int
        let parentFrame: PaxNodeId?
        let localTransform: CGAffineTransform
        let size: CGSize
        let opacity: Double
        let clipContent: Bool
        let borderRadius: CGFloat
        let clipPath: CGPath?
        let clipSignature: Int
        let children: [NativeRenderNode]
    }

    private struct ScrollerRenderNode: Identifiable {
        let id: PaxNodeId
        let zIndex: Int
        let parentFrame: PaxNodeId?
        let localTransform: CGAffineTransform
        let size: CGSize
        let opacity: Double
        let clipContent: Bool
        let borderRadius: CGFloat
        let clipSignature: Int
        let contentSize: CGSize
        let scrollX: Double
        let scrollY: Double
        let presentationScrollX: Double
        let presentationScrollY: Double
        let scrollEnabledX: Bool
        let scrollEnabledY: Bool
        let snapPointsX: [CGFloat]
        let snapPointsY: [CGFloat]
        let mask: ResolvedNativeMask?
        let children: [NativeRenderNode]
    }

    private final class RenderTreeCache {
        var generation: UInt64 = .max
        var nodes: [NativeRenderNode] = []
    }

    private static let renderTreeCache = RenderTreeCache()

    private enum NativeRenderNode: Identifiable {
        case item(NativeRenderItem)
        case frame(FrameRenderNode)
        case scroller(ScrollerRenderNode)

        var id: String {
            switch self {
            case .item(let item):
                return "item-\(item.id)"
            case .frame(let frame):
                return "frame-\(frame.id)"
            case .scroller(let scroller):
                return "scroller-\(scroller.id)"
            }
        }

        var zIndex: Int {
            switch self {
            case .item(let item):
                return item.zIndex
            case .frame(let frame):
                return frame.zIndex
            case .scroller(let scroller):
                return scroller.zIndex
            }
        }

        var numericId: PaxNodeId {
            switch self {
            case .item(let item):
                return item.id
            case .frame(let frame):
                return frame.id
            case .scroller(let scroller):
                return scroller.id
            }
        }
    }

#if os(iOS) || os(tvOS) || os(watchOS)
    fileprivate typealias PlatformBaseView = UIView
    fileprivate typealias PlatformBaseViewController = UIViewController
#elseif os(macOS)
    fileprivate typealias PlatformBaseView = NSView
    fileprivate typealias PlatformBaseViewController = NSViewController
#endif

    private class PlatformContainerView: PlatformBaseView {
        private let clipMaskLayer = CAShapeLayer()
        private struct AppliedGeometry: Equatable {
            let size: CGSize
            let localTransform: CGAffineTransform
            let zIndex: Int
            let opacity: Double
        }
        private var appliedGeometry: AppliedGeometry?
        private var appliedClipSignature: Int?

#if os(iOS) || os(tvOS) || os(watchOS)
        override init(frame: CGRect) {
            super.init(frame: frame)
            backgroundColor = .clear
            isOpaque = false
            clipsToBounds = false
            autoresizesSubviews = false
            layer.anchorPoint = CGPoint(x: 0.0, y: 0.0)
            configureNativeTransformLayer(layer)
        }
#elseif os(macOS)
        override var isFlipped: Bool { true }

        override init(frame frameRect: CGRect) {
            super.init(frame: frameRect)
            wantsLayer = true
            layer?.backgroundColor = NSColor.clear.cgColor
            layer?.anchorPoint = CGPoint(x: 0.0, y: 0.0)
            if let layer {
                configureNativeTransformLayer(layer)
            }
            autoresizesSubviews = false
        }

#endif

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        var backingLayer: CALayer {
#if os(macOS)
            guard let layer = self.layer else {
                fatalError("PlatformContainerView expected backing layer")
            }
            return layer
#else
            return self.layer
#endif
        }

        func applyClip(path: CGPath?, signature: Int, borderRadius: CGFloat, clipContent: Bool) {
            if appliedClipSignature == signature {
                return
            }
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            if !clipContent {
                backingLayer.mask = nil
                backingLayer.cornerRadius = 0
                backingLayer.masksToBounds = false
            } else if borderRadius > 0 {
                backingLayer.mask = nil
                backingLayer.cornerRadius = borderRadius
                backingLayer.masksToBounds = true
            } else if let path {
                clipMaskLayer.frame = CGRect(origin: .zero, size: bounds.size)
                clipMaskLayer.path = path
                clipMaskLayer.fillColor = platformColor(.white).cgColor
                clipMaskLayer.contents = nil
                backingLayer.mask = clipMaskLayer
                backingLayer.cornerRadius = 0
                backingLayer.masksToBounds = false
            } else {
                backingLayer.mask = nil
                backingLayer.cornerRadius = 0
                backingLayer.masksToBounds = false
            }
            CATransaction.commit()
            appliedClipSignature = signature
        }

        func applyGeometry(
            size: CGSize,
            localTransform: CGAffineTransform,
            zIndex: Int,
            opacity: Double
        ) {
            let geometry = AppliedGeometry(
                size: size,
                localTransform: localTransform,
                zIndex: zIndex,
                opacity: opacity
            )
            if appliedGeometry == geometry {
                return
            }
            let rect = CGRect(
                origin: CGPoint(x: localTransform.tx, y: localTransform.ty),
                size: size
            )
            let boundsRect = CGRect(origin: .zero, size: size)
            let linearTransform = CGAffineTransform(
                a: localTransform.a,
                b: localTransform.b,
                c: localTransform.c,
                d: localTransform.d,
                tx: 0,
                ty: 0
            )
#if os(macOS)
            let viewBackedTransform = Self.viewBackedTransform(
                size: size,
                linearTransform: linearTransform
            )
            let rectSize = viewBackedTransform.frameSize
#else
            let rectSize = size
#endif
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            frame = CGRect(origin: rect.origin, size: rectSize)
            bounds = boundsRect
            let layer = backingLayer
#if os(macOS)
            if let layerTransform = viewBackedTransform.layerTransform {
                frameRotation = viewBackedTransform.frameRotationDegrees
                layer.setAffineTransform(layerTransform)
            } else {
                // Clear stale layer transforms before using AppKit's transform-aware
                // frame rotation path. Setting this after frameRotation can erase the
                // visual rotation on layer-backed views.
                layer.setAffineTransform(.identity)
                frameRotation = viewBackedTransform.frameRotationDegrees
            }
#else
            layer.setAffineTransform(linearTransform)
#endif
            layer.zPosition = CGFloat(zIndex)
            layer.opacity = Float(opacity)
            CATransaction.commit()
            appliedGeometry = geometry
        }

#if os(macOS)
        private struct ViewBackedTransform {
            let frameSize: CGSize
            let frameRotationDegrees: CGFloat
            let layerTransform: CGAffineTransform?
        }

        private static func viewBackedTransform(
            size: CGSize,
            linearTransform: CGAffineTransform
        ) -> ViewBackedTransform {
            let scaleX = hypot(linearTransform.a, linearTransform.b)
            let scaleY = hypot(linearTransform.c, linearTransform.d)
            let determinant = linearTransform.a * linearTransform.d - linearTransform.b * linearTransform.c
            let columnDot = linearTransform.a * linearTransform.c + linearTransform.b * linearTransform.d
            let shear = abs(columnDot) / max(scaleX * scaleY, CGFloat.ulpOfOne)

            guard scaleX.isFinite,
                  scaleY.isFinite,
                  scaleX > CGFloat.ulpOfOne,
                  scaleY > CGFloat.ulpOfOne,
                  determinant > CGFloat.ulpOfOne,
                  shear < 0.001
            else {
                return ViewBackedTransform(
                    frameSize: size,
                    frameRotationDegrees: 0,
                    layerTransform: linearTransform
                )
            }

            let radians = atan2(linearTransform.b, linearTransform.a)
            return ViewBackedTransform(
                frameSize: CGSize(width: size.width * scaleX, height: size.height * scaleY),
                frameRotationDegrees: radians * 180.0 / .pi,
                layerTransform: nil
            )
        }
#endif
    }

    private static func attachPlatformSubview(_ child: PlatformBaseView, to parent: PlatformBaseView) {
        if child.superview !== parent {
            child.removeFromSuperview()
            parent.addSubview(child)
        }
    }

    private final class PlatformMaskedLeafView: PlatformContainerView {
        private static let maskRasterQueue = DispatchQueue(
            label: "dev.pax.apple.native-mask-raster",
            qos: .userInitiated
        )

        private var currentMaskLayer: CALayer?
#if os(macOS)
        private let snapshotLayer = CALayer()
        private var appliedSnapshotSignature: Int?
        private var snapshotSourceImage: CGImage?
#endif
        private var appliedMaskSignature: UInt64?
        private var appliedMaskSize: CGSize = .zero
        private var requestedMaskSignature: UInt64?
        private var requestedMaskSize: CGSize = .zero
        private var nextMaskGeneration: UInt64 = 0
        private var inFlightMaskRender: PendingMaskRender?
        private var queuedMaskRender: PendingMaskRender?
        private var contentKey: String?
        private var contentView: PlatformBaseView?
        private var appliedContentSignature: Int?
        private var debugLeafId: PaxNodeId = 0

        private static func shouldRasterizeMaskAsynchronously() -> Bool {
#if os(macOS)
            false
#else
            true
#endif
        }

        func update(item: NativeRenderItem) {
            debugLeafId = item.id
            ensureContentView(for: item.kind)
            if let contentView {
                let rect = CGRect(origin: .zero, size: item.size)
                if contentView.frame != rect {
                    contentView.frame = rect
                    contentView.bounds = rect
                }
            }
            let contentSignature = item.kind.contentSignature(size: item.size)
            if let contentView, appliedContentSignature != contentSignature {
                NativeRenderingLayer.updatePlatformLeafView(
                    contentView,
                    for: item.kind,
                    size: item.size
                )
                appliedContentSignature = contentSignature
            }
#if os(macOS)
            updateSnapshot(for: item, contentSignature: contentSignature)
#endif
            updateMask(item.mask)
        }

        private func ensureContentView(for kind: NativeLeafKind) {
            let desiredKey = kind.contentKey
            if contentKey == desiredKey, contentView != nil {
                return
            }

            contentView?.removeFromSuperview()
            let view = NativeRenderingLayer.makePlatformLeafView(for: kind)
            view.frame = bounds
            view.autoresizingMask = NativeRenderingLayer.fillAutoresizingMask()
            NativeRenderingLayer.attachPlatformSubview(view, to: self)
            contentView = view
            contentKey = desiredKey
            appliedContentSignature = nil
#if os(macOS)
            appliedSnapshotSignature = nil
#endif
        }

#if os(macOS)
        private func updateSnapshot(for item: NativeRenderItem, contentSignature: Int) {
            guard let contentView else {
                return
            }
            let shouldUseSnapshot = item.mask != nil
            if !shouldUseSnapshot {
                snapshotLayer.removeFromSuperlayer()
                snapshotLayer.contents = nil
                snapshotLayer.mask = nil
                appliedSnapshotSignature = nil
                snapshotSourceImage = nil
                contentView.isHidden = false
                contentView.alphaValue = 1.0
                contentView.layer?.opacity = 1.0
                return
            }

            if snapshotLayer.superlayer == nil {
                snapshotLayer.zPosition = 1_000
                snapshotLayer.contentsGravity = .resize
                backingLayer.addSublayer(snapshotLayer)
            }

            let rect = CGRect(origin: .zero, size: item.size)
            let scale = Self.currentMaskScale()
            let snapshotSignature = contentSignature ^ Int(bitPattern: UInt(item.id))
            if appliedSnapshotSignature != snapshotSignature {
                let previousAlpha = contentView.alphaValue
                let previousHidden = contentView.isHidden
                contentView.isHidden = false
                contentView.alphaValue = 1.0
                contentView.layer?.opacity = 1.0
                contentView.layoutSubtreeIfNeeded()
                let pixelWidth = max(Int(ceil(item.size.width * scale)), 1)
                let pixelHeight = max(Int(ceil(item.size.height * scale)), 1)
                if let rep = NSBitmapImageRep(
                    bitmapDataPlanes: nil,
                    pixelsWide: pixelWidth,
                    pixelsHigh: pixelHeight,
                    bitsPerSample: 8,
                    samplesPerPixel: 4,
                    hasAlpha: true,
                    isPlanar: false,
                    colorSpaceName: .deviceRGB,
                    bitmapFormat: [],
                    bytesPerRow: 0,
                    bitsPerPixel: 0
                ) {
                    rep.size = item.size
                    contentView.cacheDisplay(in: rect, to: rep)
                    snapshotSourceImage = rep.cgImage
                    snapshotLayer.contents = rep.cgImage
                    appliedSnapshotSignature = snapshotSignature
                }
                contentView.isHidden = previousHidden
                contentView.alphaValue = previousAlpha
            }

            CATransaction.begin()
            CATransaction.setDisableActions(true)
            snapshotLayer.frame = rect
            snapshotLayer.contentsScale = scale
            snapshotLayer.isHidden = false
            CATransaction.commit()
            contentView.isHidden = false
            contentView.alphaValue = 0.0
            contentView.layer?.opacity = 0.0
        }
#endif

        private static func currentMaskScale() -> CGFloat {
            currentNativeMaskScale()
        }

        private func enqueueMaskRender(payload: RasterizedNativeMaskPayload, scale: CGFloat) {
            nextMaskGeneration &+= 1
            let render = PendingMaskRender(
                generation: nextMaskGeneration,
                scale: scale,
                payload: payload
            )
            queuedMaskRender = render
            startNextMaskRenderIfNeeded()
        }

        private func startNextMaskRenderIfNeeded() {
            guard inFlightMaskRender == nil, let render = queuedMaskRender else {
                return
            }
            queuedMaskRender = nil
            inFlightMaskRender = render

            Self.maskRasterQueue.async { [weak self] in
                let image = cachedRasterizedMaskImage(payload: render.payload, scale: render.scale)
                DispatchQueue.main.async {
                    guard let self else {
                        return
                    }
                    guard self.inFlightMaskRender?.generation == render.generation else {
                        return
                    }
                    self.inFlightMaskRender = nil
                   if self.requestedMaskSignature == render.payload.signature,
                      self.requestedMaskSize == render.payload.size,
                      let image
                   {
                        let nextMaskLayer = CALayer()
                        nextMaskLayer.frame = CGRect(origin: .zero, size: render.payload.size)
                        nextMaskLayer.contents = image
                        nextMaskLayer.contentsScale = render.scale
                        nextMaskLayer.contentsGravity = .resize
                        CATransaction.begin()
                        CATransaction.setDisableActions(true)
                        self.backingLayer.mask = nextMaskLayer
                        CATransaction.commit()
                        self.currentMaskLayer = nextMaskLayer
                        self.appliedMaskSignature = render.payload.signature
                        self.appliedMaskSize = render.payload.size
                    }
                    self.startNextMaskRenderIfNeeded()
                }
            }
        }

        private func applyRasterizedMaskImage(
            _ image: CGImage?,
            payload: RasterizedNativeMaskPayload,
            scale: CGFloat
        ) {
            let layer = backingLayer
            NativeMaskDebug.log("apply id=\(debugLeafId) key=\(contentKey ?? "?") sig=\(payload.signature) holes=\(payload.holes.count) size=\(payload.size.width)x\(payload.size.height)")
            let nextMaskLayer: CALayer? = {
                guard let image else {
                    return nil
                }
                let maskLayer = CALayer()
                maskLayer.frame = CGRect(origin: .zero, size: payload.size)
                maskLayer.contents = image
                maskLayer.contentsScale = scale
                maskLayer.contentsGravity = .resize
                return maskLayer
            }()
            CATransaction.begin()
            CATransaction.setDisableActions(true)
#if os(macOS)
            if snapshotLayer.superlayer != nil {
                if let snapshotSourceImage, let image {
                    snapshotLayer.contents = compositedMaskedSnapshotImage(
                        snapshot: snapshotSourceImage,
                        mask: image,
                        size: payload.size,
                        scale: scale
                    ) ?? snapshotSourceImage
                } else {
                    snapshotLayer.contents = nil
                }
                snapshotLayer.mask = nil
                layer.mask = nil
            } else {
                layer.mask = nextMaskLayer
                snapshotLayer.mask = nil
            }
#else
            layer.mask = nextMaskLayer
#endif
            layer.rasterizationScale = scale
            CATransaction.commit()
            currentMaskLayer = nextMaskLayer
            self.appliedMaskSignature = payload.signature
            self.appliedMaskSize = payload.size
        }

        private func updateMask(_ mask: ResolvedNativeMask?) {
            let layer = backingLayer
            guard let mask else {
                NativeMaskDebug.log("clear id=\(debugLeafId) key=\(contentKey ?? "?")")
                requestedMaskSignature = nil
                requestedMaskSize = .zero
                appliedMaskSignature = nil
                appliedMaskSize = .zero
                currentMaskLayer = nil
                CATransaction.begin()
                CATransaction.setDisableActions(true)
                layer.mask = nil
#if os(macOS)
                snapshotLayer.mask = nil
#endif
                CATransaction.commit()
                return
            }

            requestedMaskSignature = mask.signature
            requestedMaskSize = mask.size
            NativeMaskDebug.log("request id=\(debugLeafId) key=\(contentKey ?? "?") sig=\(mask.signature) holes=\(mask.holes.count) size=\(mask.size.width)x\(mask.size.height)")
            if appliedMaskSignature == mask.signature && appliedMaskSize == mask.size {
                return
            }
            if let inFlightMaskRender,
               inFlightMaskRender.payload.signature == mask.signature,
               inFlightMaskRender.payload.size == mask.size
            {
                return
            }
            if let queuedMaskRender,
               queuedMaskRender.payload.signature == mask.signature,
               queuedMaskRender.payload.size == mask.size
            {
                return
            }
            let payload = rasterPayload(from: mask)
            let scale = Self.currentMaskScale()
            if Self.shouldRasterizeMaskAsynchronously() {
                enqueueMaskRender(payload: payload, scale: scale)
            } else {
                inFlightMaskRender = nil
                queuedMaskRender = nil
                let image = cachedRasterizedMaskImage(payload: payload, scale: scale)
                applyRasterizedMaskImage(image, payload: payload, scale: scale)
            }
        }
    }

#if os(iOS) || os(tvOS) || os(watchOS)
    private protocol PlatformScrollerDelegate: UIScrollViewDelegate {}
#elseif os(macOS)
    private protocol PlatformScrollerDelegate {}
#endif

    private final class PlatformScrollerView: PlatformContainerView, PlatformScrollerDelegate {
        private let scrollerId: PaxNodeId
#if os(iOS) || os(tvOS) || os(watchOS)
        private let scrollView = UIScrollView()
        private let innerContentView = UIView()
#elseif os(macOS)
        private final class FlippedContentView: NSView {
            override var isFlipped: Bool { true }
        }
        private let scrollView = NSScrollView()
        private let innerContentView = FlippedContentView()
        private var scrollObserver: NSObjectProtocol?
        private var liveScrollStartObserver: NSObjectProtocol?
        private var liveScrollEndObserver: NSObjectProtocol?
        private var pendingSnapWorkItem: DispatchWorkItem?
        private let macSnapQuietDelay: TimeInterval = 0.22
        private var macSnapTimer: Timer?
        private var isLiveScrolling = false
        private var lastNativeScrollTime: TimeInterval = 0
#endif
        private let canvasHostViewInternal = PlatformContainerView(frame: .zero)
        private let contentHostViewInternal = PlatformContainerView(frame: .zero)
        private var appliedContentSize: CGSize = .zero
        private var appliedScrollEnabledX: Bool?
        private var appliedScrollEnabledY: Bool?
        private var appliedSnapPointsX: [CGFloat] = []
        private var appliedSnapPointsY: [CGFloat] = []
        private var appliedScrollPosition: CGPoint = .zero
#if os(iOS) || os(tvOS) || os(watchOS)
        private var pendingIOSSnapTarget: CGPoint?
        private var isProgrammaticSnapAnimating = false
        private var iosSnapGeneration: UInt64 = 0
        private var iosSnapDisplayLink: CADisplayLink?
        private var iosSnapStartPosition: CGPoint = .zero
        private var iosSnapTargetPosition: CGPoint = .zero
        private var iosSnapStartTime: CFTimeInterval = 0
        private var iosSnapDuration: TimeInterval = 0
#elseif os(macOS)
        private var isMacSnapActive = false
        private var hasNativeSnapPoints: Bool {
            !appliedSnapPointsX.isEmpty || !appliedSnapPointsY.isEmpty
        }
        private var macScrollSequence: UInt64 = 0
        private var macSnapStartPosition: CGPoint = .zero
        private var macSnapTargetPosition: CGPoint = .zero
        private var macSnapStartTime: TimeInterval = 0
        private var macSnapDuration: TimeInterval = 0
#endif
        private var appliedMaskSignature: UInt64?
        private var appliedMaskSize: CGSize = .zero
        private var currentNativeMaskLayer: CALayer?
        private let positiveClipMaskLayer = CAShapeLayer()
        private var appliedPositiveClipSignature: Int?
        private var suppressScrollEvents = false

        var contentHostView: PlatformContainerView { contentHostViewInternal }

        init(id: PaxNodeId) {
            self.scrollerId = id
            super.init(frame: .zero)
#if os(iOS) || os(tvOS) || os(watchOS)
            scrollView.delegate = self
            scrollView.showsVerticalScrollIndicator = false
            scrollView.showsHorizontalScrollIndicator = false
            scrollView.backgroundColor = .clear
            scrollView.isOpaque = false
            scrollView.alwaysBounceVertical = true
            scrollView.alwaysBounceHorizontal = true
            scrollView.clipsToBounds = true
            scrollView.layer.masksToBounds = true
            scrollView.contentInsetAdjustmentBehavior = .never
            innerContentView.backgroundColor = .clear
            innerContentView.isOpaque = false
            scrollView.addSubview(innerContentView)
#elseif os(macOS)
            scrollView.wantsLayer = true
            scrollView.hasVerticalScroller = false
            scrollView.hasHorizontalScroller = false
            scrollView.drawsBackground = false
            scrollView.autohidesScrollers = true
            scrollView.layer?.masksToBounds = true
            scrollView.contentView.wantsLayer = true
            scrollView.contentView.layer?.masksToBounds = true
            innerContentView.wantsLayer = true
            scrollView.contentView.postsBoundsChangedNotifications = true
            scrollView.documentView = innerContentView
            scrollObserver = NotificationCenter.default.addObserver(
                forName: NSView.boundsDidChangeNotification,
                object: scrollView.contentView,
                queue: .main
            ) { [weak self] _ in
                self?.handleScroll()
            }
            liveScrollStartObserver = NotificationCenter.default.addObserver(
                forName: NSScrollView.willStartLiveScrollNotification,
                object: scrollView,
                queue: .main
            ) { [weak self] _ in
                self?.isLiveScrolling = true
                self?.beginMacUserScroll()
            }
            liveScrollEndObserver = NotificationCenter.default.addObserver(
                forName: NSScrollView.didEndLiveScrollNotification,
                object: scrollView,
                queue: .main
            ) { [weak self] _ in
                self?.isLiveScrolling = false
                self?.lastNativeScrollTime = Date().timeIntervalSinceReferenceDate
                // Generic NSScrollView has no UIKit-style target-content-offset
                // hook. Let AppKit finish live/momentum scrolling, then snap the
                // resting clip origin so native momentum does not fight us.
                self?.snapMacScrollPosition(animated: true)
            }
#endif

            canvasHostViewInternal.isHidden = false
            contentHostViewInternal.isHidden = false
#if os(iOS) || os(tvOS) || os(watchOS)
            canvasHostViewInternal.isUserInteractionEnabled = false
            canvasHostViewInternal.layer.zPosition = 0
            contentHostViewInternal.layer.zPosition = 1
#elseif os(macOS)
            canvasHostViewInternal.layer?.zPosition = 0
            contentHostViewInternal.layer?.zPosition = 1
#endif
            innerContentView.addSubview(canvasHostViewInternal)
            innerContentView.addSubview(contentHostViewInternal)
            addSubview(scrollView)

            NativeScrollerHostRegistry.shared.register(
                id: scrollerId,
                canvasHost: canvasHostViewInternal,
                contentHost: contentHostViewInternal
            )
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        deinit {
#if os(macOS)
            if let scrollObserver {
                NotificationCenter.default.removeObserver(scrollObserver)
            }
            if let liveScrollStartObserver {
                NotificationCenter.default.removeObserver(liveScrollStartObserver)
            }
            if let liveScrollEndObserver {
                NotificationCenter.default.removeObserver(liveScrollEndObserver)
            }
#endif
            NativeScrollerHostRegistry.shared.unregister(id: scrollerId)
#if os(macOS)
            cancelMacSnap()
#endif
        }

#if os(iOS) || os(tvOS) || os(watchOS)
        override func layoutSubviews() {
            super.layoutSubviews()
            scrollView.frame = bounds
        }
#elseif os(macOS)
        override func layout() {
            super.layout()
            scrollView.frame = bounds
        }
#endif

        func update(scroller: ScrollerRenderNode) {
            updateScrollEnabled(scroller.scrollEnabledX, scroller.scrollEnabledY)
            updateSnapPoints(x: scroller.snapPointsX, y: scroller.snapPointsY)
            updateContentSize(scroller.contentSize)
            let scrollX = scroller.presentationScrollX.isFinite ? scroller.presentationScrollX : scroller.scrollX
            let scrollY = scroller.presentationScrollY.isFinite ? scroller.presentationScrollY : scroller.scrollY
            updateScrollPosition(CGPoint(x: scrollX, y: scrollY))
#if os(iOS) || os(tvOS) || os(watchOS)
            scrollView.clipsToBounds = scroller.clipContent
            scrollView.layer.cornerRadius = scroller.clipContent ? scroller.borderRadius : 0
            scrollView.layer.masksToBounds = scroller.clipContent
#elseif os(macOS)
            scrollView.contentView.copiesOnScroll = false
            scrollView.layer?.cornerRadius = scroller.clipContent ? scroller.borderRadius : 0
            scrollView.layer?.masksToBounds = scroller.clipContent
            scrollView.contentView.layer?.masksToBounds = scroller.clipContent
#endif
        }

        func updatePositiveClip(paths: [CGPath], size: CGSize) {
            let signature = clipPathSignature(paths, size: size)
            guard appliedPositiveClipSignature != signature else {
                return
            }
            appliedPositiveClipSignature = signature
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            guard !paths.isEmpty else {
#if os(iOS) || os(tvOS) || os(watchOS)
                scrollView.layer.mask = nil
#elseif os(macOS)
                scrollView.layer?.mask = nil
                scrollView.contentView.layer?.mask = nil
#endif
                CATransaction.commit()
                return
            }

            positiveClipMaskLayer.frame = CGRect(origin: .zero, size: size)
            let scale = currentNativeMaskScale()
            positiveClipMaskLayer.contentsScale = scale
            positiveClipMaskLayer.rasterizationScale = scale
            if paths.count == 1, let path = paths.first {
                positiveClipMaskLayer.path = path
                positiveClipMaskLayer.fillColor = platformColor(.white).cgColor
                positiveClipMaskLayer.contents = nil
                positiveClipMaskLayer.contentsGravity = .resize
#if os(iOS) || os(tvOS) || os(watchOS)
                scrollView.layer.mask = positiveClipMaskLayer
#elseif os(macOS)
                scrollView.layer?.mask = positiveClipMaskLayer
                scrollView.contentView.layer?.mask = nil
#endif
            } else {
                if let image = rasterizedPositiveClipMaskImage(paths: paths, size: size, scale: scale) {
                    positiveClipMaskLayer.path = nil
                    positiveClipMaskLayer.fillColor = platformColor(.white).cgColor
                    positiveClipMaskLayer.contents = image
                    positiveClipMaskLayer.contentsScale = scale
                    positiveClipMaskLayer.contentsGravity = .resize
#if os(iOS) || os(tvOS) || os(watchOS)
                    scrollView.layer.mask = positiveClipMaskLayer
#elseif os(macOS)
                    scrollView.layer?.mask = positiveClipMaskLayer
                    scrollView.contentView.layer?.mask = nil
#endif
                }
            }
            CATransaction.commit()
        }

        func updateNativeMask(_ mask: ResolvedNativeMask?) {
            guard let mask else {
                guard appliedMaskSignature != nil || currentNativeMaskLayer != nil else {
                    return
                }
                CATransaction.begin()
                CATransaction.setDisableActions(true)
                backingLayer.mask = nil
                CATransaction.commit()
                appliedMaskSignature = nil
                appliedMaskSize = .zero
                currentNativeMaskLayer = nil
                return
            }

            if appliedMaskSignature == mask.signature && appliedMaskSize == mask.size {
                return
            }

            let payload = rasterPayload(from: mask)
            let scale = currentNativeMaskScale()
            guard let image = cachedRasterizedMaskImage(payload: payload, scale: scale) else {
                return
            }

            let maskLayer = CALayer()
            maskLayer.frame = CGRect(origin: .zero, size: payload.size)
            maskLayer.contents = image
            maskLayer.contentsScale = scale
            maskLayer.contentsGravity = .resize

            CATransaction.begin()
            CATransaction.setDisableActions(true)
            backingLayer.mask = maskLayer
            backingLayer.rasterizationScale = scale
            CATransaction.commit()

            appliedMaskSignature = mask.signature
            appliedMaskSize = mask.size
            currentNativeMaskLayer = maskLayer
        }

        private func updateSnapPoints(x: [CGFloat], y: [CGFloat]) {
            if appliedSnapPointsX != x {
                appliedSnapPointsX = x
            }
            if appliedSnapPointsY != y {
                appliedSnapPointsY = y
            }
#if os(macOS)
            updateMacScrollElasticity()
#endif
        }

        private func nearestSnapPoint(
            to value: CGFloat,
            points: [CGFloat],
            maxOffset: CGFloat
        ) -> CGFloat {
            guard !points.isEmpty else {
                return min(max(0, value), maxOffset)
            }
            var best = min(max(0, points[0]), maxOffset)
            var bestDistance = abs(best - value)
            for point in points.dropFirst() {
                let clamped = min(max(0, point), maxOffset)
                let distance = abs(clamped - value)
                if distance < bestDistance {
                    best = clamped
                    bestDistance = distance
                }
            }
            return best
        }

        private func snappedScrollPosition(_ position: CGPoint) -> CGPoint {
            let viewportSize: CGSize
#if os(iOS) || os(tvOS) || os(watchOS)
            viewportSize = scrollView.bounds.size
#elseif os(macOS)
            viewportSize = scrollView.contentView.bounds.size
#endif
            let maxX = max(0, appliedContentSize.width - viewportSize.width)
            let maxY = max(0, appliedContentSize.height - viewportSize.height)
            let x = appliedSnapPointsX.isEmpty
                ? min(max(0, position.x), maxX)
                : nearestSnapPoint(to: position.x, points: appliedSnapPointsX, maxOffset: maxX)
            let y = appliedSnapPointsY.isEmpty
                ? min(max(0, position.y), maxY)
                : nearestSnapPoint(to: position.y, points: appliedSnapPointsY, maxOffset: maxY)
            return CGPoint(x: x, y: y)
        }

        private func updateContentSize(_ size: CGSize) {
            let safeSize = CGSize(width: max(0, size.width), height: max(0, size.height))
            guard safeSize != appliedContentSize else {
                return
            }
            appliedContentSize = safeSize
            let rect = CGRect(origin: .zero, size: safeSize)
            innerContentView.frame = rect
            canvasHostViewInternal.frame = rect
            contentHostViewInternal.frame = rect
#if os(iOS) || os(tvOS) || os(watchOS)
            scrollView.contentSize = safeSize
#elseif os(macOS)
            innerContentView.setFrameSize(safeSize)
            if scrollView.documentView !== innerContentView {
                scrollView.documentView = innerContentView
            }
            clampMacScrollPositionToContent()
#endif
        }

        private func updateScrollEnabled(_ enableX: Bool, _ enableY: Bool) {
            guard enableX != appliedScrollEnabledX || enableY != appliedScrollEnabledY else {
                return
            }
            appliedScrollEnabledX = enableX
            appliedScrollEnabledY = enableY
#if os(iOS) || os(tvOS) || os(watchOS)
            scrollView.isScrollEnabled = enableX || enableY
            scrollView.alwaysBounceHorizontal = enableX
            scrollView.alwaysBounceVertical = enableY
#elseif os(macOS)
            updateMacScrollElasticity()
            clampMacScrollPositionToContent()
#endif
        }

#if os(macOS)
        private func setMacClipOrigin(_ origin: CGPoint) {
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            scrollView.contentView.setBoundsOrigin(origin)
            scrollView.reflectScrolledClipView(scrollView.contentView)
            CATransaction.commit()
        }

        private func updateMacScrollElasticity() {
            let enableX = appliedScrollEnabledX == true
            let enableY = appliedScrollEnabledY == true
            scrollView.horizontalScrollElasticity = enableX && appliedSnapPointsX.isEmpty
                ? .automatic
                : .none
            scrollView.verticalScrollElasticity = enableY && appliedSnapPointsY.isEmpty
                ? .automatic
                : .none
        }

        private func cancelMacSnap() {
            pendingSnapWorkItem?.cancel()
            pendingSnapWorkItem = nil
            macSnapTimer?.invalidate()
            macSnapTimer = nil
        }

        private func beginMacUserScroll() {
            cancelMacSnap()
            macScrollSequence &+= 1
            if isMacSnapActive {
                scrollView.contentView.layer?.removeAllAnimations()
            }
            suppressScrollEvents = false
            isMacSnapActive = false
        }

        private func scheduleMacSnap(delay: TimeInterval? = nil) {
            guard !appliedSnapPointsX.isEmpty || !appliedSnapPointsY.isEmpty else {
                return
            }
            guard !isLiveScrolling else {
                return
            }
            guard !isMacSnapActive else {
                return
            }
            cancelMacSnap()
            let quietDelay = delay ?? macSnapQuietDelay
            let scheduledSequence = macScrollSequence
            let workItem = DispatchWorkItem { [weak self] in
                guard let self else { return }
                guard scheduledSequence == self.macScrollSequence else {
                    return
                }
                let elapsed = Date().timeIntervalSinceReferenceDate - self.lastNativeScrollTime
                guard elapsed >= quietDelay else {
                    self.scheduleMacSnap(delay: quietDelay - elapsed)
                    return
                }
                self.snapMacScrollPosition(animated: true)
            }
            pendingSnapWorkItem = workItem
            DispatchQueue.main.asyncAfter(deadline: .now() + quietDelay, execute: workItem)
        }

        private func snapMacScrollPosition(animated: Bool) {
            guard !appliedSnapPointsX.isEmpty || !appliedSnapPointsY.isEmpty else {
                return
            }
            guard !isMacSnapActive else {
                return
            }
            let current = scrollView.contentView.bounds.origin
            let target = snappedScrollPosition(current)
            guard shouldApplyScrollPosition(target, current: current) else {
                return
            }

            cancelMacSnap()
            let distance = hypot(target.x - current.x, target.y - current.y)
            appliedScrollPosition = target
            isMacSnapActive = true
            suppressScrollEvents = false
            macSnapStartPosition = current
            macSnapTargetPosition = target

            if !animated || distance <= 0.5 {
                setMacClipOrigin(target)
                finishMacSnap()
                return
            }

            macSnapDuration = min(max(Double(distance / 2400.0), 0.14), 0.34)
            macSnapStartTime = CACurrentMediaTime()
            let timer = Timer(timeInterval: 1.0 / 60.0, repeats: true) { [weak self] timer in
                self?.stepMacSnapAnimation(timer)
            }
            macSnapTimer = timer
            RunLoop.main.add(timer, forMode: .common)
        }

        private func stepMacSnapAnimation(_ timer: Timer) {
            guard isMacSnapActive else {
                timer.invalidate()
                if macSnapTimer === timer {
                    macSnapTimer = nil
                }
                return
            }
            let elapsed = CACurrentMediaTime() - macSnapStartTime
            let rawProgress = macSnapDuration <= 0 ? 1 : min(max(elapsed / macSnapDuration, 0), 1)
            let easedProgress = 1 - pow(1 - rawProgress, 3)
            let next = CGPoint(
                x: macSnapStartPosition.x
                    + (macSnapTargetPosition.x - macSnapStartPosition.x) * easedProgress,
                y: macSnapStartPosition.y
                    + (macSnapTargetPosition.y - macSnapStartPosition.y) * easedProgress
            )
            setMacClipOrigin(next)
            if rawProgress >= 1 {
                finishMacSnap()
            }
        }

        private func finishMacSnap() {
            macSnapTimer?.invalidate()
            macSnapTimer = nil
            let target = macSnapTargetPosition
            if shouldApplyScrollPosition(target, current: scrollView.contentView.bounds.origin) {
                setMacClipOrigin(target)
            }
            isMacSnapActive = false
            let now = Date().timeIntervalSinceReferenceDate
            lastNativeScrollTime = now
            appliedScrollPosition = target
            dispatchScrollbarChange(
                id: scrollerId,
                scrollX: Double(target.x),
                scrollY: Double(target.y),
                presentationScrollX: Double(target.x),
                presentationScrollY: Double(target.y)
            )
        }

        private func clampMacScrollPositionToContent() {
            let viewportSize = scrollView.contentView.bounds.size
            let maxX = max(0, appliedContentSize.width - viewportSize.width)
            let maxY = max(0, appliedContentSize.height - viewportSize.height)
            let current = scrollView.contentView.bounds.origin
            let clamped = NSPoint(
                x: min(max(0, current.x), maxX),
                y: min(max(0, current.y), maxY)
            )
            guard shouldApplyScrollPosition(clamped, current: current) else {
                return
            }
            suppressScrollEvents = true
            setMacClipOrigin(clamped)
            suppressScrollEvents = false
            appliedScrollPosition = clamped
        }
#endif

        private func updateScrollPosition(_ position: CGPoint) {
            appliedScrollPosition = position
#if os(iOS) || os(tvOS) || os(watchOS)
            guard shouldApplyScrollPosition(position, current: scrollView.contentOffset) else {
                return
            }
            if isProgrammaticSnapAnimating {
                return
            }
            if scrollView.isTracking || scrollView.isDragging || scrollView.isDecelerating {
                return
            }
            suppressScrollEvents = true
            scrollView.setContentOffset(position, animated: false)
            suppressScrollEvents = false
#elseif os(macOS)
            let viewportSize = scrollView.contentView.bounds.size
            let target = NSPoint(
                x: min(max(0, position.x), max(0, appliedContentSize.width - viewportSize.width)),
                y: min(max(0, position.y), max(0, appliedContentSize.height - viewportSize.height))
            )
            guard shouldApplyScrollPosition(target, current: scrollView.contentView.bounds.origin) else {
                return
            }
            if Date().timeIntervalSinceReferenceDate - lastNativeScrollTime < macSnapQuietDelay {
                return
            }
            if isMacSnapActive {
                return
            }
            suppressScrollEvents = true
            setMacClipOrigin(target)
            suppressScrollEvents = false
#endif
        }

        private func shouldApplyScrollPosition(_ target: CGPoint, current: CGPoint) -> Bool {
            abs(target.x - current.x) > 0.5 || abs(target.y - current.y) > 0.5
        }

#if os(iOS) || os(tvOS) || os(watchOS)
        private func cancelIOSSnap() {
            iosSnapGeneration &+= 1
            iosSnapDisplayLink?.invalidate()
            iosSnapDisplayLink = nil
            isProgrammaticSnapAnimating = false
            suppressScrollEvents = false
        }

        private func snapIOSScrollPosition(to target: CGPoint, animated: Bool) {
            guard !appliedSnapPointsX.isEmpty || !appliedSnapPointsY.isEmpty else {
                isProgrammaticSnapAnimating = false
                return
            }
            let current = scrollView.contentOffset
            guard shouldApplyScrollPosition(target, current: current) else {
                isProgrammaticSnapAnimating = false
                suppressScrollEvents = false
                return
            }
            iosSnapDisplayLink?.invalidate()
            isProgrammaticSnapAnimating = true
            suppressScrollEvents = false
            iosSnapGeneration &+= 1
            iosSnapStartPosition = current
            iosSnapTargetPosition = target
            if !animated {
                scrollView.contentOffset = target
                finishIOSSnap()
                return
            }
            let distance = hypot(target.x - current.x, target.y - current.y)
            iosSnapDuration = min(max(Double(distance / 2200.0), 0.16), 0.42)
            iosSnapStartTime = CACurrentMediaTime()
            let displayLink = CADisplayLink(target: self, selector: #selector(stepIOSSnapAnimation(_:)))
            iosSnapDisplayLink = displayLink
            displayLink.add(to: .main, forMode: .common)
        }

        @objc private func stepIOSSnapAnimation(_ displayLink: CADisplayLink) {
            guard isProgrammaticSnapAnimating else {
                displayLink.invalidate()
                if iosSnapDisplayLink === displayLink {
                    iosSnapDisplayLink = nil
                }
                return
            }
            let elapsed = CACurrentMediaTime() - iosSnapStartTime
            let rawProgress = iosSnapDuration <= 0 ? 1 : min(max(elapsed / iosSnapDuration, 0), 1)
            let easedProgress = 1 - pow(1 - rawProgress, 3)
            let next = CGPoint(
                x: iosSnapStartPosition.x
                    + (iosSnapTargetPosition.x - iosSnapStartPosition.x) * easedProgress,
                y: iosSnapStartPosition.y
                    + (iosSnapTargetPosition.y - iosSnapStartPosition.y) * easedProgress
            )
            scrollView.contentOffset = next
            if rawProgress >= 1 {
                finishIOSSnap()
            }
        }

        private func finishIOSSnap() {
            iosSnapDisplayLink?.invalidate()
            iosSnapDisplayLink = nil
            let target = iosSnapTargetPosition
            if shouldApplyScrollPosition(target, current: scrollView.contentOffset) {
                scrollView.contentOffset = target
            }
            appliedScrollPosition = target
            suppressScrollEvents = false
            isProgrammaticSnapAnimating = false
            dispatchScrollbarChange(
                id: scrollerId,
                scrollX: Double(target.x),
                scrollY: Double(target.y),
                presentationScrollX: Double(target.x),
                presentationScrollY: Double(target.y)
            )
        }

        func scrollViewWillBeginDragging(_ scrollView: UIScrollView) {
            cancelIOSSnap()
            pendingIOSSnapTarget = nil
            scrollView.layer.removeAllAnimations()
        }

        func scrollViewWillEndDragging(
            _ scrollView: UIScrollView,
            withVelocity velocity: CGPoint,
            targetContentOffset: UnsafeMutablePointer<CGPoint>
        ) {
            guard !appliedSnapPointsX.isEmpty || !appliedSnapPointsY.isEmpty else {
                return
            }
            let projected = targetContentOffset.pointee
            pendingIOSSnapTarget = snappedScrollPosition(projected)
            isProgrammaticSnapAnimating = true
            // We drive the snap animation ourselves instead of mutating the target
            // to the snap point, because UIKit occasionally applies that mutation
            // as a one-frame jump for low-velocity releases.
            targetContentOffset.pointee = scrollView.contentOffset
        }

        func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
            if let target = pendingIOSSnapTarget {
                pendingIOSSnapTarget = nil
                DispatchQueue.main.async { [weak self] in
                    self?.snapIOSScrollPosition(to: target, animated: true)
                }
            }
        }

        func scrollViewDidEndScrollingAnimation(_ scrollView: UIScrollView) {
            isProgrammaticSnapAnimating = false
            suppressScrollEvents = false
        }

        func scrollViewDidScroll(_ scrollView: UIScrollView) {
            guard !suppressScrollEvents else {
                return
            }
            let offset = scrollView.contentOffset
            appliedScrollPosition = offset
            dispatchScrollbarChange(
                id: scrollerId,
                scrollX: Double(offset.x),
                scrollY: Double(offset.y),
                presentationScrollX: Double(offset.x),
                presentationScrollY: Double(offset.y)
            )
        }
#elseif os(macOS)
        private func handleScroll() {
            guard !suppressScrollEvents else {
                return
            }
            let origin = scrollView.contentView.bounds.origin
            let now = Date().timeIntervalSinceReferenceDate
            appliedScrollPosition = origin
            if !isMacSnapActive {
                lastNativeScrollTime = now
                macScrollSequence &+= 1
            }
            dispatchScrollbarChange(
                id: scrollerId,
                scrollX: Double(origin.x),
                scrollY: Double(origin.y),
                presentationScrollX: Double(origin.x),
                presentationScrollY: Double(origin.y)
            )
            if hasNativeSnapPoints && !isMacSnapActive && !isLiveScrolling {
                scheduleMacSnap(delay: macSnapQuietDelay)
            }
        }
#endif
    }

    private final class NativeSceneHostView: PlatformContainerView {
        private var frameViews: [PaxNodeId: PlatformContainerView] = [:]
        private var leafViews: [PaxNodeId: PlatformMaskedLeafView] = [:]
        private var scrollerViews: [PaxNodeId: PlatformScrollerView] = [:]
        private var currentNodes: [NativeRenderNode] = []

#if os(iOS) || os(tvOS) || os(watchOS)
        override init(frame: CGRect) {
            super.init(frame: frame)
            autoresizingMask = NativeRenderingLayer.fillAutoresizingMask()
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        override func didMoveToSuperview() {
            super.didMoveToSuperview()
            syncToSuperviewBoundsIfNeeded()
        }

        override func layoutSubviews() {
            super.layoutSubviews()
            syncToSuperviewBoundsIfNeeded()
        }

        private func syncToSuperviewBoundsIfNeeded() {
            guard let superview else {
                return
            }
            let targetBounds = superview.bounds
            let targetFrame = CGRect(origin: .zero, size: targetBounds.size)
            if frame != targetFrame {
                frame = targetFrame
            }
            if bounds.size != targetBounds.size {
                bounds = CGRect(origin: .zero, size: targetBounds.size)
            }
        }
#endif

        func update(nodes: [NativeRenderNode]) {
            currentNodes = nodes
#if os(iOS) || os(tvOS) || os(watchOS)
            syncToSuperviewBoundsIfNeeded()
#endif
            refreshScene()
        }

        private func refreshScene() {
            var activeFrames = Set<PaxNodeId>()
            var activeLeaves = Set<PaxNodeId>()
            var activeScrollers = Set<PaxNodeId>()
            sync(
                nodes: currentNodes,
                parentView: self,
                activeFrames: &activeFrames,
                activeLeaves: &activeLeaves,
                activeScrollers: &activeScrollers,
                positiveClipPaths: []
            )
            pruneInactiveNodes(
                activeFrames: activeFrames,
                activeLeaves: activeLeaves,
                activeScrollers: activeScrollers
            )
        }

        private func containsScroller(_ nodes: [NativeRenderNode]) -> Bool {
            nodes.contains { node in
                switch node {
                case .scroller:
                    return true
                case .frame(let frame):
                    return containsScroller(frame.children)
                case .item:
                    return false
                }
            }
        }

        private func sync(
            nodes: [NativeRenderNode],
            parentView: PlatformContainerView,
            activeFrames: inout Set<PaxNodeId>,
            activeLeaves: inout Set<PaxNodeId>,
            activeScrollers: inout Set<PaxNodeId>,
            positiveClipPaths: [CGPath]
        ) {
            for node in nodes {
                switch node {
                case .frame(let frame):
                    let inheritedFrameClips = transformedClipPaths(
                        positiveClipPaths,
                        by: safeInverseTransform(frame.localTransform)
                    )
                    var childClipPaths = inheritedFrameClips
                    activeFrames.insert(frame.id)
                    let frameView = frameViews[frame.id] ?? {
                        let view = PlatformContainerView(frame: .zero)
                        frameViews[frame.id] = view
                        return view
                    }()
                    NativeRenderingLayer.attachPlatformSubview(frameView, to: parentView)
                    frameView.applyGeometry(
                        size: frame.size,
                        localTransform: frame.localTransform,
                        zIndex: frame.zIndex,
                        opacity: frame.opacity
                    )
                    let clipPathForContainer = containsScroller(frame.children) ? nil : frame.clipPath
                    frameView.applyClip(
                        path: clipPathForContainer,
                        signature: clipPathForContainer == nil ? frame.clipSignature ^ 0x5F3759DF : frame.clipSignature,
                        borderRadius: frame.borderRadius,
                        clipContent: frame.clipContent
                    )
                    if let clipPath = frame.clipPath {
                        childClipPaths.append(clipPath)
                    }
                    sync(
                        nodes: frame.children,
                        parentView: frameView,
                        activeFrames: &activeFrames,
                        activeLeaves: &activeLeaves,
                        activeScrollers: &activeScrollers,
                        positiveClipPaths: childClipPaths
                    )
                case .scroller(let scroller):
                    let scrollerClipPaths = transformedClipPaths(
                        positiveClipPaths,
                        by: safeInverseTransform(scroller.localTransform)
                    )
                    activeScrollers.insert(scroller.id)
                    let scrollerView = scrollerViews[scroller.id] ?? {
                        let view = PlatformScrollerView(id: scroller.id)
                        scrollerViews[scroller.id] = view
                        return view
                    }()
                    NativeRenderingLayer.attachPlatformSubview(scrollerView, to: parentView)
                    scrollerView.applyGeometry(
                        size: scroller.size,
                        localTransform: scroller.localTransform,
                        zIndex: scroller.zIndex,
                        opacity: scroller.opacity
                    )
                    scrollerView.updatePositiveClip(paths: scrollerClipPaths, size: scroller.size)
                    if scroller.mask == nil {
                        scrollerView.applyClip(
                            path: nil,
                            signature: scroller.clipSignature,
                            borderRadius: scroller.borderRadius,
                            clipContent: scroller.clipContent
                        )
                    }
                    scrollerView.updateNativeMask(scroller.mask)
                    scrollerView.update(scroller: scroller)
                    sync(
                        nodes: scroller.children,
                        parentView: scrollerView.contentHostView,
                        activeFrames: &activeFrames,
                        activeLeaves: &activeLeaves,
                        activeScrollers: &activeScrollers,
                        positiveClipPaths: []
                    )
                case .item(let item):
                    activeLeaves.insert(item.id)
                    let leafView = leafViews[item.id] ?? {
                        let view = PlatformMaskedLeafView(frame: .zero)
                        leafViews[item.id] = view
                        return view
                    }()
                    NativeRenderingLayer.attachPlatformSubview(leafView, to: parentView)
                    leafView.applyGeometry(
                        size: item.size,
                        localTransform: item.localTransform,
                        zIndex: item.zIndex,
                        opacity: item.opacity
                    )
                    leafView.update(item: item)
                }
            }
        }

        private func pruneInactiveNodes(
            activeFrames: Set<PaxNodeId>,
            activeLeaves: Set<PaxNodeId>,
            activeScrollers: Set<PaxNodeId>
        ) {
            for (id, view) in frameViews where !activeFrames.contains(id) {
                view.removeFromSuperview()
                frameViews.removeValue(forKey: id)
            }
            for (id, leafView) in leafViews where !activeLeaves.contains(id) {
                leafView.removeFromSuperview()
                leafViews.removeValue(forKey: id)
            }
            for (id, scrollerView) in scrollerViews where !activeScrollers.contains(id) {
                scrollerView.removeFromSuperview()
                scrollerViews.removeValue(forKey: id)
                NativeScrollerHostRegistry.shared.unregister(id: id)
            }
        }
    }

#if os(iOS) || os(tvOS) || os(watchOS)
    private struct PlatformNativeSceneView: UIViewRepresentable {
        let nodes: [NativeRenderNode]

        func makeUIView(context: Context) -> NativeSceneHostView {
            NativeSceneHostView(frame: .zero)
        }

        func updateUIView(_ view: NativeSceneHostView, context: Context) {
            view.update(nodes: nodes)
        }
    }
#elseif os(macOS)
    private struct PlatformNativeSceneView: NSViewRepresentable {
        let nodes: [NativeRenderNode]

        func makeNSView(context: Context) -> NativeSceneHostView {
            NativeSceneHostView(frame: .zero)
        }

        func updateNSView(_ view: NativeSceneHostView, context: Context) {
            view.update(nodes: nodes)
        }
    }
#endif

    private func clampOpacity(_ opacity: Double) -> Double {
        min(max(opacity, 0.0), 1.0)
    }

    private func sortedTextElements() -> [TextElement] {
        Array(textElements.elements.values)
            .sorted { lhs, rhs in
            if lhs.zIndex == rhs.zIndex {
                return lhs.id < rhs.id
            }
            return lhs.zIndex < rhs.zIndex
        }
    }

    private func sortedElements<T: NativePositionElement>(_ elements: [PaxNodeId: T]) -> [T] {
        Array(elements.values)
            .sorted { lhs, rhs in
            if lhs.zIndex == rhs.zIndex {
                return lhs.id < rhs.id
            }
            return lhs.zIndex < rhs.zIndex
        }
    }

    private func sortedRenderItems() -> [NativeRenderItem] {
        var items: [NativeRenderItem] = []

        items.append(contentsOf: sortedTextElements().map { element in
            textItem(for: element)
        })
        items.append(contentsOf: sortedElements(nativeImageElements.elements).map { element in
            nativeImageItem(for: element)
        })
        items.append(contentsOf: sortedElements(youtubeVideoElements.elements).map { element in
            youtubeVideoItem(for: element)
        })
        items.append(contentsOf: sortedElements(buttonElements.elements).map { element in
            buttonItem(for: element)
        })
        items.append(contentsOf: sortedElements(checkboxElements.elements).map { element in
            checkboxItem(for: element)
        })
        items.append(contentsOf: sortedElements(sliderElements.elements).map { element in
            sliderItem(for: element)
        })
        items.append(contentsOf: sortedElements(dropdownElements.elements).map { element in
            dropdownItem(for: element)
        })
        items.append(contentsOf: sortedElements(radioListElements.elements).map { element in
            radioListItem(for: element)
        })
        items.append(contentsOf: sortedElements(textboxElements.elements).map { element in
            textboxItem(for: element)
        })
        items.append(contentsOf: sortedElements(eventBlockerElements.elements).map { element in
            eventBlockerItem(for: element)
        })

        return items.sorted { lhs, rhs in
            if lhs.zIndex == rhs.zIndex {
                return lhs.id < rhs.id
            }
            return lhs.zIndex < rhs.zIndex
        }
    }

    private func renderItem(element: NativePositionElement, kind: NativeLeafKind) -> NativeRenderItem {
        let size = resolvedSize(element)
        return NativeRenderItem(
            id: element.id,
            zIndex: element.zIndex,
            parentFrame: element.parentFrame,
            localTransform: affineTransform(from: element.transform)
                .concatenating(safeInverse(parentFrameTransform(element.parentFrame))),
            size: size,
            opacity: clampOpacity(element.opacity),
            kind: kind,
            mask: resolvedNativeMask(for: element.id)
        )
    }

    private func renderTextItem(_ element: TextElement, width: CGFloat, height: CGFloat) -> NativeRenderItem {
        return NativeRenderItem(
            id: element.id,
            zIndex: element.zIndex,
            parentFrame: element.parentFrame,
            localTransform: affineTransform(from: element.transform)
                .concatenating(safeInverse(parentFrameTransform(element.parentFrame))),
            size: CGSize(width: width, height: height),
            opacity: clampOpacity(element.opacity),
            kind: .text(element),
            mask: resolvedNativeMask(for: element.id)
        )
    }

    private func parentFrameTransform(_ parentFrame: PaxNodeId?) -> CGAffineTransform {
        guard let parentFrame else {
            return .identity
        }
        if let frame = frameElements.elements[parentFrame] {
            return affineTransform(from: frame.transform)
        }
        if let scroller = scrollerElements.elements[parentFrame] {
            return affineTransform(from: scroller.transform)
        }
        return .identity
    }

    private func safeInverse(_ transform: CGAffineTransform) -> CGAffineTransform {
        let determinant = (transform.a * transform.d) - (transform.b * transform.c)
        guard abs(determinant) > .ulpOfOne else {
            return .identity
        }
        return transform.inverted()
    }

    private func frameTransformInParentFrame(_ frame: FrameElement) -> CGAffineTransform {
        let parentInverse = safeInverse(parentFrameTransform(frame.parentFrame))
        return affineTransform(from: frame.transform).concatenating(parentInverse)
    }

    private func frameSize(_ frame: FrameElement) -> CGSize {
        CGSize(width: max(0, CGFloat(frame.size_x)), height: max(0, CGFloat(frame.size_y)))
    }

    private func scrollerSize(_ scroller: ScrollerElement) -> CGSize {
        CGSize(width: max(0, CGFloat(scroller.size_x)), height: max(0, CGFloat(scroller.size_y)))
    }

    private func scrollerContentSize(_ scroller: ScrollerElement) -> CGSize {
        CGSize(
            width: max(0, CGFloat(scroller.sizeInnerPaneX)),
            height: max(0, CGFloat(scroller.sizeInnerPaneY))
        )
    }

    private func localClipPath(for frame: FrameElement) -> Path? {
        guard frame.clipContent else {
            return nil
        }
        if frame.borderRadius > 0 {
            return nil
        }
        let size = frameSize(frame)
        if let clipPath = frame.clipPath,
           !clipPath.isEmpty,
           let worldPath = parseSVGPath(clipPath) {
            let localPath = worldPath.applying(safeInverse(affineTransform(from: frame.transform)))
            return localPath
        }
        return Path(CGRect(origin: .zero, size: size))
    }

    private func clipSignature(for frame: FrameElement) -> Int {
        guard frame.clipContent else {
            return 0
        }
        var hasher = Hasher()
        hasher.combine(frame.clipContent)
        hasher.combine(frame.borderRadius)
        if frame.borderRadius <= 0 {
            hasher.combine(frame.clipPath)
        }
        combineCGSize(frameSize(frame), into: &hasher)
        return hasher.finalize()
    }

    private func clipSignature(for scroller: ScrollerElement) -> Int {
        guard scroller.clipContent else {
            return 0
        }
        var hasher = Hasher()
        hasher.combine(scroller.clipContent)
        hasher.combine(scroller.borderRadius)
        combineCGSize(scrollerSize(scroller), into: &hasher)
        return hasher.finalize()
    }

    private func sortedNodes(_ nodes: [NativeRenderNode]) -> [NativeRenderNode] {
        nodes.sorted { lhs, rhs in
            if lhs.zIndex == rhs.zIndex {
                return lhs.numericId < rhs.numericId
            }
            return lhs.zIndex < rhs.zIndex
        }
    }

    private func buildRenderTree() -> [NativeRenderNode] {
        let items = sortedRenderItems()
        let itemsByParent = Dictionary(grouping: items, by: { $0.parentFrame })
        let scrollers = sortedElements(scrollerElements.elements)
        let scrollersByParent = Dictionary(grouping: scrollers, by: { $0.parentFrame })
        let framesByParent = Dictionary(
            grouping: Array(frameElements.elements.values),
            by: { $0.parentFrame }
        )

        var activeFrames: [PaxNodeId: Bool] = [:]
        func frameHasNativeDescendants(_ frameId: PaxNodeId) -> Bool {
            if let cached = activeFrames[frameId] {
                return cached
            }
            let hasItems = !(itemsByParent[frameId] ?? []).isEmpty
            let hasScrollers = !(scrollersByParent[frameId] ?? []).isEmpty
            let hasDescendants = (framesByParent[frameId] ?? []).contains { frame in
                frameHasNativeDescendants(frame.id)
            }
            let isActive = hasItems || hasScrollers || hasDescendants
            activeFrames[frameId] = isActive
            return isActive
        }

        func buildFrameNode(_ frame: FrameElement) -> FrameRenderNode {
            let children = buildChildren(parent: frame.id)
            return FrameRenderNode(
                id: frame.id,
                zIndex: frame.zIndex,
                parentFrame: frame.parentFrame,
                localTransform: frameTransformInParentFrame(frame),
                size: frameSize(frame),
                opacity: clampOpacity(frame.opacity),
                clipContent: frame.clipContent,
                borderRadius: CGFloat(frame.borderRadius),
                clipPath: localClipPath(for: frame)?.cgPath,
                clipSignature: clipSignature(for: frame),
                children: children
            )
        }

        func buildScrollerNode(_ scroller: ScrollerElement) -> ScrollerRenderNode {
            let children = buildChildren(parent: scroller.id)
            let snapPointsX = scroller.snapPointsX.map { CGFloat($0) }
            let snapPointsY = scroller.snapPointsY.map { CGFloat($0) }
            return ScrollerRenderNode(
                id: scroller.id,
                zIndex: scroller.zIndex,
                parentFrame: scroller.parentFrame,
                localTransform: affineTransform(from: scroller.transform)
                    .concatenating(safeInverse(parentFrameTransform(scroller.parentFrame))),
                size: scrollerSize(scroller),
                opacity: clampOpacity(scroller.opacity),
                clipContent: scroller.clipContent,
                borderRadius: CGFloat(scroller.borderRadius),
                clipSignature: clipSignature(for: scroller),
                contentSize: scrollerContentSize(scroller),
                scrollX: scroller.scrollX,
                scrollY: scroller.scrollY,
                presentationScrollX: scroller.presentationScrollX,
                presentationScrollY: scroller.presentationScrollY,
                scrollEnabledX: scroller.scrollEnabledX,
                scrollEnabledY: scroller.scrollEnabledY,
                snapPointsX: snapPointsX,
                snapPointsY: snapPointsY,
                mask: resolvedNativeMask(for: scroller.id),
                children: children
            )
        }

        func buildChildren(parent: PaxNodeId?) -> [NativeRenderNode] {
            let frameNodes = (framesByParent[parent] ?? [])
                .filter { frameHasNativeDescendants($0.id) }
                .map { NativeRenderNode.frame(buildFrameNode($0)) }
            let scrollerNodes = (scrollersByParent[parent] ?? [])
                .map { NativeRenderNode.scroller(buildScrollerNode($0)) }
            let itemNodes = (itemsByParent[parent] ?? []).map(NativeRenderNode.item)
            return sortedNodes(frameNodes + scrollerNodes + itemNodes)
        }

        return buildChildren(parent: nil)
    }

    private func renderTree(for generation: UInt64) -> [NativeRenderNode] {
        let cache = Self.renderTreeCache
        if cache.generation != generation {
            cache.nodes = buildRenderTree()
            cache.generation = generation
        }
        return cache.nodes
    }

    private func textItem(for element: TextElement) -> NativeRenderItem {
        let measuredWidth = element.size_x >= 0 ? CGFloat(element.size_x) : element.lastMeasuredSize?.width ?? 0
        let measuredHeight = element.size_y >= 0 ? CGFloat(element.size_y) : element.lastMeasuredSize?.height ?? 0

        return renderTextItem(element, width: measuredWidth, height: measuredHeight)
    }

    private func buttonItem(for element: ButtonElement) -> NativeRenderItem {
        renderItem(element: element, kind: .button(element))
    }

    private func checkboxItem(for element: CheckboxElement) -> NativeRenderItem {
        renderItem(element: element, kind: .checkbox(element))
    }

    private func sliderItem(for element: SliderElement) -> NativeRenderItem {
        renderItem(element: element, kind: .slider(element))
    }

    private func dropdownItem(for element: DropdownElement) -> NativeRenderItem {
        renderItem(element: element, kind: .dropdown(element))
    }

    private func radioListItem(for element: RadioListElement) -> NativeRenderItem {
        renderItem(element: element, kind: .radioList(element))
    }

    private func textboxItem(for element: TextboxElement) -> NativeRenderItem {
        renderItem(element: element, kind: .textbox(element))
    }

    private func nativeImageItem(for element: NativeImageElement) -> NativeRenderItem {
        renderItem(element: element, kind: .nativeImage(element))
    }

    private func youtubeVideoItem(for element: YoutubeVideoElement) -> NativeRenderItem {
        renderItem(element: element, kind: .youtubeVideo(element))
    }

    private func eventBlockerItem(for element: EventBlockerElement) -> NativeRenderItem {
        renderItem(element: element, kind: .eventBlocker(element))
    }

    public var body: some View {
        let generation = nativeSceneInvalidation.generation
        PlatformNativeSceneView(nodes: renderTree(for: generation))
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .transaction { transaction in
            transaction.animation = nil
            transaction.disablesAnimations = true
        }
    }
}

private final class FontRegistrationObserver {
    static let shared = FontRegistrationObserver()
    private var observerInstalled = false
    private var invalidationPending = false

    private init() {
        installIfNeeded()
    }

    private func installIfNeeded() {
        guard !observerInstalled else {
            return
        }
        observerInstalled = true
        NotificationCenter.default.addObserver(
            forName: .paxFontRegistered,
            object: nil,
            queue: .main
        ) { _ in
            self.scheduleInvalidationIfNeeded()
        }
    }

    private func scheduleInvalidationIfNeeded() {
        guard !invalidationPending else {
            return
        }
        invalidationPending = true
        DispatchQueue.main.async {
            self.invalidationPending = false
            NativeSceneInvalidation.singleton.invalidate()
        }
    }
}

public class NativeSceneInvalidation: ObservableObject {
    public static let singleton = NativeSceneInvalidation()
    @Published public var generation: UInt64 = 0

    public func invalidate() {
        generation &+= 1
    }
}

private func nativeAttributedString(for element: TextElement) -> AttributedString {
    var attributedString: AttributedString
    if element.markdown {
        attributedString = (try? AttributedString(
            markdown: element.content,
            options: AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
        )) ?? AttributedString(element.content)
    } else {
        attributedString = AttributedString(element.content)
    }

    for run in attributedString.runs {
        if run.link != nil, let linkStyle = element.style_link {
            attributedString[run.range].font = linkStyle.font.getFont(size: linkStyle.font_size)
            attributedString[run.range].underlineStyle = linkStyle.underline ? .single : .none
            attributedString[run.range].foregroundColor = linkStyle.fill
        }
    }

    return attributedString
}

private func reportMeasuredTextSizeIfNeeded(_ measuredSize: CGSize, for element: TextElement) {
    guard element.size_x < 0 || element.size_y < 0 else {
        return
    }

    let resolvedSize = CGSize(
        width: element.size_x >= 0 ? CGFloat(element.size_x) : ceil(max(0, measuredSize.width)),
        height: element.size_y >= 0 ? CGFloat(element.size_y) : ceil(max(0, measuredSize.height))
    )

    if let previous = element.lastMeasuredSize,
       abs(previous.width - resolvedSize.width) < 0.5,
       abs(previous.height - resolvedSize.height) < 0.5 {
        return
    }

    element.lastMeasuredSize = resolvedSize
    dispatchChassisResizeRequest(
        id: element.id,
        width: Double(resolvedSize.width),
        height: Double(resolvedSize.height)
    )
}

fileprivate extension NativeRenderingLayer {
    static func fillAutoresizingMask() -> PlatformBaseView.AutoresizingMask {
#if os(iOS) || os(tvOS) || os(watchOS)
        [.flexibleWidth, .flexibleHeight]
#elseif os(macOS)
        [.width, .height]
#endif
    }

    static func makePlatformLeafView(for kind: NativeLeafKind) -> PlatformBaseView {
        switch kind {
        case .text:
            return PaxNativeTextLeafView()
        case .button:
            return PaxNativeButtonView()
        case .checkbox:
            return PaxNativeCheckboxView()
        case .slider:
            return PaxNativeSliderView()
        case .dropdown:
#if os(iOS) || os(tvOS) || os(watchOS)
            return PaxNativeDropdownView()
#elseif os(macOS)
            return PaxNativeDropdownView(frame: .zero, pullsDown: false)
#endif
        case .radioList:
            return PaxNativeRadioListView()
        case .textbox(let element):
            return element.isTextArea ? PaxNativeTextboxAreaView() : PaxNativeTextboxFieldView()
        case .nativeImage:
            return PaxNativeImageView(frame: .zero)
        case .youtubeVideo:
            return PaxNativeYoutubeView()
        case .eventBlocker:
            return PaxNativeEventBlockerView()
        }
    }

    static func updatePlatformLeafView(
        _ view: PlatformBaseView,
        for kind: NativeLeafKind,
        size: CGSize
    ) {
        switch kind {
        case .text(let element):
            (view as? PaxNativeTextLeafView)?.apply(element: element, size: size)
        case .button(let element):
            (view as? PaxNativeButtonView)?.apply(element: element)
        case .checkbox(let element):
            (view as? PaxNativeCheckboxView)?.apply(element: element, size: size)
        case .slider(let element):
            (view as? PaxNativeSliderView)?.apply(element: element)
        case .dropdown(let element):
            (view as? PaxNativeDropdownView)?.apply(element: element)
        case .radioList(let element):
            (view as? PaxNativeRadioListView)?.apply(element: element)
        case .textbox(let element):
            if element.isTextArea {
                (view as? PaxNativeTextboxAreaView)?.apply(element: element)
            } else {
                (view as? PaxNativeTextboxFieldView)?.apply(element: element)
            }
        case .nativeImage(let element):
            (view as? PaxNativeImageView)?.apply(element: element)
        case .youtubeVideo(let element):
            (view as? PaxNativeYoutubeView)?.apply(element: element)
        case .eventBlocker:
            break
        }
    }
}

#if os(iOS) || os(tvOS) || os(watchOS)
private final class PaxNativeEventBlockerView: UIView {
    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .clear
        isOpaque = false
        isUserInteractionEnabled = true
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}

private final class PaxNativeTextLeafView: UIView, UITextViewDelegate {
    private let staticTextLayer = CATextLayer()
    private let selectableView = UITextView()
    private var usingSelectableView = false
    private var suppressChange = false
    private var editableNodeId: PaxNodeId = 0

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .clear
        isOpaque = false
        clipsToBounds = false
        layer.masksToBounds = false

        staticTextLayer.frame = bounds
        staticTextLayer.isWrapped = true
        staticTextLayer.truncationMode = .none
        staticTextLayer.masksToBounds = false
        staticTextLayer.contentsScale = UIScreen.main.scale
        layer.addSublayer(staticTextLayer)

        selectableView.frame = bounds
        selectableView.autoresizingMask = NativeRenderingLayer.fillAutoresizingMask()
        selectableView.backgroundColor = .clear
        selectableView.isScrollEnabled = false
        selectableView.clipsToBounds = false
        selectableView.layer.masksToBounds = false
        selectableView.textContainerInset = .zero
        selectableView.textContainer.lineFragmentPadding = 0
        selectableView.delegate = self

    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: TextElement, size: CGSize) {
        clipsToBounds = element.clip
        layer.masksToBounds = element.clip
        selectableView.clipsToBounds = element.clip
        selectableView.layer.masksToBounds = element.clip
        staticTextLayer.masksToBounds = element.clip
        // When clip=false, prefer the static text layer even for selectable text so descenders and
        // other overflow can render outside the frame. Editing still requires the native text view.
        let useSelectableView = element.editable || (element.selectable && element.clip)
        if useSelectableView != usingSelectableView {
            if useSelectableView {
                staticTextLayer.isHidden = true
                addSubview(selectableView)
            } else {
                selectableView.removeFromSuperview()
                staticTextLayer.isHidden = false
            }
            usingSelectableView = useSelectableView
        }

        let rect = CGRect(origin: .zero, size: size)
        staticTextLayer.contentsScale = UIScreen.main.scale
        selectableView.frame = rect
        selectableView.bounds = rect

        let attr = NSAttributedString(nativeAttributedString(for: element))
        let measurementConstraint = CGSize(
            width: element.size_x >= 0 ? size.width : CGFloat.greatestFiniteMagnitude,
            height: (element.size_y >= 0 && element.clip) ? size.height : CGFloat.greatestFiniteMagnitude
        )
        if useSelectableView {
            editableNodeId = element.id
            suppressChange = true
            selectableView.attributedText = attr
            suppressChange = false
            selectableView.font = element.textStyle.font.getUIFont(size: element.textStyle.font_size)
            selectableView.textColor = platformColor(element.textStyle.fill)
            selectableView.textAlignment = platformTextAlignment(element.textStyle.alignmentMultiline)
            selectableView.isEditable = element.editable
            selectableView.isSelectable = element.selectable || element.editable
            selectableView.textContainer.size = CGSize(
                width: size.width,
                height: element.clip ? size.height : CGFloat.greatestFiniteMagnitude
            )
            let measured = selectableView.sizeThatFits(measurementConstraint)
            let alignedFrame = alignedTextLayerFrame(
                containerSize: size,
                measuredTextSize: measured,
                alignment: element.textStyle.alignment,
                clip: element.clip
            )
            selectableView.frame = alignedFrame
            selectableView.bounds = CGRect(origin: .zero, size: alignedFrame.size)
            reportMeasuredTextSizeIfNeeded(measured, for: element)
        } else {
            let mutable = NSMutableAttributedString(attributedString: attr)
            let fullRange = NSRange(location: 0, length: mutable.length)
            let paragraphStyle = NSMutableParagraphStyle()
            paragraphStyle.alignment = platformHorizontalTextAlignment(element.textStyle.alignment)
            paragraphStyle.lineBreakMode = .byWordWrapping
            mutable.addAttributes(
                [
                    .font: element.textStyle.font.getUIFont(size: element.textStyle.font_size),
                    .foregroundColor: platformColor(element.textStyle.fill),
                    .paragraphStyle: paragraphStyle
                ],
                range: fullRange
            )
            staticTextLayer.alignmentMode = platformLayerTextAlignment(element.textStyle.alignment)
            staticTextLayer.string = mutable
            let measured = mutable.boundingRect(
                with: measurementConstraint,
                options: [.usesLineFragmentOrigin, .usesFontLeading],
                context: nil
            ).integral.size
            staticTextLayer.frame = alignedTextLayerFrame(
                containerSize: size,
                measuredTextSize: measured,
                alignment: element.textStyle.alignment,
                clip: element.clip
            )
            reportMeasuredTextSizeIfNeeded(measured, for: element)
        }
    }

    func textViewDidChange(_ textView: UITextView) {
        guard !suppressChange else {
            return
        }
        dispatchTextInput(id: editableNodeId, text: textView.text)
    }
}

private final class PaxNativeButtonView: UIButton {
    private var nodeId: PaxNodeId = 0

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .clear
        layer.masksToBounds = true
        addTarget(self, action: #selector(handleTap), for: .touchUpInside)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: ButtonElement) {
        nodeId = element.id
        setTitle(element.content.isEmpty ? " " : element.content, for: .normal)
        titleLabel?.font = element.style.font.getUIFont(size: element.style.font_size)
        setTitleColor(platformColor(element.style.fill), for: .normal)
        backgroundColor = platformColor(element.color)
        layer.cornerRadius = CGFloat(element.borderRadius)
        layer.borderWidth = CGFloat(element.outlineStrokeWidth)
        layer.borderColor = platformColor(element.outlineStrokeColor).cgColor
    }

    @objc private func handleTap() {
        dispatchFormButtonClick(id: nodeId)
    }
}

private final class PaxNativeCheckboxView: UIButton {
    private var nodeId: PaxNodeId = 0

    override init(frame: CGRect) {
        super.init(frame: frame)
        addTarget(self, action: #selector(handleTap), for: .touchUpInside)
        titleLabel?.textAlignment = .center
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: CheckboxElement, size: CGSize) {
        nodeId = element.id
        backgroundColor = platformColor(element.checked ? element.backgroundChecked : element.background)
        layer.cornerRadius = CGFloat(element.borderRadius)
        layer.borderWidth = CGFloat(element.outlineWidth)
        layer.borderColor = platformColor(element.outlineColor).cgColor
        let checkSize = max(10, min(size.width, size.height) * 0.55)
        titleLabel?.font = UIFont.systemFont(ofSize: checkSize, weight: .bold)
        setTitle(element.checked ? "✓" : "", for: .normal)
        setTitleColor(.white, for: .normal)
    }

    @objc private func handleTap() {
        dispatchFormCheckboxToggle(id: nodeId, state: title(for: .normal)?.isEmpty ?? true)
    }
}

private final class PaxNativeSliderView: UISlider {
    private var nodeId: PaxNodeId = 0

    override init(frame: CGRect) {
        super.init(frame: frame)
        addTarget(self, action: #selector(handleChange), for: .valueChanged)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: SliderElement) {
        nodeId = element.id
        minimumValue = Float(element.min)
        maximumValue = Float(element.max > element.min ? element.max : element.min + 1)
        value = Float(element.value)
        minimumTrackTintColor = platformColor(element.accent)
        maximumTrackTintColor = platformColor(element.background)
    }

    @objc private func handleChange() {
        dispatchFormSliderChange(id: nodeId, value: Double(value))
    }
}

private final class PaxNativeDropdownView: UIButton {
    private var nodeId: PaxNodeId = 0

    override init(frame: CGRect) {
        super.init(frame: frame)
        showsMenuAsPrimaryAction = true
        layer.masksToBounds = true
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: DropdownElement) {
        nodeId = element.id
        backgroundColor = platformColor(element.background)
        layer.cornerRadius = CGFloat(element.borderRadius)
        layer.borderWidth = CGFloat(element.strokeWidth)
        layer.borderColor = platformColor(element.strokeColor).cgColor
        titleLabel?.font = element.style.font.getUIFont(size: element.style.font_size)
        setTitleColor(platformColor(element.style.fill), for: .normal)
        let selectedIndex = min(max(Int(element.selectedId), 0), max(element.options.count - 1, 0))
        let title = element.options.indices.contains(selectedIndex) ? element.options[selectedIndex] : ""
        setTitle(title, for: .normal)
        menu = UIMenu(children: element.options.enumerated().map { index, option in
            UIAction(title: option) { _ in
                dispatchFormDropdownChange(id: element.id, selectedId: UInt32(index))
            }
        })
    }
}

private final class PaxNativeRadioListView: UIStackView {
    private var nodeId: PaxNodeId = 0

    override init(frame: CGRect) {
        super.init(frame: frame)
        axis = .vertical
        spacing = 6
        alignment = .fill
        distribution = .fillEqually
    }

    required init(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: RadioListElement) {
        nodeId = element.id
        if arrangedSubviews.count != element.options.count {
            arrangedSubviews.forEach { view in
                removeArrangedSubview(view)
                view.removeFromSuperview()
            }
            for index in element.options.indices {
                let button = UIButton(type: .system)
                button.tag = index
                button.contentHorizontalAlignment = .left
                button.addTarget(self, action: #selector(selectOption(_:)), for: .touchUpInside)
                addArrangedSubview(button)
            }
        }

        for (index, view) in arrangedSubviews.enumerated() {
            guard let button = view as? UIButton else { continue }
            let prefix = Int(element.selectedId) == index ? "◉ " : "○ "
            button.setTitle(prefix + element.options[index], for: .normal)
            button.titleLabel?.font = element.style.font.getUIFont(size: element.style.font_size)
            button.setTitleColor(platformColor(element.style.fill), for: .normal)
        }
    }

    @objc private func selectOption(_ sender: UIButton) {
        dispatchFormRadioListChange(id: nodeId, selectedId: UInt32(sender.tag))
    }
}

private final class PaxNativeTextboxFieldView: UITextField, UITextFieldDelegate {
    private var nodeId: PaxNodeId = 0
    private var isProgrammaticChange = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        borderStyle = .none
        autocorrectionType = .no
        autocapitalizationType = .none
        contentVerticalAlignment = .center
        delegate = self
        addTarget(self, action: #selector(textDidChange), for: .editingChanged)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: TextboxElement) {
        nodeId = element.id
        isProgrammaticChange = true
        if text != element.text {
            text = element.text
        }
        isProgrammaticChange = false
        placeholder = element.placeholder
        font = element.style.font.getUIFont(size: element.style.font_size)
        textColor = platformColor(element.style.fill)
        textAlignment = platformTextAlignment(element.style.alignmentMultiline)
        contentVerticalAlignment = .center
        backgroundColor = platformColor(element.background)
        layer.cornerRadius = CGFloat(element.borderRadius)
        layer.borderWidth = CGFloat(max(element.outlineWidth, element.strokeWidth))
        layer.borderColor = platformColor(element.outlineWidth > 0 ? element.outlineColor : element.strokeColor).cgColor
        if element.focusOnMount && !isFirstResponder {
            DispatchQueue.main.async { [weak self] in self?.becomeFirstResponder() }
        }
    }

    @objc private func textDidChange() {
        guard !isProgrammaticChange else { return }
        dispatchFormTextboxInput(id: nodeId, text: text ?? "")
    }

    func textFieldDidEndEditing(_ textField: UITextField) {
        dispatchFormTextboxChange(id: nodeId, text: textField.text ?? "")
    }
}

private final class PaxNativeTextboxAreaView: UITextView, UITextViewDelegate {
    private var nodeId: PaxNodeId = 0
    private var isProgrammaticChange = false

    override init(frame: CGRect, textContainer: NSTextContainer?) {
        super.init(frame: frame, textContainer: textContainer)
        backgroundColor = .clear
        textContainerInset = UIEdgeInsets(top: 6, left: 4, bottom: 6, right: 4)
        delegate = self
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: TextboxElement) {
        nodeId = element.id
        isProgrammaticChange = true
        if text != element.text {
            text = element.text
        }
        isProgrammaticChange = false
        font = element.style.font.getUIFont(size: element.style.font_size)
        textColor = platformColor(element.style.fill)
        textAlignment = platformTextAlignment(element.style.alignmentMultiline)
        backgroundColor = platformColor(element.background)
        layer.cornerRadius = CGFloat(element.borderRadius)
        layer.borderWidth = CGFloat(max(element.outlineWidth, element.strokeWidth))
        layer.borderColor = platformColor(element.outlineWidth > 0 ? element.outlineColor : element.strokeColor).cgColor
        if element.focusOnMount && !isFirstResponder {
            DispatchQueue.main.async { [weak self] in self?.becomeFirstResponder() }
        }
    }

    func textViewDidChange(_ textView: UITextView) {
        guard !isProgrammaticChange else { return }
        dispatchFormTextboxInput(id: nodeId, text: textView.text)
    }

    func textViewDidEndEditing(_ textView: UITextView) {
        dispatchFormTextboxChange(id: nodeId, text: textView.text)
    }
}

private final class PaxNativeImageView: UIImageView {
    private var currentURL: String = ""

    override init(frame: CGRect) {
        super.init(frame: frame)
        clipsToBounds = true
        backgroundColor = .clear
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: NativeImageElement) {
        if currentURL != element.url {
            currentURL = element.url
            image = loadPlatformImage(path: element.url)
        }
        switch element.fit {
        case "cover":
            contentMode = .scaleAspectFill
        case "fill":
            contentMode = .scaleToFill
        default:
            contentMode = .scaleAspectFit
        }
    }
}

private final class PaxNativeYoutubeView: UIButton {
    private var currentURL: URL?

    override init(frame: CGRect) {
        super.init(frame: frame)
        setTitle("Open Video", for: .normal)
        setTitleColor(.white, for: .normal)
        backgroundColor = UIColor.black.withAlphaComponent(0.85)
        layer.cornerRadius = 12
        addTarget(self, action: #selector(openVideo), for: .touchUpInside)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: YoutubeVideoElement) {
        currentURL = URL(string: element.url)
    }

    @objc private func openVideo() {
        guard let currentURL else { return }
        UIApplication.shared.open(currentURL)
    }
}

private func loadPlatformImage(path: String) -> UIImage? {
    UIImage(contentsOfFile: path)
}
#elseif os(macOS)
private final class PaxNativeEventBlockerView: NSView {
    override var isFlipped: Bool { true }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer?.backgroundColor = NSColor.clear.cgColor
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}

private final class PaxNativeTextLeafView: NSView, NSTextViewDelegate {
    override var isFlipped: Bool { true }

    private let staticTextLayer = CATextLayer()
    private let scrollView = NSScrollView()
    private let textView = NSTextView()
    private var usingTextView = false
    private var suppressChange = false
    private var editableNodeId: PaxNodeId = 0

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer?.backgroundColor = NSColor.clear.cgColor
        layer?.masksToBounds = false
        staticTextLayer.frame = bounds
        staticTextLayer.isWrapped = true
        staticTextLayer.truncationMode = .none
        staticTextLayer.masksToBounds = false
        staticTextLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 1.0
        layer?.addSublayer(staticTextLayer)

        scrollView.frame = bounds
        scrollView.autoresizingMask = NativeRenderingLayer.fillAutoresizingMask()
        scrollView.hasVerticalScroller = false
        scrollView.hasHorizontalScroller = false
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        scrollView.documentView = textView
        scrollView.wantsLayer = true
        scrollView.layer?.masksToBounds = false

        textView.drawsBackground = false
        textView.isEditable = false
        textView.isSelectable = true
        textView.delegate = self
        textView.textContainerInset = .zero
        textView.textContainer?.lineFragmentPadding = 0
        textView.isVerticallyResizable = false
        textView.isHorizontallyResizable = false
        textView.textContainer?.widthTracksTextView = true
        textView.wantsLayer = true
        textView.layer?.masksToBounds = false
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: TextElement, size: CGSize) {
        layer?.masksToBounds = element.clip
        scrollView.layer?.masksToBounds = element.clip
        textView.layer?.masksToBounds = element.clip
        staticTextLayer.masksToBounds = element.clip
        // When clip=false, prefer the static text layer even for selectable text so descenders and
        // other overflow can render outside the frame. Editing still requires the native text view.
        let useTextView = element.editable || (element.selectable && element.clip)
        if useTextView != usingTextView {
            if useTextView {
                staticTextLayer.isHidden = true
                addSubview(scrollView)
            } else {
                scrollView.removeFromSuperview()
                staticTextLayer.isHidden = false
            }
            usingTextView = useTextView
        }

        let attr = NSAttributedString(nativeAttributedString(for: element))
        let rect = CGRect(origin: .zero, size: size)
        let measurementConstraint = CGSize(
            width: element.size_x >= 0 ? size.width : CGFloat.greatestFiniteMagnitude,
            height: (element.size_y >= 0 && element.clip) ? size.height : CGFloat.greatestFiniteMagnitude
        )
        if useTextView {
            editableNodeId = element.id
            suppressChange = true
            textView.textStorage?.setAttributedString(attr)
            suppressChange = false
            textView.font = element.textStyle.font.getNSFont(size: element.textStyle.font_size)
            textView.textColor = platformColor(element.textStyle.fill)
            textView.alignment = platformTextAlignment(element.textStyle.alignmentMultiline)
            textView.isEditable = element.editable
            textView.isSelectable = element.selectable || element.editable
            textView.textContainer?.containerSize = CGSize(
                width: size.width,
                height: element.clip ? size.height : CGFloat.greatestFiniteMagnitude
            )
            let measured = textView.fittingSize
            let alignedFrame = alignedTextLayerFrame(
                containerSize: size,
                measuredTextSize: measured,
                alignment: element.textStyle.alignment,
                clip: element.clip
            )
            scrollView.frame = alignedFrame
            scrollView.bounds = CGRect(origin: .zero, size: alignedFrame.size)
            textView.frame = CGRect(origin: .zero, size: alignedFrame.size)
            textView.bounds = CGRect(origin: .zero, size: alignedFrame.size)
            reportMeasuredTextSizeIfNeeded(measured, for: element)
        } else {
            let mutable = NSMutableAttributedString(attributedString: attr)
            let fullRange = NSRange(location: 0, length: mutable.length)
            let paragraphStyle = NSMutableParagraphStyle()
            paragraphStyle.alignment = platformHorizontalTextAlignment(element.textStyle.alignment)
            paragraphStyle.lineBreakMode = .byWordWrapping
            mutable.addAttributes(
                [
                    .font: element.textStyle.font.getNSFont(size: element.textStyle.font_size),
                    .foregroundColor: platformColor(element.textStyle.fill),
                    .paragraphStyle: paragraphStyle
                ],
                range: fullRange
            )
            staticTextLayer.contentsScale = window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 1.0
            staticTextLayer.alignmentMode = platformLayerTextAlignment(element.textStyle.alignment)
            staticTextLayer.string = mutable
            let measured = mutable.boundingRect(
                with: measurementConstraint,
                options: [.usesLineFragmentOrigin, .usesFontLeading],
                context: nil
            ).integral.size
            staticTextLayer.frame = alignedTextLayerFrame(
                containerSize: size,
                measuredTextSize: measured,
                alignment: element.textStyle.alignment,
                clip: element.clip
            )
            reportMeasuredTextSizeIfNeeded(measured, for: element)
        }
    }

    func textDidChange(_ notification: Notification) {
        guard !suppressChange, let textView = notification.object as? NSTextView else {
            return
        }
        dispatchTextInput(id: editableNodeId, text: textView.string)
    }
}

private final class PaxNativeButtonView: NSButton {
    private var nodeId: PaxNodeId = 0

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        isBordered = false
        bezelStyle = .regularSquare
        wantsLayer = true
        target = self
        action = #selector(handleTap)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: ButtonElement) {
        nodeId = element.id
        title = element.content.isEmpty ? " " : element.content
        font = element.style.font.getNSFont(size: element.style.font_size)
        contentTintColor = platformColor(element.style.fill)
        layer?.backgroundColor = platformColor(element.color).cgColor
        layer?.cornerRadius = CGFloat(element.borderRadius)
        layer?.borderWidth = CGFloat(element.outlineStrokeWidth)
        layer?.borderColor = platformColor(element.outlineStrokeColor).cgColor
    }

    @objc private func handleTap() {
        dispatchFormButtonClick(id: nodeId)
    }
}

private final class PaxNativeCheckboxView: NSButton {
    private var nodeId: PaxNodeId = 0

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        isBordered = false
        wantsLayer = true
        target = self
        action = #selector(handleTap)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: CheckboxElement, size: CGSize) {
        nodeId = element.id
        title = element.checked ? "✓" : ""
        font = NSFont.systemFont(ofSize: max(10, min(size.width, size.height) * 0.55), weight: .bold)
        contentTintColor = .white
        layer?.backgroundColor = platformColor(element.checked ? element.backgroundChecked : element.background).cgColor
        layer?.cornerRadius = CGFloat(element.borderRadius)
        layer?.borderWidth = CGFloat(element.outlineWidth)
        layer?.borderColor = platformColor(element.outlineColor).cgColor
    }

    @objc private func handleTap() {
        dispatchFormCheckboxToggle(id: nodeId, state: title.isEmpty)
    }
}

private final class PaxNativeSliderView: NSSlider {
    private var nodeId: PaxNodeId = 0

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        target = self
        action = #selector(handleChange)
        isContinuous = true
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: SliderElement) {
        nodeId = element.id
        minValue = element.min
        maxValue = element.max > element.min ? element.max : element.min + 1
        doubleValue = element.value
    }

    @objc private func handleChange() {
        dispatchFormSliderChange(id: nodeId, value: doubleValue)
    }
}

private final class PaxNativeDropdownView: NSPopUpButton {
    private var nodeId: PaxNodeId = 0

    override init(frame frameRect: NSRect, pullsDown flag: Bool) {
        super.init(frame: frameRect, pullsDown: flag)
        wantsLayer = true
        target = self
        action = #selector(handleChange)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: DropdownElement) {
        nodeId = element.id
        if itemTitles != element.options {
            removeAllItems()
            addItems(withTitles: element.options)
        }
        let resolvedFont = element.style.font.getNSFont(size: element.style.font_size)
        let resolvedTextColor = platformColor(element.style.fill)
        let titleAttributes: [NSAttributedString.Key: Any] = [
            .font: resolvedFont,
            .foregroundColor: resolvedTextColor,
        ]
        for (index, option) in element.options.enumerated() {
            guard index < itemArray.count else {
                continue
            }
            itemArray[index].attributedTitle = NSAttributedString(
                string: option,
                attributes: titleAttributes
            )
        }
        selectItem(at: min(max(Int(element.selectedId), 0), max(numberOfItems - 1, 0)))
        font = resolvedFont
        attributedTitle = selectedItem?.attributedTitle ?? NSAttributedString(
            string: titleOfSelectedItem ?? "",
            attributes: titleAttributes
        )
        contentTintColor = resolvedTextColor
        appearance = NSAppearance(named: .aqua)
        layer?.backgroundColor = platformColor(element.background).cgColor
        layer?.cornerRadius = CGFloat(element.borderRadius)
        layer?.borderWidth = CGFloat(element.strokeWidth)
        layer?.borderColor = platformColor(element.strokeColor).cgColor
    }

    @objc private func handleChange() {
        dispatchFormDropdownChange(id: nodeId, selectedId: UInt32(max(indexOfSelectedItem, 0)))
    }
}

private final class PaxNativeRadioListView: NSStackView {
    private var nodeId: PaxNodeId = 0

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        orientation = .vertical
        spacing = 6
        alignment = .leading
        distribution = .fillEqually
    }

    required init(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: RadioListElement) {
        nodeId = element.id
        if arrangedSubviews.count != element.options.count {
            arrangedSubviews.forEach { view in
                removeArrangedSubview(view)
                view.removeFromSuperview()
            }
            for index in element.options.indices {
                let button = NSButton(radioButtonWithTitle: "", target: self, action: #selector(selectOption(_:)))
                button.tag = index
                addArrangedSubview(button)
            }
        }

        for (index, view) in arrangedSubviews.enumerated() {
            guard let button = view as? NSButton else { continue }
            button.title = element.options[index]
            button.font = element.style.font.getNSFont(size: element.style.font_size)
            button.contentTintColor = platformColor(element.style.fill)
            button.state = Int(element.selectedId) == index ? .on : .off
        }
    }

    @objc private func selectOption(_ sender: NSButton) {
        dispatchFormRadioListChange(id: nodeId, selectedId: UInt32(sender.tag))
    }
}

private final class VerticallyCenteredTextFieldCell: NSTextFieldCell {
    private func verticallyCenteredRect(for rect: NSRect) -> NSRect {
        var drawingRect = super.drawingRect(forBounds: rect)
        let textHeight = min(drawingRect.height, cellSize(forBounds: rect).height)
        drawingRect.origin.y += max(0, (drawingRect.height - textHeight) * 0.5)
        drawingRect.size.height = textHeight
        return drawingRect
    }

    override func titleRect(forBounds rect: NSRect) -> NSRect {
        verticallyCenteredRect(for: rect)
    }

    override func drawingRect(forBounds rect: NSRect) -> NSRect {
        verticallyCenteredRect(for: rect)
    }

    override func edit(
        withFrame rect: NSRect,
        in controlView: NSView,
        editor textObj: NSText,
        delegate: Any?,
        event: NSEvent?
    ) {
        super.edit(
            withFrame: verticallyCenteredRect(for: rect),
            in: controlView,
            editor: textObj,
            delegate: delegate,
            event: event
        )
    }

    override func select(
        withFrame rect: NSRect,
        in controlView: NSView,
        editor textObj: NSText,
        delegate: Any?,
        start selStart: Int,
        length selLength: Int
    ) {
        super.select(
            withFrame: verticallyCenteredRect(for: rect),
            in: controlView,
            editor: textObj,
            delegate: delegate,
            start: selStart,
            length: selLength
        )
    }
}

private final class PaxNativeTextboxFieldView: NSTextField, NSTextFieldDelegate {
    private var nodeId: PaxNodeId = 0
    private var isProgrammaticChange = false

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        cell = VerticallyCenteredTextFieldCell(textCell: "")
        delegate = self
        isBordered = false
        isBezeled = false
        isEditable = true
        isSelectable = true
        isEnabled = true
        drawsBackground = false
        focusRingType = .none
        wantsLayer = true
        layer?.masksToBounds = true
        (cell as? NSTextFieldCell)?.usesSingleLineMode = true
        (cell as? NSTextFieldCell)?.isScrollable = true
        (cell as? NSTextFieldCell)?.wraps = false
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: TextboxElement) {
        nodeId = element.id
        isProgrammaticChange = true
        if stringValue != element.text {
            stringValue = element.text
        }
        isProgrammaticChange = false
        placeholderString = element.placeholder
        font = element.style.font.getNSFont(size: element.style.font_size)
        textColor = platformColor(element.style.fill)
        alignment = platformTextAlignment(element.style.alignmentMultiline)
        drawsBackground = false
        layer?.backgroundColor = platformColor(element.background).cgColor
        layer?.cornerRadius = CGFloat(element.borderRadius)
        layer?.masksToBounds = true
        layer?.borderWidth = CGFloat(max(element.outlineWidth, element.strokeWidth))
        layer?.borderColor = platformColor(element.outlineWidth > 0 ? element.outlineColor : element.strokeColor).cgColor
        if element.focusOnMount, window?.firstResponder !== currentEditor() {
            DispatchQueue.main.async { [weak self] in self?.window?.makeFirstResponder(self) }
        }
    }

    override func textDidChange(_ notification: Notification) {
        guard !isProgrammaticChange else { return }
        dispatchFormTextboxInput(id: nodeId, text: stringValue)
    }

    func controlTextDidEndEditing(_ obj: Notification) {
        dispatchFormTextboxChange(id: nodeId, text: stringValue)
    }
}

private final class PaxNativeTextboxAreaView: NSScrollView, NSTextViewDelegate {
    private let textView = NSTextView()
    private var nodeId: PaxNodeId = 0
    private var isProgrammaticChange = false

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        borderType = .noBorder
        drawsBackground = false
        hasVerticalScroller = false
        hasHorizontalScroller = false
        textView.drawsBackground = false
        textView.delegate = self
        textView.textContainerInset = NSSize(width: 4, height: 6)
        textView.textContainer?.lineFragmentPadding = 0
        documentView = textView
        wantsLayer = true
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: TextboxElement) {
        nodeId = element.id
        isProgrammaticChange = true
        if textView.string != element.text {
            textView.string = element.text
        }
        isProgrammaticChange = false
        textView.font = element.style.font.getNSFont(size: element.style.font_size)
        textView.textColor = platformColor(element.style.fill)
        textView.alignment = platformTextAlignment(element.style.alignmentMultiline)
        layer?.backgroundColor = platformColor(element.background).cgColor
        layer?.cornerRadius = CGFloat(element.borderRadius)
        layer?.borderWidth = CGFloat(max(element.outlineWidth, element.strokeWidth))
        layer?.borderColor = platformColor(element.outlineWidth > 0 ? element.outlineColor : element.strokeColor).cgColor
        if element.focusOnMount, window?.firstResponder !== textView {
            DispatchQueue.main.async { [weak self] in self?.window?.makeFirstResponder(self?.textView) }
        }
    }

    func textDidChange(_ notification: Notification) {
        guard !isProgrammaticChange else { return }
        dispatchFormTextboxInput(id: nodeId, text: textView.string)
    }

    func textDidEndEditing(_ notification: Notification) {
        dispatchFormTextboxChange(id: nodeId, text: textView.string)
    }
}

private final class PaxNativeImageView: NSImageView {
    private var currentURL: String = ""

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        imageScaling = .scaleProportionallyUpOrDown
        imageAlignment = .alignCenter
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: NativeImageElement) {
        if currentURL != element.url {
            currentURL = element.url
            image = loadPlatformImage(path: element.url)
        }
        switch element.fit {
        case "cover":
            imageScaling = .scaleAxesIndependently
        case "fill":
            imageScaling = .scaleAxesIndependently
        default:
            imageScaling = .scaleProportionallyUpOrDown
        }
    }
}

private final class PaxNativeYoutubeView: NSButton {
    private var currentURL: URL?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        title = "Open Video"
        isBordered = false
        wantsLayer = true
        layer?.backgroundColor = NSColor.black.withAlphaComponent(0.85).cgColor
        layer?.cornerRadius = 12
        target = self
        action = #selector(openVideo)
        contentTintColor = .white
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(element: YoutubeVideoElement) {
        currentURL = URL(string: element.url)
    }

    @objc private func openVideo() {
        guard let currentURL else { return }
        NSWorkspace.shared.open(currentURL)
    }
}

private func loadPlatformImage(path: String) -> NSImage? {
    NSImage(contentsOfFile: path)
}
#endif

#if os(iOS) || os(tvOS) || os(watchOS)
private struct EventBlockerPlatformView: UIViewRepresentable {
    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .clear
        view.isUserInteractionEnabled = true
        return view
    }

    func updateUIView(_ uiView: UIView, context: Context) {}
}

private struct PaxTextboxField: UIViewRepresentable {
    let element: TextboxElement

    func makeCoordinator() -> Coordinator {
        Coordinator(id: element.id)
    }

    func makeUIView(context: Context) -> UITextField {
        let field = UITextField()
        field.borderStyle = .none
        field.autocorrectionType = .no
        field.autocapitalizationType = .none
        field.addTarget(context.coordinator, action: #selector(Coordinator.textDidChange(_:)), for: .editingChanged)
        field.delegate = context.coordinator
        return field
    }

    func updateUIView(_ uiView: UITextField, context: Context) {
        context.coordinator.id = element.id
        context.coordinator.isProgrammaticChange = true
        if uiView.text != element.text {
            uiView.text = element.text
        }
        context.coordinator.isProgrammaticChange = false

        uiView.placeholder = element.placeholder
        uiView.font = element.style.font.getUIFont(size: element.style.font_size)
        uiView.textColor = platformColor(element.style.fill)
        uiView.textAlignment = platformTextAlignment(element.style.alignmentMultiline)
        uiView.backgroundColor = platformColor(element.background)
        uiView.layer.cornerRadius = element.borderRadius
        uiView.layer.borderWidth = CGFloat(max(element.outlineWidth, element.strokeWidth))
        uiView.layer.borderColor = platformColor(element.outlineWidth > 0 ? element.outlineColor : element.strokeColor).cgColor

        if element.focusOnMount && !uiView.isFirstResponder {
            DispatchQueue.main.async {
                uiView.becomeFirstResponder()
            }
        }
    }

    final class Coordinator: NSObject, UITextFieldDelegate {
        var id: PaxNodeId
        var isProgrammaticChange = false

        init(id: PaxNodeId) {
            self.id = id
        }

        @objc func textDidChange(_ sender: UITextField) {
            guard !isProgrammaticChange else {
                return
            }
            dispatchFormTextboxInput(id: id, text: sender.text ?? "")
        }

        func textFieldDidEndEditing(_ textField: UITextField) {
            dispatchFormTextboxChange(id: id, text: textField.text ?? "")
        }
    }
}

private struct PaxTextboxArea: UIViewRepresentable {
    let element: TextboxElement

    func makeCoordinator() -> Coordinator {
        Coordinator(id: element.id)
    }

    func makeUIView(context: Context) -> UITextView {
        let view = UITextView()
        view.backgroundColor = .clear
        view.delegate = context.coordinator
        view.textContainerInset = UIEdgeInsets(top: 6, left: 4, bottom: 6, right: 4)
        return view
    }

    func updateUIView(_ uiView: UITextView, context: Context) {
        context.coordinator.id = element.id
        context.coordinator.isProgrammaticChange = true
        if uiView.text != element.text {
            uiView.text = element.text
        }
        context.coordinator.isProgrammaticChange = false

        uiView.font = element.style.font.getUIFont(size: element.style.font_size)
        uiView.textColor = platformColor(element.style.fill)
        uiView.backgroundColor = platformColor(element.background)
        uiView.layer.cornerRadius = element.borderRadius
        uiView.layer.borderWidth = CGFloat(max(element.outlineWidth, element.strokeWidth))
        uiView.layer.borderColor = platformColor(element.outlineWidth > 0 ? element.outlineColor : element.strokeColor).cgColor
        uiView.textAlignment = platformTextAlignment(element.style.alignmentMultiline)

        if element.focusOnMount && !uiView.isFirstResponder {
            DispatchQueue.main.async {
                uiView.becomeFirstResponder()
            }
        }
    }

    final class Coordinator: NSObject, UITextViewDelegate {
        var id: PaxNodeId
        var isProgrammaticChange = false

        init(id: PaxNodeId) {
            self.id = id
        }

        func textViewDidChange(_ textView: UITextView) {
            guard !isProgrammaticChange else {
                return
            }
            dispatchFormTextboxInput(id: id, text: textView.text)
        }

        func textViewDidEndEditing(_ textView: UITextView) {
            dispatchFormTextboxChange(id: id, text: textView.text)
        }
    }
}

private struct EditableTextView: UIViewRepresentable {
    let element: TextElement

    func makeCoordinator() -> Coordinator {
        Coordinator(id: element.id)
    }

    func makeUIView(context: Context) -> UITextView {
        let view = UITextView()
        view.backgroundColor = .clear
        view.delegate = context.coordinator
        view.isScrollEnabled = false
        view.textContainerInset = .zero
        view.textContainer.lineFragmentPadding = 0
        return view
    }

    func updateUIView(_ uiView: UITextView, context: Context) {
        context.coordinator.id = element.id
        context.coordinator.isProgrammaticChange = true
        if uiView.text != element.content {
            uiView.text = element.content
        }
        context.coordinator.isProgrammaticChange = false

        uiView.font = element.textStyle.font.getUIFont(size: element.textStyle.font_size)
        uiView.textColor = platformColor(element.textStyle.fill)
        uiView.textAlignment = platformTextAlignment(element.textStyle.alignmentMultiline)
        uiView.isEditable = element.editable
        uiView.isSelectable = element.selectable || element.editable
    }

    final class Coordinator: NSObject, UITextViewDelegate {
        var id: PaxNodeId
        var isProgrammaticChange = false

        init(id: PaxNodeId) {
            self.id = id
        }

        func textViewDidChange(_ textView: UITextView) {
            guard !isProgrammaticChange else {
                return
            }
            dispatchTextInput(id: id, text: textView.text)
        }
    }
}
#else
private struct EventBlockerPlatformView: View {
    var body: some View {
        Color.clear
            .contentShape(Rectangle())
    }
}

private struct PaxTextboxField: View {
    let element: TextboxElement

    var body: some View {
        TextField(
            "",
            text: Binding(
                get: { element.text },
                set: {
                    dispatchFormTextboxInput(id: element.id, text: $0)
                    dispatchFormTextboxChange(id: element.id, text: $0)
                }
            ),
            prompt: element.placeholder.isEmpty ? nil : Text(element.placeholder)
        )
    }
}

private struct PaxTextboxArea: View {
    let element: TextboxElement

    var body: some View {
        TextEditor(
            text: Binding(
                get: { element.text },
                set: {
                    dispatchFormTextboxInput(id: element.id, text: $0)
                    dispatchFormTextboxChange(id: element.id, text: $0)
                }
            )
        )
    }
}

private struct EditableTextView: View {
    let element: TextElement

    var body: some View {
        TextEditor(
            text: Binding(
                get: { element.content },
                set: { dispatchTextInput(id: element.id, text: $0) }
            )
        )
    }
}
#endif

public class TextElements: ObservableObject {
    public static let singleton = TextElements()
    @Published public var elements: [PaxNodeId: TextElement] = [:]

    public func add(element: TextElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class FrameElements: ObservableObject {
    public static let singleton = FrameElements()
    @Published public var elements: [PaxNodeId: FrameElement] = [:]

    public func add(element: FrameElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }

    public func get(id: PaxNodeId) -> FrameElement? {
        elements[id]
    }
}

public class ButtonElements: ObservableObject {
    public static let singleton = ButtonElements()
    @Published public var elements: [PaxNodeId: ButtonElement] = [:]

    public func add(element: ButtonElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class CheckboxElements: ObservableObject {
    public static let singleton = CheckboxElements()
    @Published public var elements: [PaxNodeId: CheckboxElement] = [:]

    public func add(element: CheckboxElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class NativeImageElements: ObservableObject {
    public static let singleton = NativeImageElements()
    @Published public var elements: [PaxNodeId: NativeImageElement] = [:]

    public func add(element: NativeImageElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class YoutubeVideoElements: ObservableObject {
    public static let singleton = YoutubeVideoElements()
    @Published public var elements: [PaxNodeId: YoutubeVideoElement] = [:]

    public func add(element: YoutubeVideoElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class DropdownElements: ObservableObject {
    public static let singleton = DropdownElements()
    @Published public var elements: [PaxNodeId: DropdownElement] = [:]

    public func add(element: DropdownElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class RadioListElements: ObservableObject {
    public static let singleton = RadioListElements()
    @Published public var elements: [PaxNodeId: RadioListElement] = [:]

    public func add(element: RadioListElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class SliderElements: ObservableObject {
    public static let singleton = SliderElements()
    @Published public var elements: [PaxNodeId: SliderElement] = [:]

    public func add(element: SliderElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class TextboxElements: ObservableObject {
    public static let singleton = TextboxElements()
    @Published public var elements: [PaxNodeId: TextboxElement] = [:]

    public func add(element: TextboxElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class EventBlockerElements: ObservableObject {
    public static let singleton = EventBlockerElements()
    @Published public var elements: [PaxNodeId: EventBlockerElement] = [:]

    public func add(element: EventBlockerElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class ScrollerElements: ObservableObject {
    public static let singleton = ScrollerElements()
    @Published public var elements: [PaxNodeId: ScrollerElement] = [:]

    public func add(element: ScrollerElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }

    public func get(id: PaxNodeId) -> ScrollerElement? {
        elements[id]
    }
}
