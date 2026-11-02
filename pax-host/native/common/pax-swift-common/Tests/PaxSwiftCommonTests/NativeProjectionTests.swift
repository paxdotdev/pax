#if os(macOS)
import AppKit
import SwiftUI
import Messages
import XCTest
@testable import Rendering

final class NativeProjectionTests: XCTestCase {
    private let matrix: [Float] = [1,0,0,0.0012, 0,1,0,0.00045, 0,0,1,0, 70,100,0,1]

    func testPlaneTransportCompositionAndInverse() throws {
        let projection = try XCTUnwrap(PaxNativeProjection(matrix))
        let inverse = try XCTUnwrap(projection.inverse())
        let child = PaxNativeProjection(affine: CGAffineTransform(translationX: 25, y: 110))
        for point in [CGPoint.zero, CGPoint(x: 400, y: 300), CGPoint(x: -80, y: 900)] {
            let actual = try XCTUnwrap((projection * child).project(point))
            let x = point.x + 25, y = point.y + 110
            let w = 1 + CGFloat(matrix[3])*x + CGFloat(matrix[7])*y
            XCTAssertEqual(actual.x, (x + 70)/w, accuracy: 1e-9)
            XCTAssertEqual(actual.y, (y + 100)/w, accuracy: 1e-9)
            let restored = try XCTUnwrap(inverse.project(actual))
            XCTAssertEqual(restored.x, x, accuracy: 1e-9)
            XCTAssertEqual(restored.y, y, accuracy: 1e-9)
        }
        XCTAssertNil(PaxNativeProjection([Float](repeating: 0, count: 16))?.inverse())
        XCTAssertNil(PaxNativeProjection([1, 2, 3] as [Float]))
        XCTAssertNil(projection.project(CGPoint(x: -1000, y: 0)))
    }

    func testProjectedCurvedClipPreservesItsInterior() throws {
        let projection = try XCTUnwrap(PaxNativeProjection(matrix))
        let ellipse = CGPath(ellipseIn: CGRect(x: 0, y: 0, width: 100, height: 80), transform: nil)
        let path = try XCTUnwrap(projection.projectPath(ellipse))
        for x in stride(from: -10.0, through: 110.0, by: 5) {
            for y in stride(from: -10.0, through: 90.0, by: 5) {
                let radiusSquared = pow((x-50)/50, 2) + pow((y-40)/40, 2)
                if abs(radiusSquared-1) < 0.02 { continue }
                let point = try XCTUnwrap(projection.project(CGPoint(x: x, y: y)))
                XCTAssertEqual(path.contains(point), radiusSquared < 1)
            }
        }
    }

    private final class Host: NSView, PaxProjectedNativeHost {
        override var isFlipped: Bool { true }
        var nativeProjection: PaxNativeProjection?
        var focusedOwner: NSView?
        func updateProjectedFocus(_ owner: NSView?, focused: Bool) {
            if focused { focusedOwner = owner }
            else if focusedOwner === owner { focusedOwner = nil }
        }
    }

