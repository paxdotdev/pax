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

private func textAlignment(from alignment: TextAlignment) -> TextAlignment {
    alignment
}

#if os(iOS) || os(tvOS) || os(watchOS)
private func platformColor(_ color: Color) -> UIColor {
    UIColor(color)
}

private struct RasterizedMaskHolePayload {
    let cgPath: CGPath
    let clipCGPaths: [CGPath]
    let opacity: CGFloat
}

private struct RasterizedNativeMaskPayload {
    let signature: UInt64
    let size: CGSize
    let frameClipCGPaths: [CGPath]
    let holes: [RasterizedMaskHolePayload]
}

private struct PendingMaskRender {
    let generation: UInt64
    let scale: CGFloat
    let payload: RasterizedNativeMaskPayload
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

private final class LayerMaskedHostingController: UIViewController {
    private static let maskRasterQueue = DispatchQueue(
        label: "dev.pax.swift.native-mask-raster",
        qos: .userInitiated
    )

    private let hostingController = UIHostingController(rootView: AnyView(EmptyView()))
    private let maskLayer = CALayer()
    private var appliedMaskSignature: UInt64?
    private var appliedMaskSize: CGSize = .zero
    private var requestedMaskSignature: UInt64?
    private var requestedMaskSize: CGSize = .zero
    private var nextMaskGeneration: UInt64 = 0
    private var inFlightMaskRender: PendingMaskRender?
    private var queuedMaskRender: PendingMaskRender?

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .clear
        view.isOpaque = false
        hostingController.view.backgroundColor = .clear
        hostingController.view.isOpaque = false
        hostingController.view.translatesAutoresizingMaskIntoConstraints = false
        addChild(hostingController)
        view.addSubview(hostingController.view)
        NSLayoutConstraint.activate([
            hostingController.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            hostingController.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            hostingController.view.topAnchor.constraint(equalTo: view.topAnchor),
            hostingController.view.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        ])
        hostingController.didMove(toParent: self)
        view.layer.mask = maskLayer
    }

    private static func currentMaskScale() -> CGFloat {
        #if targetEnvironment(simulator)
        return 1.0
        #else
        return UIScreen.main.scale
        #endif
    }

    private static func rasterPayload(from mask: ResolvedNativeMask) -> RasterizedNativeMaskPayload {
        RasterizedNativeMaskPayload(
            signature: mask.signature,
            size: mask.size,
            frameClipCGPaths: mask.frameClipCGPaths,
            holes: mask.holes.map { hole in
                RasterizedMaskHolePayload(
                    cgPath: hole.cgPath,
                    clipCGPaths: hole.clipCGPaths,
                    opacity: CGFloat(hole.opacity)
                )
            }
        )
    }

    private static func rasterizedMaskImage(
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

        context.scaleBy(x: scale, y: scale)

        let bounds = CGRect(origin: .zero, size: payload.size)
        context.saveGState()
        if !payload.frameClipCGPaths.isEmpty {
            for clip in payload.frameClipCGPaths {
                context.addPath(clip)
                context.clip()
            }
        }
        context.setFillColor(gray: 1.0, alpha: 1.0)
        context.fill(bounds)
        for hole in payload.holes {
            context.saveGState()
            for clip in hole.clipCGPaths {
                context.addPath(clip)
                context.clip()
            }
            context.setFillColor(gray: 0.0, alpha: hole.opacity)
            context.addPath(hole.cgPath)
            context.fillPath()
            context.restoreGState()
        }
        context.restoreGState()
        return context.makeImage()
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
            let image = Self.rasterizedMaskImage(
                payload: render.payload,
                scale: render.scale
            )
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
                    CATransaction.begin()
                    CATransaction.setDisableActions(true)
                    self.maskLayer.frame = CGRect(origin: .zero, size: render.payload.size)
                    self.maskLayer.contents = image
                    self.maskLayer.contentsScale = render.scale
                    CATransaction.commit()
                    self.appliedMaskSignature = render.payload.signature
                    self.appliedMaskSize = render.payload.size
                }
                self.startNextMaskRenderIfNeeded()
            }
        }
    }

    func update(rootView: AnyView, mask: ResolvedNativeMask) {
        hostingController.rootView = rootView
        requestedMaskSignature = mask.signature
        requestedMaskSize = mask.size
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
        enqueueMaskRender(
            payload: Self.rasterPayload(from: mask),
            scale: Self.currentMaskScale()
        )
    }
}

