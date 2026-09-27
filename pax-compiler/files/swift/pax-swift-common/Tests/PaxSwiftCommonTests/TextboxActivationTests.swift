#if os(macOS)
import AppKit
import XCTest
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
}
#endif
