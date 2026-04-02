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

private struct PendingMaskRender {
    let generation: UInt64
    let scale: CGFloat
    let payload: RasterizedNativeMaskPayload
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

    context.scaleBy(x: scale, y: scale)
    let bounds = CGRect(origin: .zero, size: payload.size)
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

    return context.makeImage()
}

#if os(iOS) || os(tvOS) || os(watchOS)
private func platformColor(_ color: Color) -> UIColor {
    UIColor(color)
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
            let image = rasterizedMaskImage(
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
            payload: rasterPayload(from: mask),
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

private final class LayerMaskedHostingController: NSViewController {
    private static let maskRasterQueue = DispatchQueue(
        label: "dev.pax.swift.native-mask-raster",
        qos: .userInitiated
    )

    private let hostingController = NSHostingController(rootView: AnyView(EmptyView()))
    private let maskLayer = CALayer()
    private var appliedMaskSignature: UInt64?
    private var appliedMaskSize: CGSize = .zero
    private var requestedMaskSignature: UInt64?
    private var requestedMaskSize: CGSize = .zero
    private var nextMaskGeneration: UInt64 = 0
    private var inFlightMaskRender: PendingMaskRender?
    private var queuedMaskRender: PendingMaskRender?

    override func loadView() {
        let rootView = NSView()
        rootView.wantsLayer = true
        rootView.layer?.backgroundColor = NSColor.clear.cgColor
        rootView.layer?.mask = maskLayer
        self.view = rootView
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        hostingController.view.translatesAutoresizingMaskIntoConstraints = false
        addChild(hostingController)
        view.addSubview(hostingController.view)
        NSLayoutConstraint.activate([
            hostingController.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            hostingController.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            hostingController.view.topAnchor.constraint(equalTo: view.topAnchor),
            hostingController.view.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        ])
    }

    private static func currentMaskScale() -> CGFloat {
        NSScreen.main?.backingScaleFactor ?? 1.0
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
            let image = rasterizedMaskImage(
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
            payload: rasterPayload(from: mask),
            scale: Self.currentMaskScale()
        )
    }
}

private struct LayerMaskedView: NSViewControllerRepresentable {
    let content: AnyView
    let mask: ResolvedNativeMask

    func makeNSViewController(context: Context) -> LayerMaskedHostingController {
        LayerMaskedHostingController()
    }

    func updateNSViewController(_ controller: LayerMaskedHostingController, context: Context) {
        controller.update(rootView: content, mask: mask)
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

    private struct FrameRenderNode: Identifiable {
        let id: PaxNodeId
        let zIndex: Int
        let parentFrame: PaxNodeId?
        let localTransform: CGAffineTransform
        let size: CGSize
        let opacity: Double
        let clipPath: Path?
        let children: [NativeRenderNode]
    }

    private enum NativeRenderNode: Identifiable {
        case item(NativeRenderItem)
        case frame(FrameRenderNode)

        var id: String {
            switch self {
            case .item(let item):
                return "item-\(item.id)"
            case .frame(let frame):
                return "frame-\(frame.id)"
            }
        }

        var zIndex: Int {
            switch self {
            case .item(let item):
                return item.zIndex
            case .frame(let frame):
                return frame.zIndex
            }
        }

        var numericId: PaxNodeId {
            switch self {
            case .item(let item):
                return item.id
            case .frame(let frame):
                return frame.id
            }
        }
    }

    private func clampOpacity(_ opacity: Double) -> Double {
        min(max(opacity, 0.0), 1.0)
    }

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
        return AnyView(
            LayerMaskedView(content: AnyView(view), mask: mask)
                .frame(width: mask.size.width, height: mask.size.height)
                .transaction { transaction in
                    transaction.animation = nil
                    transaction.disablesAnimations = true
                }
        )
    }

    private func applyFrameClip<V: View>(_ view: V, clipPath: Path?) -> AnyView {
        guard let clipPath else {
            return AnyView(view)
        }
        return AnyView(view.clipShape(ResolvedPathShape(resolvedPath: clipPath)))
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
            opacity: clampOpacity(element.opacity),
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
            opacity: clampOpacity(element.opacity),
            content: bounded,
            mask: resolvedNativeMask(for: element.id)
        )
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

    private func frameTransformInParentFrame(_ frame: FrameElement) -> CGAffineTransform {
        let parentInverse = safeInverse(parentFrameTransform(frame.parentFrame))
        return affineTransform(from: frame.transform).concatenating(parentInverse)
    }

    private func frameSize(_ frame: FrameElement) -> CGSize {
        CGSize(width: max(0, CGFloat(frame.size_x)), height: max(0, CGFloat(frame.size_y)))
    }

    private func localClipPath(for frame: FrameElement) -> Path? {
        guard frame.clipContent else {
            return nil
        }
        let size = frameSize(frame)
        if let clipPath = frame.clipPath,
           !clipPath.isEmpty,
           let worldPath = parseSVGPath(clipPath) {
            return worldPath.applying(safeInverse(affineTransform(from: frame.transform)))
        }
        return Path(CGRect(origin: .zero, size: size))
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
        let framesByParent = Dictionary(grouping: Array(frameElements.elements.values), by: { $0.parentFrame })

        var activeFrames: [PaxNodeId: Bool] = [:]
        func frameHasNativeDescendants(_ frameId: PaxNodeId) -> Bool {
            if let cached = activeFrames[frameId] {
                return cached
            }
            let hasItems = !(itemsByParent[frameId] ?? []).isEmpty
            let hasDescendants = (framesByParent[frameId] ?? []).contains { frame in
                frameHasNativeDescendants(frame.id)
            }
            let isActive = hasItems || hasDescendants
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
                clipPath: localClipPath(for: frame),
                children: children
            )
        }

        func buildChildren(parent: PaxNodeId?) -> [NativeRenderNode] {
            let frameNodes = (framesByParent[parent] ?? [])
                .filter { frameHasNativeDescendants($0.id) }
                .map { NativeRenderNode.frame(buildFrameNode($0)) }
            let itemNodes = (itemsByParent[parent] ?? []).map(NativeRenderNode.item)
            return sortedNodes(frameNodes + itemNodes)
        }

        return buildChildren(parent: nil)
    }

    private func positionedItem(_ item: NativeRenderItem) -> AnyView {
        let localMasked = applyResolvedMask(item.content, mask: item.mask)
        let base = localMasked
            .position(x: item.size.width / 2.0, y: item.size.height / 2.0)
            .transformEffect(itemTransformInParentFrame(item))
            .zIndex(Double(item.zIndex))
            .opacity(item.opacity)
            .transaction { transaction in
                transaction.animation = nil
                transaction.disablesAnimations = true
            }
        return AnyView(base)
    }

    private func renderFrameNode(_ frame: FrameRenderNode) -> AnyView {
        let content = AnyView(
            ZStack(alignment: .topLeading) {
                ForEach(frame.children) { child in
                    renderNode(child)
                }
            }
            .frame(width: frame.size.width, height: frame.size.height, alignment: .topLeading)
        )
        let clipped = applyFrameClip(content, clipPath: frame.clipPath)
        let base = clipped
            .compositingGroup()
            .position(x: frame.size.width / 2.0, y: frame.size.height / 2.0)
            .transformEffect(frame.localTransform)
            .zIndex(Double(frame.zIndex))
            .opacity(frame.opacity)
            .transaction { transaction in
                transaction.animation = nil
                transaction.disablesAnimations = true
            }
        return AnyView(base)
    }

    private func renderNode(_ node: NativeRenderNode) -> AnyView {
        switch node {
        case .item(let item):
            return positionedItem(item)
        case .frame(let frame):
            return renderFrameNode(frame)
        }
    }

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
            ForEach(buildRenderTree()) { node in
                renderNode(node)
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
