#if os(macOS)
import AppKit
import XCTest
import FlexBuffers
import Messages
@testable import Rendering

final class TextboxActivationTests: XCTestCase {
    private func event(_ type: NSEvent.EventType, x: CGFloat = 20,
                       timestamp: TimeInterval = 1, window: Int = 1) -> NSEvent {
        NSEvent.mouseEvent(with: type, location: NSPoint(x: x, y: 30),
                          modifierFlags: [], timestamp: timestamp,
                          windowNumber: window, context: nil,
                          eventNumber: 1, clickCount: 1, pressure: 0)!
    }

    func testCompletedPrimaryClickActivates() {
        XCTAssertTrue(PaxTextboxTextView.isActivation(
            from: event(.leftMouseDown), to: event(.leftMouseUp, timestamp: 2)))
    }

    func testTextSelectionDragDoesNotActivate() {
        XCTAssertFalse(PaxTextboxTextView.isActivation(
            from: event(.leftMouseDown), to: event(.leftMouseUp, x: 60, timestamp: 2)))
    }

    func testIncompleteOrUnrelatedEventDoesNotActivate() {
        let down = event(.leftMouseDown)
        XCTAssertFalse(PaxTextboxTextView.isActivation(from: down, to: down))
        XCTAssertFalse(PaxTextboxTextView.isActivation(
            from: down, to: event(.leftMouseUp, timestamp: 0)))
        XCTAssertFalse(PaxTextboxTextView.isActivation(
            from: down, to: event(.leftMouseUp, timestamp: 2, window: 2)))
        XCTAssertFalse(PaxTextboxTextView.isActivation(
            from: event(.rightMouseDown), to: event(.rightMouseUp, timestamp: 2)))
    }

    @MainActor
    private func capture(_ body: (PaxTextboxTextView, () -> [Data]) throws -> Void) rethrows -> [Data] {
        _ = NSApplication.shared
        let dispatcher = NativeInterruptDispatcher.shared
        let previousSender = dispatcher.sendData
        let previousConversion = dispatcher.convertWindowPointToPax
        let previousFocus = dispatcher.requestKeyboardFocus
        var messages: [Data] = []
        var focusRequests = 0
        dispatcher.sendData = { messages.append($0) }
        dispatcher.convertWindowPointToPax = { point, _ in
            CGPoint(x: point.x + 100, y: point.y + 200)
        }
        dispatcher.requestKeyboardFocus = { focusRequests += 1 }
        defer {
            dispatcher.sendData = previousSender
            dispatcher.convertWindowPointToPax = previousConversion
            dispatcher.requestKeyboardFocus = previousFocus
        }
        let view = PaxTextboxTextView(frame: NSRect(x: 0, y: 0, width: 100, height: 100))
        try body(view, { messages })
        XCTAssertEqual(focusRequests, 0, "Text selection must not move keyboard focus to the canvas")
        return messages
    }

    private func names(_ messages: [Data]) throws -> [String] {
        try messages.map { data in
            let root = try XCTUnwrap(FlexBuffer.decode(data: data))
            let kinds: [StaticString] = ["MouseDown", "MouseUp", "MouseCancel", "Click"]
            return try XCTUnwrap(kinds.first { root[$0] != nil }).description
        }
    }

    @MainActor
    func testClickBracketsNativeTrackingAndPreservesTextFocus() throws {
        let messages = try capture { view, sent in
            view.forwardMouseSequence(from: event(.leftMouseDown)) {
                XCTAssertEqual(try? self.names(sent()), ["MouseDown"])
                return self.event(.leftMouseUp, x: 22, timestamp: 2)
            }
            XCTAssertEqual(try names(sent()), ["MouseDown", "MouseUp", "Click"])
        }
        let down = try XCTUnwrap(FlexBuffer.decode(data: messages[0]))
        let up = try XCTUnwrap(FlexBuffer.decode(data: messages[1]))
        let click = try XCTUnwrap(FlexBuffer.decode(data: messages[2]))
        XCTAssertEqual(down["MouseDown"]?["x"]?.asDouble, 120)
        XCTAssertEqual(up["MouseUp"]?["x"]?.asDouble, 122)
        XCTAssertEqual(click["Click"]?["x"]?.asDouble, 122)
        XCTAssertEqual(click["Click"]?["y"]?.asDouble, 230)
        XCTAssertEqual(click["Click"]?["button"]?.asString, "Left")
    }

    @MainActor
    func testSelectionDragReleasesWithoutClick() throws {
        let messages = capture { view, _ in
            view.forwardMouseSequence(from: event(.leftMouseDown)) {
                self.event(.leftMouseUp, x: 60, timestamp: 2)
            }
        }
        XCTAssertEqual(try names(messages), ["MouseDown", "MouseUp"])
    }

    @MainActor
    func testShortReleaseOutsideActivationBoundsDoesNotClick() throws {
        let messages = capture { view, _ in
            let control = NSView(frame: NSRect(x: 0, y: 0, width: 21, height: 100))
            view.activationView = control
            view.forwardMouseSequence(from: event(.leftMouseDown)) {
                self.event(.leftMouseUp, x: 22, timestamp: 2)
            }
        }
        XCTAssertEqual(try names(messages), ["MouseDown", "MouseUp"])
    }

    @MainActor
    func testInterruptedTrackingCancelsAndNextClickStillWorks() throws {
        for end in [nil, event(.leftMouseDown), event(.leftMouseUp, timestamp: 0),
                    event(.leftMouseUp, timestamp: 2, window: 2)] {
            let messages = capture { view, _ in
                view.forwardMouseSequence(from: event(.leftMouseDown)) { end }
                view.forwardMouseSequence(from: event(.leftMouseDown)) {
                    self.event(.leftMouseUp, timestamp: 2)
                }
            }
            XCTAssertEqual(try names(messages), ["MouseDown", "MouseCancel", "MouseDown", "MouseUp", "Click"])
            let cancelled = try XCTUnwrap(FlexBuffer.decode(data: messages[1]))
            XCTAssertEqual(cancelled["MouseCancel"]?["x"]?.asDouble, 120)
        }
    }

    @MainActor
    func testLostHostConversionCancelsCapture() throws {
        let messages = capture { view, _ in
            view.forwardMouseSequence(from: event(.leftMouseDown)) {
                NativeInterruptDispatcher.shared.convertWindowPointToPax = nil
                return self.event(.leftMouseUp, timestamp: 2)
            }
        }
        XCTAssertEqual(try names(messages), ["MouseDown", "MouseCancel"])
    }

    @MainActor
    func testUnhostedTextboxStillTracksNativelyWithoutPartialSequence() throws {
        var tracked = false
        let messages = capture { view, _ in
            NativeInterruptDispatcher.shared.convertWindowPointToPax = nil
            view.forwardMouseSequence(from: event(.leftMouseDown)) {
                tracked = true
                return self.event(.leftMouseUp, timestamp: 2)
            }
        }
        XCTAssertTrue(tracked)
        XCTAssertTrue(messages.isEmpty)
    }
}
#endif