    @MainActor
    func testTextScreenQueriesAndAutoscrollUseTheProjectedViewport() throws {
        _ = NSApplication.shared
        let window = NSWindow(contentRect: NSRect(x: 100, y: 100, width: 950, height: 680),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let host = Host(frame: NSRect(x: 0, y: 0, width: 740, height: 480))
        host.nativeProjection = try XCTUnwrap(PaxNativeProjection(matrix))
        let root = Host(frame: NSRect(x: 0, y: 0, width: 950, height: 680))
        window.contentView = root
        root.addSubview(host)
        let scroll = NSScrollView(frame: NSRect(x: 25, y: 110, width: 650, height: 300))
        scroll.contentView = PaxProjectedClipView()
        host.addSubview(scroll)
        let text = PaxTextboxTextView(frame: NSRect(x: 0, y: 0, width: 650, height: 1200))
        text.font = .monospacedSystemFont(ofSize: 22, weight: .regular)
        text.string = (0..<40).map { "Line \($0): 0123456789 ABCDEFGHIJ abcdefghij" }.joined(separator: "\n")
        text.isVerticallyResizable = true
        text.isHorizontallyResizable = false
        text.textContainer?.widthTracksTextView = true
        scroll.documentView = text
        text.layoutManager!.ensureLayout(for: text.textContainer!)
        XCTAssertTrue(window.makeFirstResponder(text))
        XCTAssertTrue(host.focusedOwner === scroll)

        for offset in [CGFloat(0), 100] {
            scroll.contentView.scroll(to: NSPoint(x: 0, y: offset))
            let range = NSRange(location: offset == 0 ? 10 : 215, length: 1)
            let rect = text.accessibilityFrame(for: range)
            let point = NSPoint(x: rect.midX, y: rect.midY)
            XCTAssertEqual(text.characterIndex(for: point), range.location)
            XCTAssertEqual(text.accessibilityRange(for: point), range)
            var actual = NSRange()
            let imeRect = text.firstRect(forCharacterRange: range, actualRange: &actual)
            XCTAssertTrue(imeRect.intersects(rect))
        }

        scroll.contentView.scroll(to: .zero)
        let logical = host.convert(NSPoint(x: 100, y: 450), to: nil)
        let visual = try XCTUnwrap(text.paxVisualWindowPoint(logical))
        let event = try XCTUnwrap(NSEvent.mouseEvent(with: .leftMouseDragged, location: visual,
            modifierFlags: [], timestamp: 1, windowNumber: window.windowNumber, context: nil,
            eventNumber: 1, clickCount: 1, pressure: 1))
        XCTAssertTrue(text.autoscroll(with: event))
        XCTAssertGreaterThan(scroll.contentView.bounds.origin.y, 0)
        XCTAssertTrue(window.makeFirstResponder(nil))
        XCTAssertNil(host.focusedOwner)
    }

    @MainActor
    func testProjectedControlRemainsVisibleInsideScrolledAncestor() throws {
        _ = NSApplication.shared
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 600, height: 400),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let scroll = NSScrollView(frame: CGRect(x: 0, y: 0, width: 600, height: 400))
        let document = Host(frame: CGRect(x: 0, y: 0, width: 600, height: 2400))
        scroll.documentView = document
        window.contentView = scroll
        let host = NativeRenderingLayer.PlatformContainerView(frame: .zero)
        document.addSubview(host)
        let field = PaxProjectedTextField(frame: CGRect(x: 0, y: 0, width: 240, height: 42))
        field.stringValue = "Visible native text"
        host.addSubview(field)
        let projection = try XCTUnwrap(PaxNativeProjection(
            [1,0,0,0.0001, 0,1,0,0, 0,0,1,0, 70,1700,0,1] as [Float]))
        host.applyGeometry(size: field.bounds.size, localTransform: .identity,
                           projection: projection, zIndex: 0, opacity: 1)
        scroll.contentView.scroll(to: CGPoint(x: 0, y: 1600))
        window.contentView?.layoutSubtreeIfNeeded()
        XCTAssertFalse(field.visibleRect.isEmpty,
            "AppKit must draw a projected control that is visible in a scrolled viewport")
        let local = CGPoint(x: 100, y: 20)
        let visual = document.convert(try XCTUnwrap(projection.project(local)), to: nil)
        let actual = field.convert(visual, from: nil)
        XCTAssertEqual(actual.x, local.x, accuracy: 1e-8)
        XCTAssertEqual(actual.y, local.y, accuracy: 1e-8)
    }

    @MainActor
    func testOffscreenProjectedControlHasFiniteEmptyVisibleRectAndRecovers() throws {
        _ = NSApplication.shared
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 600, height: 400),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let root = Host(frame: CGRect(x: 0, y: 0, width: 600, height: 400))
        window.contentView = root
        let host = NativeRenderingLayer.PlatformContainerView(frame: .zero)
        root.addSubview(host)
        let field = PaxProjectedTextField(frame: CGRect(x: 0, y: 0, width: 240, height: 42))
        field.stringValue = "Retained while offscreen"
        host.addSubview(field)
        let nested = NativeRenderingLayer.PlatformContainerView(frame: .zero)
        host.addSubview(nested)
        nested.applyGeometry(size: CGSize(width: 20, height: 20), localTransform: .identity,
                             projection: .identity, zIndex: 0, opacity: 1)

