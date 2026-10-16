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

    @MainActor
    func testWheelHandlersCanPreventNativeScrollingWithoutDuplicateDelivery() throws {
        let dispatcher = NativeInterruptDispatcher.shared
        let previousSend = dispatcher.wheelInterrupt
        let previousConversion = dispatcher.convertWindowPointToPax
        defer {
            dispatcher.wheelInterrupt = previousSend
            dispatcher.convertWindowPointToPax = previousConversion
        }
        var deliveries = 0
        var prevented = true
        dispatcher.wheelInterrupt = { _ in deliveries += 1; return prevented }
        dispatcher.convertWindowPointToPax = { point, _ in point }
        try fixture { host, _, scroll, positions in
            let before = scroll.contentView.bounds.origin
            scroll.scrollWheel(with: makePaxTestWheel(x: 0, y: -80))
            flush(host)
            XCTAssertEqual(deliveries, 1)
            XCTAssertEqual(scroll.contentView.bounds.origin, before)
            XCTAssertTrue(positions().isEmpty)
            prevented = false
            scroll.scrollWheel(with: makePaxTestWheel(x: 0, y: -80))
            flush(host)
            XCTAssertEqual(deliveries, 2)
            XCTAssertGreaterThan(scroll.contentView.bounds.origin.y, before.y)
            XCTAssertFalse(positions().isEmpty, "Unconsumed input still uses AppKit scrolling and position feedback")
        }
    }
}

