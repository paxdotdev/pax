import XCTest
import FlexBuffers
@testable import Rendering
#if os(macOS)
import AppKit
#endif

final class PaxKeyboardTests: XCTestCase {
    func testRustCompatibleModifierFixture() throws {
        // The native Rust probe consumes this exact Swift payload through pax_interrupt.
        let fixture = try String(contentsOf: XCTUnwrap(Bundle.module.url(
            forResource: "keyboard-interrupt", withExtension: "hex", subdirectory: "Fixtures")))
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let event = PaxKeyboardEvent(phase: "KeyDown", key: "+",
            modifiers: ["Shift", "Control", "Alt", "Command"], isRepeat: true)
        XCTAssertEqual(event.encoded().map { String(format: "%02x", $0) }.joined(), fixture)
    }

    func testNormalizedKeysAndWireEncoding() throws {
        XCTAssertEqual(PaxKeyboardKey.mac(code: 51, characters: "\u{7f}", fallback: ""), "Backspace")
        XCTAssertEqual(PaxKeyboardKey.hid(code: 76, characters: "", fallback: ""), "Delete")
        XCTAssertEqual(PaxKeyboardKey.hid(code: 80, characters: "", fallback: ""), "ArrowLeft")
        XCTAssertEqual(PaxKeyboardKey.mac(code: 0, characters: "A", fallback: "A"), "A")
        XCTAssertEqual(PaxKeyboardKey.hid(code: 4, characters: "\u{1}", fallback: "a"), "a")
        XCTAssertEqual(PaxKeyboardKey.mac(code: 14, characters: "", fallback: "e"), "Dead")
        for key in ["\"", "\\", "θ", "é", "🙂", "Enter"] {
            let event = PaxKeyboardEvent(phase: "KeyDown", key: key, modifiers: ["Shift", "Alt"], isRepeat: true)
            let value = try XCTUnwrap(FlexBuffer.decode(data: event.encoded()))
            XCTAssertEqual(value["KeyDown"]?["key"]?.asString, key)
            XCTAssertEqual(value["KeyDown"]?["is_repeat"]?.asBool, true)
            XCTAssertEqual(value["KeyDown"]?["modifiers"]?.asVector?.makeIterator().compactMap { $0.asString }, ["Shift", "Alt"])
        }
    }

    func testRepeatCancellationAndBalancedReleaseOnFocusLoss() {
        let state = PaxKeyboardState()
        var events: [PaxKeyboardEvent] = []
        state.send = { events.append($0); return $0.phase == "KeyDown" && $0.key == "6" }
        XCTAssertTrue(state.down(code: 22, key: "6", modifiers: []))
        XCTAssertTrue(state.down(code: 22, key: "6", modifiers: []))
        XCTAssertEqual(events.map(\.phase), ["KeyDown", "KeyDown"])
        XCTAssertEqual(events.map(\.isRepeat), [false, true])
        state.releaseAll()
        XCTAssertEqual(events.last?.phase, "KeyUp")
        XCTAssertEqual(events.last?.key, "6")
        let count = events.count
        XCTAssertFalse(state.up(code: 22, modifiers: []))
        state.releaseAll()
        XCTAssertEqual(events.count, count)
        XCTAssertFalse(state.down(code: 4, key: "a", modifiers: []))
        XCTAssertEqual(events.suffix(2).map(\.phase), ["KeyDown", "KeyPress"])
        _ = state.up(code: 4, modifiers: [])
        _ = state.down(code: 4, key: "a", modifiers: ["Command"])
        XCTAssertEqual(events.last?.phase, "KeyDown")
    }

#if os(macOS)
    @MainActor
    func testNativeFirstResponderIsolationDefaultHandlingAndTeardown() throws {
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 400, height: 300),
            styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let container = NSView(frame: CGRect(x: 0, y: 0, width: 400, height: 300))
        window.contentView = container
        let canvas = PaxKeyboardView(frame: container.bounds)
        let editor = NSTextView(frame: CGRect(x: 0, y: 0, width: 100, height: 50))
        container.addSubview(canvas)
        container.addSubview(editor)
        let fallback = KeyboardFallback()
        canvas.nextResponder = fallback
        var events: [String] = []
        canvas.keyboardInterrupt = { data in
            let value = FlexBuffer.decode(data: data)!
            if let key = value["KeyDown"]?["key"]?.asString { events.append("down:\(key)"); return key == "6" }
            if let key = value["KeyUp"]?["key"]?.asString { events.append("up:\(key)") }
            return false
        }
        func key(_ chars: String, code: UInt16 = 22) -> NSEvent {
            NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [], timestamp: 0,
                windowNumber: window.windowNumber, context: nil, characters: chars,
                charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        }
        // Invoke responder methods directly, without synthesizing system input.
        XCTAssertTrue(window.makeFirstResponder(editor))
        canvas.keyDown(with: key("6"))
        XCTAssertTrue(events.isEmpty, "A sibling native editor owns its keystrokes")
        canvas.focusForKeyboardInput()
        XCTAssertTrue(window.firstResponder === canvas)
        fallback.count = 0
        canvas.keyDown(with: key("6"))
        XCTAssertEqual(events, ["down:6"])
        XCTAssertEqual(fallback.count, 0)
        canvas.keyDown(with: key("\u{f704}", code: 122))
        XCTAssertEqual(fallback.count, 1, "Unhandled keys must reach the host responder")
        XCTAssertTrue(window.makeFirstResponder(editor))
        XCTAssertTrue(events.contains("up:6"), "Focus loss must release held keys")
        canvas.focusForKeyboardInput()
        canvas.endKeyboardInput()
        XCTAssertFalse(window.firstResponder === canvas)
        let count = events.count
        canvas.keyDown(with: key("6"))
        XCTAssertEqual(events.count, count)
    }
#endif
}

#if os(macOS)
private final class KeyboardFallback: NSResponder {
    var count = 0
    override func keyDown(with event: NSEvent) { count += 1 }
}
#endif
