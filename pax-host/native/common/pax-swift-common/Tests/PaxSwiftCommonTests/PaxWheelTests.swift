#if os(macOS)
import AppKit
import XCTest
import FlexBuffers
import Messages
@testable import Rendering

func makePaxTestWheel(x: Int32 = -30, y: Int32 = -20, flags: CGEventFlags = []) -> NSEvent {
    let event = CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 2,
                        wheel1: y, wheel2: x, wheel3: 0)!
    event.flags = flags
    return NSEvent(cgEvent: event)!
}

final class PaxWheelTests: XCTestCase {
    func testWheelEncodingUsesScrollDirectionAndUntypedModifiers() throws {
        let wheel = PaxWheelEvent(event: makePaxTestWheel(
            flags: [.maskShift, .maskControl, .maskAlternate, .maskCommand]), point: CGPoint(x: 120, y: 230))
        let root = try XCTUnwrap(FlexBuffer.decode(data: wheel.encoded()))
        let payload = try XCTUnwrap(root["Wheel"])
        XCTAssertEqual(payload["x"]?.asDouble, 120)
        XCTAssertEqual(payload["y"]?.asDouble, 230)
        XCTAssertEqual(payload["delta_x"]?.asDouble, 30)
        XCTAssertEqual(payload["delta_y"]?.asDouble, 20)
        XCTAssertEqual(payload["modifiers"]?.asVector?.makeIterator().compactMap { $0.asString },
                       ["Shift", "Control", "Alt", "Command"])
        let reverse = PaxWheelEvent(event: makePaxTestWheel(x: 30, y: 20), point: .zero)
        XCTAssertEqual(reverse.deltaX, -30)
        XCTAssertEqual(reverse.deltaY, -20)
    }

    @MainActor
    func testResponderChainForwardsOnceAndPreservesCancellation() throws {
        let dispatcher = NativeInterruptDispatcher.shared
        let previousSend = dispatcher.wheelInterrupt
        let previousConversion = dispatcher.convertWindowPointToPax
        defer {
            dispatcher.wheelInterrupt = previousSend
            dispatcher.convertWindowPointToPax = previousConversion
        }
        var messages: [Data] = []
        var prevented = true
        dispatcher.wheelInterrupt = { messages.append($0); return prevented }
        dispatcher.convertWindowPointToPax = { _, _ in CGPoint(x: 15, y: 25) }
        let inner = NSView(), outer = NSView()
        let event = makePaxTestWheel()
        XCTAssertTrue(PaxWheelInput.forward(event, from: inner))
        XCTAssertTrue(PaxWheelInput.forward(event, from: outer))
        XCTAssertEqual(messages.count, 1, "Nested responders must not deliver the wheel twice")
        let root = try XCTUnwrap(FlexBuffer.decode(data: messages[0]))
        XCTAssertEqual(root["Wheel"]?["x"]?.asDouble, 15)
        XCTAssertEqual(root["Wheel"]?["y"]?.asDouble, 25)
        prevented = false
        let next = makePaxTestWheel()
        XCTAssertFalse(PaxWheelInput.forward(next, from: inner))
        XCTAssertFalse(PaxWheelInput.forward(next, from: outer))
        XCTAssertEqual(messages.count, 2)
        dispatcher.convertWindowPointToPax = nil
        XCTAssertFalse(PaxWheelInput.forward(makePaxTestWheel(), from: inner))
        XCTAssertEqual(messages.count, 2, "Detached content must leave wheel input native")
    }
}
#endif
