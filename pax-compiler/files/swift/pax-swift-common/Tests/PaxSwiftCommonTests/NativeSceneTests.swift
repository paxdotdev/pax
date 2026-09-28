import XCTest
import SwiftUI
import Messages
import FlexBuffers
@testable import Rendering

#if os(macOS)
final class NativeSceneTests: XCTestCase {
    // Exercise document fallback interaction and both label/document snapshot compositing.
    // NativeTextTests covers ordinary label interaction and representation changes.
    @MainActor
    func testCullingDetachesRetainedLeavesAndPreservesInteractionOverflowAndAccessibility() throws {
        PaxNativeHostState.reset()
        defer { PaxNativeHostState.reset() }
        let state = NativeCullingState.shared
        state.setAccessibilityActive(false)
        for id in 1...40 {
            let text = TextElement.makeDefault(id: PaxNodeId(id), parentFrame: nil, renderLayerId: 0)
            text.content = "Row \(id)"
            text.size_x = 200
            text.size_y = 40
            text.clip = true
            text.selectable = true
            text.markdown = true
            text.zIndex = id
            text.transform = [1, 0, 0, 1, 10, Float(id * 50)]
            TextElements.singleton.add(element: text)
        }
        TextElements.singleton.elements[4]!.editable = true
        let overflow = TextElements.singleton.elements[5]!
        overflow.clip = false
        overflow.wrap = false
        overflow.size_x = 8
        overflow.content = "An overflowing line must stay attached"
        let invalidation = NativeSceneInvalidation.singleton
        invalidation.invalidate()
        let host = NSHostingView(rootView: NativeRenderingLayer())
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 400, height: 400),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = host
        defer { window.close() }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        func flush() {
            host.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.03))
            host.layoutSubtreeIfNeeded()
        }
        flush()
        let originals = descendants(host).compactMap { $0 as? NSTextView }
        XCTAssertEqual(originals.count, 40)
        let row1 = try XCTUnwrap(originals.first { $0.string == "Row 1" })
        let row2 = try XCTUnwrap(originals.first { $0.string == "Row 2" })
        let row3 = try XCTUnwrap(originals.first { $0.string == "Row 3" })
        let row1Leaf = try XCTUnwrap(row1.enclosingScrollView?.superview?.superview)
        row2.setSelectedRange(NSRange(location: 0, length: 3))
        XCTAssertTrue(window.makeFirstResponder(row3))
        let rebuilds = invalidation.fullReconciliations
        let sink = NativeSceneTestSink()
        let data = try FlexBufferBuilder.encodeMap { root in
            root.addVector("messages") { messages in
                messages.addMap { message in
                    message.addMap("NativeCullUpdate") { patch in
                        patch.add("cull", (1...40).map { UInt($0) })
                        patch.add("restore", [UInt]())
                    }
                }
            }
        }
        sink.processNativeMessageQueueData(data.data)
        flush()
        XCTAssertNil(row1Leaf.superview)
        XCTAssertTrue(window.firstResponder === row3)
        XCTAssertEqual(row2.selectedRange(), NSRange(location: 0, length: 3))
        XCTAssertEqual(descendants(host).compactMap { $0 as? NSTextView }.count, 4)
        XCTAssertEqual(invalidation.fullReconciliations, rebuilds)
        // Cold views still receive content and geometry updates, without mounting.
        let model = TextElements.singleton.elements[1]!
        model.content = "Updated while cold"
        model.transform[5] = 25
        invalidation.invalidate(ids: [1], rebuild: false)
        flush()
        XCTAssertNil(row1Leaf.superview)
        XCTAssertEqual(row1.string, "Updated while cold")
        XCTAssertEqual(row1Leaf.frame.origin.y, 25)
        state.apply(cull: [], restore: [1, 20, 30])
        invalidation.invalidate(ids: [1, 20, 30], rebuild: false)
        flush()
        XCTAssertNotNil(row1Leaf.superview)
        XCTAssertTrue(descendants(host).contains { $0 === row1 })
        let siblings = row1Leaf.superview!.subviews
        let z = siblings.compactMap { $0.layer?.zPosition }
        XCTAssertEqual(z, z.sorted())
        XCTAssertEqual(invalidation.fullReconciliations, rebuilds)
        // Accessibility changes restore the complete native hierarchy live.
        state.setAccessibilityActive(true)
        flush()
        XCTAssertEqual(descendants(host).compactMap { $0 as? NSTextView }.count, 40)
        state.setAccessibilityActive(false)
        flush()
        XCTAssertEqual(descendants(host).compactMap { $0 as? NSTextView }.count, 7)
        // Releasing selection/focus permits culling on the next journal update.
        row2.setSelectedRange(NSRange(location: 0, length: 0))
        XCTAssertTrue(window.makeFirstResponder(nil))
        invalidation.invalidate(ids: [1], rebuild: false)
        flush()
        XCTAssertEqual(descendants(host).compactMap { $0 as? NSTextView }.count, 5)
        // Removing a detached node must discard its retained view, not resurrect it.
        TextElements.singleton.elements.removeValue(forKey: 40)
        invalidation.invalidate()
        flush()
        state.apply(cull: [], restore: Array(1...40))
        invalidation.invalidate()
        flush()
        XCTAssertEqual(descendants(host).compactMap { $0 as? NSTextView }.count, 39)
    }


    @MainActor
    func testSnapshotContentChangesKeepMaskAndUnmaskRestoresNativeContent() throws {
        try checkSnapshotContentChanges(markdown: false)
        try checkSnapshotContentChanges(markdown: true)
    }

    @MainActor
    private func checkSnapshotContentChanges(markdown: Bool) throws {
        PaxNativeHostState.reset()
        defer { PaxNativeHostState.reset() }
        let text = TextElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        text.content = String(repeating: "M", count: 60)
        text.selectable = true
        text.markdown = markdown
        text.clip = true
        text.size_x = 200
        text.size_y = 40
        TextElements.singleton.add(element: text)
        let path = Path(CGRect(x: 0, y: 0, width: 100, height: 40))
        setResolvedNativeMask(id: 1, mask: ResolvedNativeMask(signature: 1,
            size: CGSize(width: 200, height: 40), holes: [ResolvedMaskHole(signature: 1,
                path: path, clips: [], cgPath: path.cgPath, clipCGPaths: [], opacity: 1)]))
        let invalidation = NativeSceneInvalidation.singleton
        invalidation.invalidate()
        let host = NSHostingView(rootView: NativeRenderingLayer())
        host.frame = CGRect(x: 0, y: 0, width: 400, height: 400)
        func flush() {
            host.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.03))
            host.layoutSubtreeIfNeeded()
        }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        flush()
        let content = try XCTUnwrap(descendants(host).compactMap { $0 as? PaxNativeTextLeafView }.first)
        let native = try XCTUnwrap(content.subviews.first)
        let leaf = try XCTUnwrap(content.superview)
        let snapshot = try XCTUnwrap(leaf.layer?.sublayers?.first { $0.zPosition == 1_000 })
        func checkMaskedPixels() throws {
            let image = try XCTUnwrap(snapshot.contents as! CGImage?)
            var pixels = [UInt8](repeating: 0, count: 200 * 40 * 4)
            pixels.withUnsafeMutableBytes { bytes in
                let context = CGContext(data: bytes.baseAddress, width: 200, height: 40,
                    bitsPerComponent: 8, bytesPerRow: 800, space: CGColorSpaceCreateDeviceRGB(),
                    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
                context.draw(image, in: CGRect(x: 0, y: 0, width: 200, height: 40))
            }
            var left = 0, right = 0
            for y in 0..<40 {
                for x in 0..<99 { left += Int(pixels[(y * 200 + x) * 4 + 3]) }
                for x in 101..<200 { right += Int(pixels[(y * 200 + x) * 4 + 3]) }
            }
            XCTAssertEqual(left, 0)
            XCTAssertGreaterThan(right, 0)
        }
        try checkMaskedPixels()
        text.content = String(repeating: "W", count: 60)
        invalidation.invalidate(ids: [1], rebuild: false)
        flush()
        try checkMaskedPixels()
        XCTAssertEqual(content.alphaValue, 0)
        setResolvedNativeMask(id: 1, mask: nil)
        invalidation.invalidate(ids: [1], rebuild: false)
        flush()
        XCTAssertNil(snapshot.superlayer)
        XCTAssertEqual(content.alphaValue, 1)
        XCTAssertFalse(content.isHidden)
        XCTAssertTrue(descendants(host).contains { $0 === native })
    }

    @MainActor
    func testGeometryFieldsPreserveCoordinatesWithoutUnrelatedWrites() throws {
        PaxNativeHostState.reset()
        defer { PaxNativeHostState.reset() }
        let text = TextElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        text.content = "Selectable content"
        text.selectable = true
        text.markdown = true
        text.clip = true
        text.size_x = 200
        text.size_y = 40
        text.transform = [1, 0, 0, 1, 20, 30]
        TextElements.singleton.add(element: text)
        let invalidation = NativeSceneInvalidation.singleton
        invalidation.invalidate()
        let host = NSHostingView(rootView: NativeRenderingLayer())
        host.frame = CGRect(x: 0, y: 0, width: 400, height: 400)
        func flush() {
            host.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.03))
            host.layoutSubtreeIfNeeded()
        }
        func descendants(_ view: NSView) -> [NSView] {
            [view] + view.subviews.flatMap(descendants)
        }
        func update() {
            invalidation.invalidate(ids: [1], rebuild: false)
            flush()
        }
        flush()
        let native = try XCTUnwrap(descendants(host).compactMap { $0 as? NSTextView }.first)
        let leaf = try XCTUnwrap(native.enclosingScrollView?.superview?.superview)
        native.setSelectedRange(NSRange(location: 0, length: 4))

        for degrees in [0.0, 30.0, -20.0] {
            let radians = degrees * .pi / 180
            let a = Float(cos(radians) * 0.92), b = Float(sin(radians) * 0.92)
            text.transform = [a, b, -b, a, 20, 30]
            update()
            let point = leaf.convert(CGPoint(x: 10, y: 5), to: leaf.superview)
            XCTAssertEqual(point.x, CGFloat(20 + a * 10 - b * 5), accuracy: 0.001)
            XCTAssertEqual(point.y, CGFloat(30 + b * 10 + a * 5), accuracy: 0.001)
            XCTAssertEqual(leaf.bounds.width, 200, accuracy: 0.001)
            XCTAssertEqual(leaf.bounds.height, 40, accuracy: 0.001)
#if DEBUG
            invalidation.geometryWrites = NativeGeometryWrites()
#endif
            text.opacity = text.opacity == 0.5 ? 0.8 : 0.5
            update()
#if DEBUG
            XCTAssertEqual(invalidation.geometryWrites, NativeGeometryWrites(opacity: 1))
#endif
            XCTAssertEqual(leaf.convert(CGPoint(x: 10, y: 5), to: leaf.superview), point)
            XCTAssertEqual(native.selectedRange(), NSRange(location: 0, length: 4))
        }

        // Shear uses a backing-layer transform; returning to rotation must restore
        // AppKit's coordinate conversion, including when opacity changes afterwards.
        text.transform = [1, 0, 0.2, 1, 20, 30]
        update()
        XCTAssertEqual(leaf.layer?.affineTransform().c ?? 0, 0.2, accuracy: 0.001)
        text.transform = [0, 1, -1, 0, 20, 30]
        update()
        let rotated = leaf.convert(CGPoint(x: 10, y: 5), to: leaf.superview)
        XCTAssertEqual(rotated.x, 15, accuracy: 0.001)
        XCTAssertEqual(rotated.y, 40, accuracy: 0.001)
