import XCTest
import SwiftUI
import Messages
import FlexBuffers
@testable import Rendering

#if os(macOS)
final class NativeScrollerTests: XCTestCase {
    @MainActor
    private func fixture(transform: [Float] = [1, 0, 0, 1, 0, 0],
                         _ body: (NSView, ScrollerElement, NSScrollView, () -> [CGPoint]) throws -> Void) throws {
        PaxNativeHostState.reset()
        defer { PaxNativeHostState.reset() }
        var positions: [CGPoint] = []
        let previous = NativeInterruptDispatcher.shared.sendData
        NativeInterruptDispatcher.shared.sendData = { data in
            guard let root = FlexBuffer.decode(data: data),
                  let x = root["Scrollbar"]?["scroll_x"]?.asDouble,
                  let y = root["Scrollbar"]?["scroll_y"]?.asDouble else { return }
            positions.append(CGPoint(x: x, y: y))
        }
        defer { NativeInterruptDispatcher.shared.sendData = previous }
        let model = ScrollerElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        model.size_x = 400
        model.size_y = 300
        model.transform = transform
        model.sizeInnerPaneX = 8192
        model.sizeInnerPaneY = 8192
        model.scrollX = 3896
        model.scrollY = 3946
        ScrollerElements.singleton.add(element: model)
        NativeSceneInvalidation.singleton.invalidate()
        let host = NSHostingView(rootView: NativeRenderingLayer())
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 600, height: 500),
            styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = host
        defer { window.close() }
        flush(host)
        let scroll = try XCTUnwrap(descendants(host).compactMap { $0 as? NSScrollView }.first)
        try body(host, model, scroll, { positions })
    }

    @MainActor
    private func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }

    @MainActor
    private func flush(_ host: NSView) {
        host.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.03))
        host.layoutSubtreeIfNeeded()
    }

    @MainActor
    private func update(_ host: NSView) {
        NativeSceneInvalidation.singleton.invalidate(ids: [1], rebuild: false)
        flush(host)
    }

    @MainActor
    func testInitialPositionAndResizeDoNotBecomeUserScrolls() throws {
        try fixture { host, model, scroll, positions in
            XCTAssertEqual(scroll.contentView.bounds.origin, CGPoint(x: 3896, y: 3946))
            XCTAssertTrue(positions().isEmpty, "Mounting must not overwrite the authored position")
            model.size_x = 500
            model.size_y = 350
            update(host)
            XCTAssertEqual(scroll.contentView.bounds.origin, CGPoint(x: 3896, y: 3946))
            XCTAssertTrue(positions().isEmpty, "Resizing is not a scroll gesture")
            NativeScrollerHostRegistry.shared.updateScrollPosition(id: 1, position: CGPoint(x: 3846, y: 3921))
            XCTAssertEqual(scroll.contentView.bounds.origin, CGPoint(x: 3846, y: 3921),
                "A layout notification must not start the native-gesture quiet period")
        }
    }

    @MainActor
    func testContentResizePreservesRequestedPositionAndNativeScrollStillPublishes() throws {
        try fixture { host, model, scroll, positions in
            model.sizeInnerPaneX = 200
            model.sizeInnerPaneY = 200
            update(host)
            XCTAssertEqual(scroll.contentView.bounds.origin, .zero)
            XCTAssertTrue(positions().isEmpty)
            model.sizeInnerPaneX = 8192
            model.sizeInnerPaneY = 8192
            update(host)
            XCTAssertEqual(scroll.contentView.bounds.origin, CGPoint(x: 3896, y: 3946))
            XCTAssertTrue(positions().isEmpty)
            // Move the actual native clip view, as AppKit does after wheel/trackpad input.
            let nativePosition = CGPoint(x: 3800, y: 3900)
            scroll.contentView.scroll(to: nativePosition)
            scroll.reflectScrolledClipView(scroll.contentView)
            XCTAssertEqual(positions().last, nativePosition)
            // A full reconciliation must not replay the model's now-stale initial offset.
            model.opacity = 0.8
            update(host)
            XCTAssertEqual(scroll.contentView.bounds.origin, nativePosition)
        }
    }

    @MainActor
    func testExplicitPositionArrivingBeforeContentGrowthIsRestoredAfterLayout() throws {
        try fixture { host, model, scroll, positions in
            // Native message handling delivers scroll offsets before the geometry
            // reconciliation from the same patch (for example, graph zoom).
            NativeScrollerHostRegistry.shared.updateScrollPosition(id: 1, position: .zero)
            model.sizeInnerPaneX = 200
            model.sizeInnerPaneY = 200
            update(host)
            NativeScrollerHostRegistry.shared.updateScrollPosition(id: 1, position: CGPoint(x: 6000, y: 6100))
            model.sizeInnerPaneX = 16384
            model.sizeInnerPaneY = 16384
            update(host)
            XCTAssertEqual(scroll.contentView.bounds.origin, CGPoint(x: 6000, y: 6100))
            XCTAssertTrue(positions().isEmpty)
        }
    }

    @MainActor
    func testFractionalPlacementDoesNotPublishAppKitPixelAlignmentAsScrolling() throws {
        try fixture(transform: [1, 0, 0, 1, 20.24, 30.12]) { host, model, scroll, positions in
            XCTAssertTrue(positions().isEmpty, "Attaching/placing a scroller must not erase its initial position")
            XCTAssertEqual(scroll.contentView.bounds.origin.x, 3896, accuracy: 0.5)
            XCTAssertEqual(scroll.contentView.bounds.origin.y, 3946, accuracy: 0.5)
            model.transform[4] += 0.13
            model.transform[5] += 0.17
            update(host)
            XCTAssertTrue(positions().isEmpty, "Moving the container must not enter the scroll input path")
            NativeScrollerHostRegistry.shared.updateScrollPosition(id: 1, position: CGPoint(x: 3700, y: 3800))
            XCTAssertEqual(scroll.contentView.bounds.origin.x, 3700, accuracy: 0.5)
            XCTAssertEqual(scroll.contentView.bounds.origin.y, 3800, accuracy: 0.5)
        }
    }
}
#endif
