import XCTest
import FlexBuffers
import Messages
@testable import Rendering

final class PaxTouchSequenceTests: XCTestCase {
    private func contact(_ id: Int64 = 1, _ x: Double = 30, _ y: Double = 40)
        -> PaxTouchSequence.Contact {
        .init(identifier: id, x: x, y: y)
    }

    private func capture(_ body: (PaxTouchSequence) -> Void) throws -> [String] {
        var interrupts: [Data] = []
        let previous = NativeInterruptDispatcher.shared.sendData
        NativeInterruptDispatcher.shared.sendData = { interrupts.append($0) }
        defer { NativeInterruptDispatcher.shared.sendData = previous }
        body(PaxTouchSequence())
        return try interrupts.map { data in
            let root = try XCTUnwrap(FlexBuffer.decode(data: data))
            let names: [StaticString] = ["TouchStart", "TouchMove", "TouchEnd", "TouchCancel", "Tap"]
            return try XCTUnwrap(names.first { root[$0] != nil }).description
        }
    }

    func testTapIsForwardedExactlyOnceAndNextTapStillWorks() throws {
        let events = try capture { sequence in
            for _ in 0..<2 {
                sequence.forward(.began, contacts: [contact()])
                sequence.forward(.ended, contacts: [contact()])
                sequence.forward(.ended, contacts: [contact()])
            }
        }
        XCTAssertEqual(events, ["TouchStart", "TouchEnd", "Tap", "TouchStart", "TouchEnd", "Tap"])
    }

    func testDragReturningToStartDoesNotDismissUnderlay() throws {
        let events = try capture { sequence in
            sequence.forward(.began, contacts: [contact()])
            sequence.forward(.moved, contacts: [contact(1, 60, 40)])
            sequence.forward(.ended, contacts: [contact()])
        }
        XCTAssertEqual(events, ["TouchStart", "TouchMove", "TouchEnd"])
    }

    func testMovementOnFinalEventAlsoRejectsTap() throws {
        let events = try capture { sequence in
            sequence.forward(.began, contacts: [contact()])
            sequence.forward(.ended, contacts: [contact(1, 60, 40)])
        }
        XCTAssertEqual(events, ["TouchStart", "TouchEnd"])
    }

    func testCancellationDoesNotTapAndDoesNotPoisonNextSequence() throws {
        let events = try capture { sequence in
            sequence.forward(.began, contacts: [contact()])
            sequence.forward(.cancelled, contacts: [contact()])
            sequence.forward(.began, contacts: [contact()])
            sequence.forward(.ended, contacts: [contact()])
        }
        XCTAssertEqual(events, ["TouchStart", "TouchCancel", "TouchStart", "TouchEnd", "Tap"])
    }

    func testSecondFingerPreventsTapEvenWhenItEndsFirst() throws {
        let events = try capture { sequence in
            sequence.forward(.began, contacts: [contact()])
            sequence.forward(.began, contacts: [contact(2)])
            sequence.forward(.ended, contacts: [contact(2)])
            sequence.forward(.ended, contacts: [contact()])
        }
        XCTAssertEqual(events, ["TouchStart", "TouchStart", "TouchEnd", "TouchEnd"])
    }

    func testCoordinatesDeltasAndRemainingContactsAreForwarded() throws {
        var interrupts: [Data] = []
        let previous = NativeInterruptDispatcher.shared.sendData
        NativeInterruptDispatcher.shared.sendData = { interrupts.append($0) }
        defer { NativeInterruptDispatcher.shared.sendData = previous }
        let sequence = PaxTouchSequence()
        sequence.forward(.began, contacts: [contact(1, 10, 20), contact(2, 50, 60)])
        sequence.forward(.moved, contacts: [contact(2, 53, 56)])
        let root = try XCTUnwrap(FlexBuffer.decode(data: try XCTUnwrap(interrupts.last)))
        let vector = try XCTUnwrap(root["TouchMove"]?["touches"]?.asVector)
        let iterator = vector.makeIterator()
        let changed = try XCTUnwrap(iterator.next())
        XCTAssertEqual(changed["identifier"]?.asInt, 2)
        XCTAssertEqual(changed["x"]?.asDouble, 53)
        XCTAssertEqual(changed["y"]?.asDouble, 56)
        XCTAssertEqual(changed["delta_x"]?.asDouble, 3)
        XCTAssertEqual(changed["delta_y"]?.asDouble, -4)
        let stationary = try XCTUnwrap(iterator.next())
        XCTAssertEqual(stationary["identifier"]?.asInt, 1)
        XCTAssertEqual(stationary["delta_x"]?.asDouble, 0)
        XCTAssertEqual(stationary["delta_y"]?.asDouble, 0)
    }
}