private struct LayerMaskedView: UIViewControllerRepresentable {
    let content: AnyView
    let mask: ResolvedNativeMask

    func makeUIViewController(context: Context) -> LayerMaskedHostingController {
        LayerMaskedHostingController()
    }

    func updateUIViewController(_ controller: LayerMaskedHostingController, context: Context) {
        controller.update(rootView: content, mask: mask)
    }
}
#elseif os(macOS)
private func platformColor(_ color: Color) -> NSColor {
    NSColor(color)
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
#endif

public struct NativeRenderingLayer: View {
    public init() {}

    @ObservedObject var nativeSceneInvalidation = NativeSceneInvalidation.singleton
    let textElements = TextElements.singleton
    let frameElements = FrameElements.singleton
    let buttonElements = ButtonElements.singleton
    let checkboxElements = CheckboxElements.singleton
    let nativeImageElements = NativeImageElements.singleton
    let youtubeVideoElements = YoutubeVideoElements.singleton
    let dropdownElements = DropdownElements.singleton
    let radioSetElements = RadioSetElements.singleton
    let sliderElements = SliderElements.singleton
    let textboxElements = TextboxElements.singleton
    let eventBlockerElements = EventBlockerElements.singleton

    private struct NativeRenderItem: Identifiable {
        let id: PaxNodeId
        let zIndex: Int
        let parentFrame: PaxNodeId?
        let transform: [Float]
        let size: CGSize
        let opacity: Double
        let content: AnyView
        let mask: ResolvedNativeMask?
    }

#if os(iOS) || os(tvOS) || os(watchOS)
    private struct WorldNativeMask {
        let signature: UInt64
        let frameClips: [Path]
        let frameClipCGPaths: [CGPath]
        let holes: [ResolvedMaskHole]
    }

    private struct NativeRenderGroup: Identifiable {
        let id: String
        let parentFrame: PaxNodeId?
        let bounds: CGRect
        let mask: ResolvedNativeMask?
        let items: [NativeRenderItem]
    }
#endif

    private func sortedTextElements() -> [TextElement] {
        Array(textElements.elements.values).sorted { lhs, rhs in
            if lhs.zIndex == rhs.zIndex {
                return lhs.id < rhs.id
            }
            return lhs.zIndex < rhs.zIndex
        }
    }

    private func sortedElements<T: NativePositionElement>(_ elements: [PaxNodeId: T]) -> [T] {
        Array(elements.values).sorted { lhs, rhs in
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
        items.append(contentsOf: sortedElements(radioSetElements.elements).map { element in
            radioSetItem(for: element)
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

    private func applyResolvedMask<V: View>(_ view: V, mask: ResolvedNativeMask?) -> AnyView {
        guard let mask else {
            return AnyView(
                view
                    .transaction { transaction in
                        transaction.animation = nil
                        transaction.disablesAnimations = true
                }
            )
        }
#if os(iOS) || os(tvOS) || os(watchOS)
        return AnyView(
            LayerMaskedView(content: AnyView(view), mask: mask)
                .frame(width: mask.size.width, height: mask.size.height)
                .transaction { transaction in
                    transaction.animation = nil
                    transaction.disablesAnimations = true
                }
        )
#else
        let frameClipped = mask.frameClips.reduce(AnyView(view)) { current, path in
            AnyView(current.clipShape(ResolvedPathShape(resolvedPath: path)))
        }
        guard !mask.holes.isEmpty else {
            return AnyView(
                frameClipped
                    .transaction { transaction in
                        transaction.animation = nil
                        transaction.disablesAnimations = true
                    }
            )
        }
        return AnyView(
            frameClipped
                .compositingGroup()
                .mask(HoleMaskView(mask: mask).id(mask.signature))
                .transaction { transaction in
                    transaction.animation = nil
                    transaction.disablesAnimations = true
                }
        )
#endif
    }

    private func resolvedFrameOpacity(startingAt parentFrame: PaxNodeId?) -> Double {
        var opacity = 1.0
        var currentFrame = parentFrame
        while let frameId = currentFrame, let frame = frameElements.elements[frameId] {
            opacity *= frame.opacity
            currentFrame = frame.parentFrame
        }
        return min(max(opacity, 0.0), 1.0)
    }

    private func resolvedOpacity(_ localOpacity: Double, parentFrame: PaxNodeId?) -> Double {
        min(max(localOpacity * resolvedFrameOpacity(startingAt: parentFrame), 0.0), 1.0)
    }

    private func renderItemContent<V: View>(_ view: V, element: NativePositionElement) -> NativeRenderItem {
        let size = resolvedSize(element)
        let bounded = AnyView(
            view
                .frame(width: resolvedDimension(element.size_x), height: resolvedDimension(element.size_y))
                .clipped()
        )
        return NativeRenderItem(
            id: element.id,
            zIndex: element.zIndex,
            parentFrame: element.parentFrame,
            transform: element.transform,
            size: size,
            opacity: resolvedOpacity(element.opacity, parentFrame: element.parentFrame),
            content: bounded,
            mask: resolvedNativeMask(for: element.id)
        )
    }

    private func renderTextContent<V: View>(_ view: V, element: TextElement, width: CGFloat, height: CGFloat) -> NativeRenderItem {
        let bounded = AnyView(
            view
                .frame(width: width > 0 ? width : nil, height: height > 0 ? height : nil)
        )
        return NativeRenderItem(
            id: element.id,
            zIndex: element.zIndex,
            parentFrame: element.parentFrame,
            transform: element.transform,
            size: CGSize(width: width, height: height),
            opacity: resolvedOpacity(element.opacity, parentFrame: element.parentFrame),
            content: bounded,
            mask: resolvedNativeMask(for: element.id)
        )
    }

    private func positionedItem(_ item: NativeRenderItem) -> AnyView {
        let localMasked = applyResolvedMask(item.content, mask: item.mask)
        let base = localMasked
            .position(x: item.size.width / 2.0, y: item.size.height / 2.0)
            .transformEffect(affineTransform(from: item.transform))
            .zIndex(Double(item.zIndex))
            .opacity(item.opacity)
            .transaction { transaction in
                transaction.animation = nil
                transaction.disablesAnimations = true
            }
        return AnyView(base)
    }

#if os(iOS) || os(tvOS) || os(watchOS)
    private func mixMaskHash(_ state: UInt64, _ value: UInt64) -> UInt64 {
        state &* 1099511628211 ^ value
    }

    private func hashCGPath(_ path: CGPath) -> UInt64 {
        var hash = UInt64(1469598103934665603)
        path.applyWithBlock { elementPointer in
            let element = elementPointer.pointee
            hash = mixMaskHash(hash, UInt64(element.type.rawValue))
            let pointCount: Int
            switch element.type {
            case .moveToPoint, .addLineToPoint:
                pointCount = 1
            case .addQuadCurveToPoint:
                pointCount = 2
            case .addCurveToPoint:
                pointCount = 3
            case .closeSubpath:
                pointCount = 0
            @unknown default:
                pointCount = 0
            }
            if pointCount > 0 {
                for index in 0..<pointCount {
                    let point = element.points[index]
                    hash = mixMaskHash(hash, Double(point.x).bitPattern)
                    hash = mixMaskHash(hash, Double(point.y).bitPattern)
                }
            }
        }
        return hash
    }

    private func transformedBounds(size: CGSize, transform: CGAffineTransform) -> CGRect {
        let rect = CGRect(origin: .zero, size: size)
        let points = [
            CGPoint(x: rect.minX, y: rect.minY).applying(transform),
            CGPoint(x: rect.maxX, y: rect.minY).applying(transform),
            CGPoint(x: rect.minX, y: rect.maxY).applying(transform),
            CGPoint(x: rect.maxX, y: rect.maxY).applying(transform),
        ]
        let minX = points.map(\.x).min() ?? 0
        let maxX = points.map(\.x).max() ?? 0
        let minY = points.map(\.y).min() ?? 0
        let maxY = points.map(\.y).max() ?? 0
        return CGRect(x: minX, y: minY, width: max(0, maxX - minX), height: max(0, maxY - minY))
    }

    private func parentFrameTransform(_ parentFrame: PaxNodeId?) -> CGAffineTransform {
        guard let parentFrame, let frame = frameElements.elements[parentFrame] else {
            return .identity
        }
        return affineTransform(from: frame.transform)
    }

    private func safeInverse(_ transform: CGAffineTransform) -> CGAffineTransform {
        let determinant = (transform.a * transform.d) - (transform.b * transform.c)
        guard abs(determinant) > .ulpOfOne else {
            return .identity
        }
        return transform.inverted()
    }

    private func itemTransformInParentFrame(_ item: NativeRenderItem) -> CGAffineTransform {
        let parentInverse = safeInverse(parentFrameTransform(item.parentFrame))
        return affineTransform(from: item.transform).concatenating(parentInverse)
    }

    private func worldMask(for item: NativeRenderItem) -> WorldNativeMask? {
        guard let mask = item.mask else {
            return nil
        }

        let transform = itemTransformInParentFrame(item)
        var signature = UInt64(1469598103934665603)
        let worldFrameClips = mask.frameClips.compactMap { path -> Path? in
            let worldPath = path.applying(transform)
            guard !worldPath.isEmpty else {
                return nil
            }
            let cgPath = worldPath.cgPath
            signature = mixMaskHash(signature, hashCGPath(cgPath))
            return worldPath
        }
        let worldFrameClipCGPaths = worldFrameClips.map(\.cgPath)
        let worldHoles = mask.holes.compactMap { hole -> ResolvedMaskHole? in
            let worldPath = hole.path.applying(transform)
            guard !worldPath.isEmpty else {
                return nil
            }
            let worldClips = hole.clips.compactMap { clip -> Path? in
                let worldClip = clip.applying(transform)
                return worldClip.isEmpty ? nil : worldClip
            }
            let worldCGPath = worldPath.cgPath
            signature = mixMaskHash(signature, hashCGPath(worldCGPath))
            for clip in worldClips {
                signature = mixMaskHash(signature, hashCGPath(clip.cgPath))
            }
            signature = mixMaskHash(signature, hole.opacity.bitPattern)
            return ResolvedMaskHole(
                signature: hole.signature,
                path: worldPath,
                clips: worldClips,
                cgPath: worldCGPath,
                clipCGPaths: worldClips.map(\.cgPath),
                opacity: hole.opacity
            )
        }

        guard !worldFrameClips.isEmpty || !worldHoles.isEmpty else {
            return nil
        }

        return WorldNativeMask(
            signature: signature,
            frameClips: worldFrameClips,
            frameClipCGPaths: worldFrameClipCGPaths,
            holes: worldHoles
        )
    }

    private func maskTranslated(_ mask: WorldNativeMask, into bounds: CGRect) -> ResolvedNativeMask {
        let translation = CGAffineTransform(translationX: -bounds.minX, y: -bounds.minY)
        let translatedFrameClips = mask.frameClips.map { $0.applying(translation) }
        let translatedHoles = mask.holes.map { hole in
            let translatedPath = hole.path.applying(translation)
            let translatedClips = hole.clips.map { $0.applying(translation) }
            return ResolvedMaskHole(
                signature: hole.signature,
                path: translatedPath,
                clips: translatedClips,
                cgPath: translatedPath.cgPath,
                clipCGPaths: translatedClips.map(\.cgPath),
                opacity: hole.opacity
            )
        }
        return ResolvedNativeMask(
            signature: mask.signature,
            size: bounds.size,
            frameClips: translatedFrameClips,
            frameClipCGPaths: translatedFrameClips.map(\.cgPath),
            holes: translatedHoles
        )
    }

    private func renderGroupedItem(_ item: NativeRenderItem, groupBounds: CGRect) -> AnyView {
        var transform = itemTransformInParentFrame(item)
        transform.tx -= groupBounds.minX
        transform.ty -= groupBounds.minY
        let base = item.content
            .position(x: item.size.width / 2.0, y: item.size.height / 2.0)
            .transformEffect(transform)
            .zIndex(Double(item.zIndex))
            .opacity(item.opacity)
            .transaction { transaction in
                transaction.animation = nil
                transaction.disablesAnimations = true
            }
        return AnyView(base)
    }

    private func groupedRenderItems(_ items: [NativeRenderItem]) -> [NativeRenderGroup] {
        var groups: [NativeRenderGroup] = []
        var currentItems: [NativeRenderItem] = []
        var currentMask: WorldNativeMask?

        func stableGroupId(for items: [NativeRenderItem]) -> String {
            items.map { String($0.id) }.joined(separator: "-")
        }

        func flushCurrentGroup() {
            guard !currentItems.isEmpty else {
                return
            }
            let bounds = currentItems
                .map { transformedBounds(size: $0.size, transform: itemTransformInParentFrame($0)) }
                .reduce(into: CGRect.null) { partialResult, rect in
                    partialResult = partialResult.union(rect)
                }
            let resolvedBounds = bounds.isNull ? CGRect(origin: .zero, size: .zero) : bounds.integral
            let groupMask = currentMask.map { maskTranslated($0, into: resolvedBounds) }
            groups.append(
                NativeRenderGroup(
                    id: stableGroupId(for: currentItems),
                    parentFrame: currentItems[0].parentFrame,
                    bounds: resolvedBounds,
                    mask: groupMask,
                    items: currentItems
                )
            )
            currentItems.removeAll(keepingCapacity: true)
            currentMask = nil
        }

        for item in items {
            let itemMask = worldMask(for: item)
            if currentItems.isEmpty {
                currentItems = [item]
                currentMask = itemMask
                continue
            }
            if let currentMask,
               let itemMask,
               currentItems[0].parentFrame == item.parentFrame,
               currentMask.signature == itemMask.signature {
                currentItems.append(item)
                continue
            }
            flushCurrentGroup()
            currentItems = [item]
            currentMask = itemMask
        }

        flushCurrentGroup()
        return groups
    }

    private func groupedView(for group: NativeRenderGroup) -> AnyView {
        let groupContent = AnyView(
            ZStack(alignment: .topLeading) {
                ForEach(group.items) { item in
                    renderGroupedItem(item, groupBounds: group.bounds)
                }
            }
            .frame(width: group.bounds.width, height: group.bounds.height, alignment: .topLeading)
        )
        let masked = applyResolvedMask(groupContent, mask: group.mask)
        let outerTransform = parentFrameTransform(group.parentFrame)
        let base = masked
            .position(x: group.bounds.midX, y: group.bounds.midY)
            .transformEffect(outerTransform)
            .transaction { transaction in
                transaction.animation = nil
                transaction.disablesAnimations = true
            }
        return AnyView(base)
    }
#endif

    private func attributedString(for element: TextElement) -> AttributedString {
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

    private func textItem(for element: TextElement) -> NativeRenderItem {
        let measuredWidth = element.size_x >= 0 ? CGFloat(element.size_x) : element.lastMeasuredSize?.width ?? 0
        let measuredHeight = element.size_y >= 0 ? CGFloat(element.size_y) : element.lastMeasuredSize?.height ?? 0

        if element.editable {
            return renderTextContent(
                EditableTextView(element: element),
                element: element,
                width: measuredWidth,
                height: measuredHeight
            )
        }

        let baseText = Text(attributedString(for: element))
            .foregroundColor(element.textStyle.fill)
            .font(element.textStyle.font.getFont(size: element.textStyle.font_size))
            .frame(
                width: measuredWidth > 0 ? measuredWidth : nil,
                height: measuredHeight > 0 ? measuredHeight : nil,
                alignment: element.textStyle.alignment
            )
            .allowsHitTesting(element.selectable)

        let text: AnyView
        if element.selectable {
            text = AnyView(baseText.textSelection(.enabled))
        } else {
            text = AnyView(baseText.textSelection(.disabled))
        }

        return renderTextContent(text, element: element, width: measuredWidth, height: measuredHeight)
    }

    private func buttonItem(for element: ButtonElement) -> NativeRenderItem {
        let button = Button(action: {
            dispatchFormButtonClick(id: element.id)
        }) {
            Text(element.content.isEmpty ? " " : element.content)
                .font(element.style.font.getFont(size: element.style.font_size))
                .foregroundColor(element.style.fill)
                .multilineTextAlignment(textAlignment(from: element.style.alignmentMultiline))
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: element.style.alignment)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
        }
        .buttonStyle(.plain)
        .background(
            RoundedRectangle(cornerRadius: element.borderRadius)
                .fill(element.color)
        )
        .overlay(
            RoundedRectangle(cornerRadius: element.borderRadius)
                .stroke(element.outlineStrokeColor, lineWidth: element.outlineStrokeWidth)
        )

        return renderItemContent(button, element: element)
    }

    private func checkboxItem(for element: CheckboxElement) -> NativeRenderItem {
        let checkbox = Button(action: {
            dispatchFormCheckboxToggle(id: element.id, state: !element.checked)
        }) {
            ZStack {
                RoundedRectangle(cornerRadius: element.borderRadius)
                    .fill(element.checked ? element.backgroundChecked : element.background)
                RoundedRectangle(cornerRadius: element.borderRadius)
                    .stroke(element.outlineColor, lineWidth: element.outlineWidth)
                if element.checked {
                    Image(systemName: "checkmark")
                        .foregroundColor(.white)
                        .font(.system(size: max(10, min(resolvedSize(element).width, resolvedSize(element).height) * 0.55)))
                }
            }
        }
        .buttonStyle(.plain)

        return renderItemContent(checkbox, element: element)
    }

    private func sliderItem(for element: SliderElement) -> NativeRenderItem {
        let upperBound = element.max > element.min ? element.max : element.min + 1
        let step = element.step > 0 ? element.step : 0.001

        let slider = Slider(
            value: Binding(
                get: { element.value },
                set: { dispatchFormSliderChange(id: element.id, value: $0) }
            ),
            in: element.min...upperBound,
            step: step
        )
        .tint(element.accent)

        return renderItemContent(ZStack { slider }, element: element)
    }

    private func dropdownItem(for element: DropdownElement) -> NativeRenderItem {
        let picker = Picker(
            "",
            selection: Binding(
                get: { Int(element.selectedId) },
                set: { dispatchFormDropdownChange(id: element.id, selectedId: UInt32($0)) }
            )
        ) {
            ForEach(Array(element.options.enumerated()), id: \.offset) { index, option in
                Text(option)
                    .font(element.style.font.getFont(size: element.style.font_size))
                    .tag(index)
            }
        }
        .pickerStyle(.menu)
        .labelsHidden()
        .tint(element.style.fill)
        .background(
            RoundedRectangle(cornerRadius: element.borderRadius)
                .fill(element.background)
        )
        .overlay(
            RoundedRectangle(cornerRadius: element.borderRadius)
                .stroke(element.strokeColor, lineWidth: element.strokeWidth)
        )

        return renderItemContent(picker, element: element)
    }

    private func radioSetItem(for element: RadioSetElement) -> NativeRenderItem {
        let control = VStack(alignment: .leading, spacing: 6) {
            ForEach(Array(element.options.enumerated()), id: \.offset) { index, option in
                Button(action: {
                    dispatchFormRadioSetChange(id: element.id, selectedId: UInt32(index))
                }) {
                    HStack(spacing: 8) {
                        ZStack {
                            Circle()
                                .fill(element.background)
                            Circle()
                                .stroke(element.outlineColor, lineWidth: element.outlineWidth)
                            if Int(element.selectedId) == index {
                                Circle()
                                    .fill(element.backgroundChecked)
                                    .padding(4)
                            }
                        }
                        .frame(width: 20, height: 20)

                        Text(option)
                            .font(element.style.font.getFont(size: element.style.font_size))
                            .foregroundColor(element.style.fill)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
                .buttonStyle(.plain)
            }
        }

        return renderItemContent(control, element: element)
    }

    private func textboxItem(for element: TextboxElement) -> NativeRenderItem {
        if element.isTextArea {
            return renderItemContent(PaxTextboxArea(element: element), element: element)
        }
        return renderItemContent(PaxTextboxField(element: element), element: element)
    }

    private func nativeImageItem(for element: NativeImageElement) -> NativeRenderItem {
        let imageView = Group {
            if let url = URL(string: element.url), url.scheme != nil {
                AsyncImage(url: url) { phase in
                    switch phase {
                    case .success(let image):
                        configuredImage(image, fit: element.fit)
                    default:
                        Color.clear
                    }
                }
            } else if !element.url.isEmpty {
                if let platformImage = loadPlatformImage(path: element.url) {
                    configuredPlatformImage(platformImage, fit: element.fit)
                } else {
                    Color.clear
                }
            } else {
                Color.clear
            }
        }

        return renderItemContent(imageView, element: element)
    }

    private func youtubeVideoItem(for element: YoutubeVideoElement) -> NativeRenderItem {
        let control = Group {
            if let url = URL(string: element.url) {
                Link(destination: url) {
                    ZStack {
                        RoundedRectangle(cornerRadius: 12)
                            .fill(Color.black.opacity(0.85))
                        VStack(spacing: 8) {
                            Image(systemName: "play.rectangle.fill")
                                .font(.system(size: 28))
                                .foregroundColor(.white)
                            Text("Open Video")
                                .foregroundColor(.white)
                        }
                    }
                }
                .buttonStyle(.plain)
            } else {
                Color.clear
            }
        }

        return renderItemContent(control, element: element)
    }

    private func eventBlockerItem(for element: EventBlockerElement) -> NativeRenderItem {
        renderItemContent(EventBlockerPlatformView(), element: element)
    }

    public var body: some View {
        let _ = nativeSceneInvalidation.generation
        ZStack(alignment: .topLeading) {
            ForEach(sortedRenderItems()) { item in
                positionedItem(item)
            }
        }
        .transaction { transaction in
            transaction.animation = nil
            transaction.disablesAnimations = true
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

private func configuredImage(_ image: Image, fit: String) -> some View {
    Group {
        switch fit {
        case "cover":
            image.resizable().scaledToFill()
        case "fill":
            image.resizable()
        default:
            image.resizable().scaledToFit()
        }
    }
    .clipped()
}

#if os(iOS) || os(tvOS) || os(watchOS)
private func loadPlatformImage(path: String) -> UIImage? {
    UIImage(contentsOfFile: path)
}

private func configuredPlatformImage(_ image: UIImage, fit: String) -> some View {
    configuredImage(Image(uiImage: image), fit: fit)
}
#elseif os(macOS)
private func loadPlatformImage(path: String) -> NSImage? {
    NSImage(contentsOfFile: path)
}

private func configuredPlatformImage(_ image: NSImage, fit: String) -> some View {
    configuredImage(Image(nsImage: image), fit: fit)
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

public class RadioSetElements: ObservableObject {
    public static let singleton = RadioSetElements()
    @Published public var elements: [PaxNodeId: RadioSetElement] = [:]

    public func add(element: RadioSetElement) {
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