final class NestedScrollerAxisTests: XCTestCase {
    @MainActor
    private func fixture(_ body: (NSScrollView, NSScrollView, NSView) throws -> Void) throws {
        PaxNativeHostState.reset()
        defer { PaxNativeHostState.reset() }
        let outer = ScrollerElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        outer.size_x = 400
        outer.size_y = 300
        outer.sizeInnerPaneX = 400
        outer.sizeInnerPaneY = 1500
        outer.scrollEnabledX = false
        outer.scrollEnabledY = true
        outer.contentLayerId = 1
        let row = ScrollerElement.makeDefault(id: 2, parentFrame: 1, renderLayerId: 1)
        row.size_x = 400
        row.size_y = 150
        row.sizeInnerPaneX = 1600
        row.sizeInnerPaneY = 150
        row.scrollEnabledX = true
        row.scrollEnabledY = false
        row.contentLayerId = 2
        ScrollerElements.singleton.add(element: outer)
        ScrollerElements.singleton.add(element: row)
        NativeSceneInvalidation.singleton.invalidate()
        let host = NSHostingView(rootView: NativeRenderingLayer())
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 400, height: 300),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = host
        defer { window.close() }
        flush(host)
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        let scrolls = descendants(host).compactMap { $0 as? NSScrollView }
        let outerView = try XCTUnwrap(scrolls.first { $0.documentView!.frame.height > 1000 })
        let rowView = try XCTUnwrap(scrolls.first { $0.documentView!.frame.width > 1000 })
        try body(outerView, rowView, host)
    }

    @MainActor
    private func flush(_ host: NSView) {
        host.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.03))
        host.layoutSubtreeIfNeeded()
    }

    private func wheel(x: Int32, y: Int32, phase: NSEvent.Phase = [],
                       momentum: NSEvent.Phase = []) throws -> NSEvent {
        let cg = try XCTUnwrap(CGEvent(scrollWheelEvent2Source: nil, units: .pixel,
                                      wheelCount: 2, wheel1: y, wheel2: x, wheel3: 0))
        // CG and NS use different raw values for changed/ended phases.
        let cgPhase: Int64 = phase == .began ? 1 : phase == .changed ? 2
            : phase == .ended ? 4 : phase == .cancelled ? 8 : 0
        let cgMomentum: Int64 = momentum == .began ? 1 : momentum == .changed ? 2
            : momentum == .ended ? 3 : 0
        cg.setIntegerValueField(.scrollWheelEventScrollPhase, value: cgPhase)
        cg.setIntegerValueField(.scrollWheelEventMomentumPhase, value: cgMomentum)
        let event = try XCTUnwrap(NSEvent(cgEvent: cg))
        XCTAssertEqual(event.phase, phase)
        XCTAssertEqual(event.momentumPhase, momentum)
        return event
    }

    @MainActor
    func testHorizontalRowForwardsVerticalWheelAfterHorizontalGesture() throws {
        try fixture { outer, row, host in
            row.scrollWheel(with: try wheel(x: -50, y: 0))
            row.scrollWheel(with: try wheel(x: -50, y: 0))
            flush(host)
            XCTAssertGreaterThan(row.contentView.bounds.origin.x, 0)
            let x = row.contentView.bounds.origin.x
            let y = outer.contentView.bounds.origin.y
            row.scrollWheel(with: try wheel(x: 0, y: -50))
            row.scrollWheel(with: try wheel(x: 0, y: -50))
            flush(host)
            XCTAssertGreaterThan(outer.contentView.bounds.origin.y, y)
            XCTAssertEqual(row.contentView.bounds.origin.x, x, accuracy: 0.1)
            XCTAssertEqual(row.contentView.bounds.origin.y, 0, accuracy: 0.1)
            // A subsequent horizontal reversal stays in the row, not its parent.
            let outerY = outer.contentView.bounds.origin.y
            row.scrollWheel(with: try wheel(x: 30, y: 1))
            flush(host)
            XCTAssertLessThan(row.contentView.bounds.origin.x, x)
            XCTAssertEqual(outer.contentView.bounds.origin.y, outerY, accuracy: 0.1)
        }
    }

    @MainActor
    func testVerticalDominantDiagonalAndPhaseEventsForwardUnmodified() throws {
        try fixture { _, row, _ in
            let recorder = WheelRecorder()
            let previous = row.nextResponder
            row.nextResponder = recorder
            defer { row.nextResponder = previous }
            // Supported-axis trackpad events belong to AppKit's event loop;
            // forwarded events can be checked directly without starting it.
            let events = try [
                wheel(x: 2, y: -30, phase: .began),
                wheel(x: 1, y: -40, phase: .changed),
                wheel(x: 0, y: 0, phase: .ended),
                wheel(x: 0, y: -15, momentum: .began),
                wheel(x: 0, y: -8, momentum: .changed),
                wheel(x: 0, y: 0, momentum: .ended),
            ]
            for event in events { row.scrollWheel(with: event) }
            XCTAssertEqual(recorder.events.count, events.count)
            for (actual, expected) in zip(recorder.events, events) {
                XCTAssertTrue(actual === expected)
            }
        }
    }

    @MainActor
    func testVerticalScrollerForwardsHorizontalInputAndAxisPatchesStayLive() throws {
        try fixture { outer, row, host in
            let recorder = WheelRecorder()
            let previous = outer.nextResponder
            outer.nextResponder = recorder
            defer { outer.nextResponder = previous }
            let horizontal = try wheel(x: -40, y: 1)
            outer.scrollWheel(with: horizontal)
            XCTAssertTrue(recorder.events.last === horizontal)

            let model = try XCTUnwrap(ScrollerElements.singleton.elements[2])
            model.scrollEnabledX = false
            NativeSceneInvalidation.singleton.invalidate(ids: [2], rebuild: false)
            flush(host)
            let x = row.contentView.bounds.origin.x
            row.scrollWheel(with: horizontal)
            flush(host)
            XCTAssertEqual(row.contentView.bounds.origin.x, x, accuracy: 0.1)
            XCTAssertEqual(recorder.events.count, 2)
            model.scrollEnabledX = true
            NativeSceneInvalidation.singleton.invalidate(ids: [2], rebuild: false)
            flush(host)
            row.scrollWheel(with: horizontal)
            flush(host)
            XCTAssertGreaterThan(row.contentView.bounds.origin.x, x)
            XCTAssertEqual(recorder.events.count, 2)
        }
    }
}

private final class WheelRecorder: NSResponder {
    var events: [NSEvent] = []
    override func scrollWheel(with event: NSEvent) { events.append(event) }
}
#endif
