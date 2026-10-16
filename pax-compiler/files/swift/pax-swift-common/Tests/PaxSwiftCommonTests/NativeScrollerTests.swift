import XCTest
import SwiftUI
import Messages
@testable import Rendering

#if os(macOS)
final class NativeScrollerTests: XCTestCase {
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