        for y: Float in [700, 100, -200, 100] {
            let projection = try XCTUnwrap(PaxNativeProjection(
                [1,0,0,0.0001, 0,1,0,0, 0,0,1,0, 70,y,0,1] as [Float]))
            host.applyGeometry(size: field.bounds.size, localTransform: .identity,
                               projection: projection, zIndex: 0, opacity: 1)
            let visible = host.visibleRect
            XCTAssertTrue(visible.origin.x.isFinite && visible.origin.y.isFinite,
                          "An offscreen visibleRect must be empty, never CGRect.null (infinite origin)")
            XCTAssertTrue(visible.width.isFinite && visible.height.isFinite)
            XCTAssertEqual(visible.isEmpty, y != 100)
            XCTAssertEqual(nested.visibleRect.isEmpty, y != 100,
                           "An empty parent viewport must stay empty through nested projection")
            window.contentView?.layoutSubtreeIfNeeded()
            XCTAssertEqual(field.stringValue, "Retained while offscreen")
        }
    }

    @MainActor
    func testProjectedControlsKeepLayoutConversionsLogical() throws {
        _ = NSApplication.shared
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 600, height: 400),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let root = Host(frame: CGRect(x: 0, y: 0, width: 600, height: 400))
        window.contentView = root
        let host = Host(frame: CGRect(x: 2000, y: 700, width: 240, height: 42))
        root.addSubview(host)
        host.nativeProjection = try XCTUnwrap(PaxNativeProjection(
            [1,0,0,0.001, 0,1,0,0, 0,0,1,0, 2000,700,0,1] as [Float]))
        // The window origin is beyond this plane's inverse horizon. AppKit's
        // convertSize implementation probes it to subtract translation, even
        // though the control and its logical layout are perfectly finite.
        XCTAssertNil(host.nativeProjection?.inverse()?.project(root.convert(.zero, from: nil)))
        let views: [NSView] = [PaxProjectedButton(), PaxProjectedSlider(), PaxProjectedPopUpButton(),
            PaxProjectedTextField(), PaxProjectedImageView(), PaxProjectedTextView(), PaxProjectedClipView()]
        let frame = CGRect(x: 10, y: 5, width: 140, height: 32)
        for view in views {
            view.frame = frame
            host.addSubview(view)
            let reference = view.isFlipped ? Host(frame: frame) : NSView(frame: frame)
            host.addSubview(reference)
            let size = NSSize(width: 80, height: 20)
            let rect = NSRect(x: 100, y: 50, width: 80, height: 20)
            XCTAssertEqual(view.convert(size, from: nil), reference.convert(size, from: nil), "\(type(of: view))")
            XCTAssertEqual(view.convert(rect, from: nil), reference.convert(rect, from: nil), "\(type(of: view))")
            let local = CGPoint(x: 30, y: 10)
            let visual = root.convert(try XCTUnwrap(host.nativeProjection?.project(
                CGPoint(x: local.x + frame.minX, y: local.y + frame.minY))), to: nil)
            let actual = view.convert(visual, from: nil)
            XCTAssertEqual(actual.x, local.x, accuracy: 1e-8)
            XCTAssertEqual(actual.y, view.isFlipped ? local.y : frame.height - local.y, accuracy: 1e-8)
            view.removeFromSuperview()
            reference.removeFromSuperview()
        }
    }

    @MainActor
    func testSceneTextboxKeepsNativeEditorAndLiveMaskAcrossProjectionChanges() throws {
        PaxNativeHostState.reset()
        defer { setResolvedNativeMask(id: 1, mask: nil); PaxNativeHostState.reset() }
        let element = TextboxElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        element.transform = matrix
        element.size_x = 240; element.size_y = 42
        element.text = "Native editing stays mounted"
        TextboxElements.singleton.add(element: element)
        let path = Path(CGRect(x: 0, y: 0, width: 50, height: 42))
        setResolvedNativeMask(id: 1, mask: ResolvedNativeMask(signature: 1055,
            size: CGSize(width: 240, height: 42), holes: [ResolvedMaskHole(signature: 1055,
                path: path, clips: [], cgPath: path.cgPath, clipCGPaths: [], opacity: 1)]))
        let invalidation = NativeSceneInvalidation.singleton
        invalidation.invalidate()
        let scene = NSHostingView(rootView: NativeRenderingLayer())
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 600, height: 400),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = scene
        defer { window.close() }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        func flush() {
            scene.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.03))
            scene.layoutSubtreeIfNeeded()
        }
        flush()
        let field = try XCTUnwrap(descendants(scene).compactMap { $0 as? PaxProjectedTextField }.first)
        let leaf = try XCTUnwrap(field.superview as? NativeRenderingLayer.PlatformMaskedLeafView)
        let original = try XCTUnwrap(leaf.nativeProjection)
        XCTAssertTrue(CATransform3DEqualToTransform(leaf.layer!.transform, original.layerTransform),
                      "Attaching native content must preserve its host's projective layer transform: \(leaf.layer!.transform)")
        XCTAssertEqual(field.alphaValue, 1)
        XCTAssertNotNil(leaf.layer?.mask)
        XCTAssertFalse(leaf.layer?.sublayers?.contains { $0.zPosition == 1_000 } ?? false)
        XCTAssertNil(leaf.hitTest(try XCTUnwrap(original.project(CGPoint(x: 20, y: 20)))))
        XCTAssertNotNil(leaf.hitTest(try XCTUnwrap(original.project(CGPoint(x: 180, y: 20)))))
        XCTAssertTrue(window.makeFirstResponder(field))
        let editor = try XCTUnwrap(field.currentEditor() as? PaxTextboxTextView)
        editor.setSelectedRange(NSRange(location: 7, length: 4))
        for next in [[Float](matrix.enumerated().map { $0.offset == 12 ? $0.element+80 : $0.element }),
                     [1,0,0,1,70,100]] {
            element.transform = next
            invalidation.invalidate(ids: [1], rebuild: false)
            flush()
            XCTAssertTrue(field.currentEditor() === editor)
            XCTAssertTrue(window.firstResponder === editor)
            XCTAssertEqual(editor.selectedRange(), NSRange(location: 7, length: 4))
            XCTAssertEqual(field.alphaValue, 1)
            XCTAssertNotNil(leaf.layer?.mask)
        }
        XCTAssertNil(leaf.nativeProjection)
        XCTAssertTrue(window.makeFirstResponder(nil))
        element.transform = matrix
        invalidation.invalidate(ids: [1], rebuild: false)
        flush()
        let ring = leaf.layer?.sublayers?.first { $0.zPosition == 2_000 } as? CAShapeLayer
        XCTAssertNil(ring?.path, "Returning to perspective must not restore a stale focus ring")
    }
}
#endif