#if DEBUG
        invalidation.geometryWrites = NativeGeometryWrites()
#endif
        text.transform[4] += 7
        update()
#if DEBUG
        XCTAssertEqual(invalidation.geometryWrites, NativeGeometryWrites(origin: 1))
        invalidation.geometryWrites = NativeGeometryWrites()
#endif
        text.zIndex += 1
        update()
#if DEBUG
        XCTAssertEqual(invalidation.geometryWrites, NativeGeometryWrites(stacking: 1))
#endif
        XCTAssertEqual(native.selectedRange(), NSRange(location: 0, length: 4))
    }

    @MainActor
    func testLeafUpdatesKeepNativeViewsAndSkipTreeReconciliation() {
        PaxNativeHostState.reset()
        defer { PaxNativeHostState.reset() }
        for id in 1...100 {
            let text = TextElement.makeDefault(id: PaxNodeId(id), parentFrame: nil, renderLayerId: 0)
            text.content = "Row \(id)"
            text.selectable = true
            text.markdown = true
            text.clip = true
            text.size_x = 200
            text.size_y = 20
            text.transform = [1, 0, 0, 1, 0, Float(id * 20)]
            TextElements.singleton.add(element: text)
        }
        let invalidation = NativeSceneInvalidation.singleton
        invalidation.invalidate()
        let host = NSHostingView(rootView: NativeRenderingLayer())
        host.frame = CGRect(x: 0, y: 0, width: 400, height: 400)
        func flush() {
            host.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.05))
            host.layoutSubtreeIfNeeded()
        }
        func identities(_ view: NSView) -> Set<ObjectIdentifier> {
            let controls: Set<ObjectIdentifier> = (view is NSTextView || view is NSScrollView)
                ? [ObjectIdentifier(view)] : []
            return view.subviews.reduce(into: controls) { result, child in
                result.formUnion(identities(child))
            }
        }
        flush()
        let original = identities(host)
        XCTAssertGreaterThan(original.count, 100)
        let rebuilds = invalidation.fullReconciliations
        let updates = invalidation.leafUpdates
        let text = TextElements.singleton.elements[50]!
        text.content = "Changed row"
        text.transform[5] += 5
        invalidation.invalidate(ids: [50], rebuild: false)
        flush()
        XCTAssertEqual(invalidation.fullReconciliations, rebuilds)
        XCTAssertEqual(invalidation.leafUpdates, updates + 1)
        XCTAssertEqual(identities(host), original)
        // Scaling retains the same native text view, logical content size, and selection.
        func textViews(_ view: NSView) -> [NSTextView] {
            (view as? NSTextView).map { [$0] } ?? view.subviews.flatMap(textViews)
        }
        if let nativeText = textViews(host).first(where: { $0.string == "Changed row" }) {
            nativeText.setSelectedRange(NSRange(location: 0, length: 7))
            let originalSize = nativeText.frame.size
            for scale: Float in [0.92, 1.04, 0.98, 1] {
                text.transform[0] = scale
                text.transform[3] = scale
                invalidation.invalidate(ids: [50], rebuild: false)
                flush()
                XCTAssertEqual(nativeText.frame.size, originalSize)
                XCTAssertEqual(nativeText.selectedRange(), NSRange(location: 0, length: 7))
            }
            XCTAssertEqual(identities(host), original)
            XCTAssertEqual(invalidation.fullReconciliations, rebuilds)
        } else {
            XCTFail("Expected a selectable native text view")
        }
        // A placement change must reconcile order even when delivered as a leaf patch.
        text.zIndex += 100
        invalidation.invalidate(ids: [50], rebuild: false)
        flush()
        XCTAssertEqual(invalidation.fullReconciliations, rebuilds + 1)
        XCTAssertEqual(identities(host), original)
    }
}
#endif

#if os(macOS)

private final class NativeSceneTestSink: NativeMessageHandling {
    let textElements = TextElements.singleton
    let frameElements = FrameElements.singleton
    let buttonElements = ButtonElements.singleton
    let photoPickerElements = PhotoPickerElements.singleton
    let checkboxElements = CheckboxElements.singleton
    let scrollerElements = ScrollerElements.singleton
    let nativeImageElements = NativeImageElements.singleton
    let youtubeVideoElements = YoutubeVideoElements.singleton
    let dropdownElements = DropdownElements.singleton
    let radioListElements = RadioListElements.singleton
    let sliderElements = SliderElements.singleton
    let textboxElements = TextboxElements.singleton
    let eventBlockerElements = EventBlockerElements.singleton
    let glassSurfaceElements = GlassSurfaceElements.singleton
    func handleImageLoad(patch: ImageLoadPatch) {}
    func handleNavigate(patch: NavigationPatchMessage) {}
    func didUpdateTextElement(_ textElement: TextElement, measureGeneration: UInt64?) {}
}
#endif
