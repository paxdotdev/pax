import Foundation
import FlexBuffers
#if os(macOS)
import AppKit
#else
import UIKit
#endif

struct PaxKeyboardEvent {
    let phase: String
    let key: String
    let modifiers: [String]
    let isRepeat: Bool

    func encoded() -> Data {
        // Use the encoder, not JSON interpolation: quotes, slashes and Unicode are keys too.
        try! FlexBufferBuilder.encodeMap { root in
            root.addMapWithStringKey(phase) { event in
                event.addString("key", key)
                // Rust's enum decoder requires String values. A typed string vector
                // is read as Key values by flexbuffers 2 and cannot decode modifiers.
                event.addVector("modifiers") { vector in
                    for modifier in modifiers { vector.addString(modifier) }
                }
                event.add("is_repeat", isRepeat)
            }
        }.data
    }
}

final class PaxKeyboardState {
    private struct HeldKey { let key: String; var consumed: Bool }
    private var held: [Int: HeldKey] = [:]
    var send: (PaxKeyboardEvent) -> Bool = { _ in false }

    func isHeld(_ code: Int) -> Bool { held[code] != nil }

    func down(code: Int, key: String, modifiers: [String], isRepeat: Bool = false) -> Bool {
        let repeated = isRepeat || held[code] != nil
        held[code] = HeldKey(key: key, consumed: held[code]?.consumed ?? false)
        var consumed = send(PaxKeyboardEvent(phase: "KeyDown", key: key, modifiers: modifiers, isRepeat: repeated))
        // Match web's legacy key_press boundary: cancelling key_down suppresses it.
        let printable = key == "Enter" || (key.count == 1
            && !key.unicodeScalars.contains { CharacterSet.controlCharacters.contains($0) })
        if held[code] != nil, !consumed, printable, !modifiers.contains("Control"), !modifiers.contains("Command") {
            consumed = send(PaxKeyboardEvent(phase: "KeyPress", key: key, modifiers: modifiers, isRepeat: repeated))
        }
        // A handler may have surrendered focus, releasing the key during dispatch.
        if var current = held[code] {
            current.consumed = current.consumed || consumed
            held[code] = current
        }
        return consumed
    }

    func up(code: Int, modifiers: [String]) -> Bool {
        guard let previous = held.removeValue(forKey: code) else { return false }
        let consumed = send(PaxKeyboardEvent(phase: "KeyUp", key: previous.key, modifiers: modifiers, isRepeat: false))
        return consumed || previous.consumed
    }

    func releaseAll() {
        let keys = held.sorted { $0.key < $1.key }
        held.removeAll()
        for (_, value) in keys {
            _ = send(PaxKeyboardEvent(phase: "KeyUp", key: value.key, modifiers: [], isRepeat: false))
        }
    }
}

enum PaxKeyboardKey {
    static func character(_ characters: String, fallback: String) -> String {
        if characters.isEmpty { return "Dead" }
        if characters.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }) {
            return fallback.isEmpty ? "Unidentified" : fallback
        }
        return characters
    }

    static func mac(code: Int, characters: String, fallback: String) -> String {
        let special: [Int: String] = [
            36: "Enter", 76: "Enter", 48: "Tab", 51: "Backspace", 117: "Delete",
            53: "Escape", 123: "ArrowLeft", 124: "ArrowRight", 125: "ArrowDown", 126: "ArrowUp",
            115: "Home", 119: "End", 116: "PageUp", 121: "PageDown", 114: "Insert",
            54: "Meta", 55: "Meta", 56: "Shift", 60: "Shift", 58: "Alt", 61: "Alt",
            59: "Control", 62: "Control", 57: "CapsLock", 63: "Fn",
            122: "F1", 120: "F2", 99: "F3", 118: "F4", 96: "F5", 97: "F6",
            98: "F7", 100: "F8", 101: "F9", 109: "F10", 103: "F11", 111: "F12",
            105: "F13", 107: "F14", 113: "F15", 106: "F16", 64: "F17", 79: "F18",
            80: "F19", 90: "F20", 71: "Clear",
        ]
        return special[code] ?? character(characters, fallback: fallback)
    }

    static func hid(code: Int, characters: String, fallback: String) -> String {
        let special: [Int: String] = [
            40: "Enter", 88: "Enter", 41: "Escape", 42: "Backspace", 43: "Tab",
            57: "CapsLock", 73: "Insert", 74: "Home", 75: "PageUp", 76: "Delete",
            77: "End", 78: "PageDown", 79: "ArrowRight", 80: "ArrowLeft",
            81: "ArrowDown", 82: "ArrowUp", 83: "NumLock", 101: "ContextMenu",
            224: "Control", 225: "Shift", 226: "Alt", 227: "Meta",
            228: "Control", 229: "Shift", 230: "Alt", 231: "Meta",
        ]
        if (58...69).contains(code) { return "F\(code - 57)" }
        if (104...115).contains(code) { return "F\(code - 91)" }
        return special[code] ?? character(characters, fallback: fallback)
    }
}

#if os(macOS)
/// Canvas first responder. Native controls are siblings and retain their own input paths.
open class PaxKeyboardView: NSView {
    public var keyboardInterrupt: ((Data) -> Bool)?
    private var keyboardEnabled = true
    private let keyboard = PaxKeyboardState()
    private var observers: [NSObjectProtocol] = []

    open override var acceptsFirstResponder: Bool { keyboardEnabled }

    public func focusForKeyboardInput() {
        guard keyboardEnabled, window != nil, !isHiddenOrHasHiddenAncestor else { return }
        window?.makeFirstResponder(self)
    }

    open override func viewDidMoveToWindow() {
        keyboard.releaseAll()
        for observer in observers { NotificationCenter.default.removeObserver(observer) }
        observers.removeAll()
        super.viewDidMoveToWindow()
        keyboard.send = { [weak self] event in self?.keyboardInterrupt?(event.encoded()) ?? false }
        guard let window else { return }
        for (name, object) in [(NSWindow.didResignKeyNotification, window as AnyObject),
                               (NSApplication.didResignActiveNotification, NSApplication.shared as AnyObject)] {
            observers.append(NotificationCenter.default.addObserver(forName: name, object: object, queue: .main) {
                [weak self] _ in self?.keyboard.releaseAll()
            })
        }
    }

    open override func resignFirstResponder() -> Bool {
        guard super.resignFirstResponder() else { return false }
        keyboard.releaseAll()
        return true
    }

    public func endKeyboardInput() {
        keyboard.releaseAll()
        keyboardEnabled = false
        if window?.firstResponder === self { window?.makeFirstResponder(nil) }
        keyboardInterrupt = nil
    }

    private func modifiers(_ flags: NSEvent.ModifierFlags) -> [String] {
        [(.shift, "Shift"), (.control, "Control"), (.option, "Alt"), (.command, "Command")]
            .compactMap { (flag: NSEvent.ModifierFlags, name: String) in flags.contains(flag) ? name : nil }
    }

    open override func keyDown(with event: NSEvent) {
        guard keyboardEnabled, window?.firstResponder === self else { super.keyDown(with: event); return }
        let key = PaxKeyboardKey.mac(code: Int(event.keyCode), characters: event.characters ?? "",
                                     fallback: event.charactersIgnoringModifiers ?? "")
        if !keyboard.down(code: Int(event.keyCode), key: key, modifiers: modifiers(event.modifierFlags), isRepeat: event.isARepeat) {
            super.keyDown(with: event)
        }
    }

    open override func keyUp(with event: NSEvent) {
        guard keyboardEnabled, window?.firstResponder === self,
              keyboard.up(code: Int(event.keyCode), modifiers: modifiers(event.modifierFlags)) else {
            super.keyUp(with: event); return
        }
    }

    open override func flagsChanged(with event: NSEvent) {
        guard keyboardEnabled, window?.firstResponder === self else { super.flagsChanged(with: event); return }
        let code = Int(event.keyCode)
        let flag: NSEvent.ModifierFlags
        switch code {
        case 54, 55: flag = .command
        case 56, 60: flag = .shift
        case 58, 61: flag = .option
        case 59, 62: flag = .control
        case 57: flag = .capsLock
        case 63: flag = .function
        default: super.flagsChanged(with: event); return
        }
        let down = event.modifierFlags.contains(flag) && !keyboard.isHeld(code)
        let consumed = down
            ? keyboard.down(code: code, key: PaxKeyboardKey.mac(code: code, characters: "", fallback: ""), modifiers: modifiers(event.modifierFlags))
            : keyboard.up(code: code, modifiers: modifiers(event.modifierFlags))
        if !consumed { super.flagsChanged(with: event) }
    }

    deinit { for observer in observers { NotificationCenter.default.removeObserver(observer) } }
}
#elseif os(iOS)
/// Hardware-key first responder; it does not request a software keyboard or implement text composition.
open class PaxKeyboardView: UIView {
    public var keyboardInterrupt: ((Data) -> Bool)?
    private var keyboardEnabled = true
    private let keyboard = PaxKeyboardState()
    private var inactiveObserver: NSObjectProtocol?

    open override var canBecomeFirstResponder: Bool { keyboardEnabled }

    public func focusForKeyboardInput() {
        guard keyboardEnabled, window != nil, !isHidden else { return }
        becomeFirstResponder()
    }

    open override func didMoveToWindow() {
        keyboard.releaseAll()
        super.didMoveToWindow()
        keyboard.send = { [weak self] event in self?.keyboardInterrupt?(event.encoded()) ?? false }
        if let inactiveObserver { NotificationCenter.default.removeObserver(inactiveObserver) }
        inactiveObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.willResignActiveNotification, object: nil, queue: .main
        ) { [weak self] _ in self?.keyboard.releaseAll() }
    }

    open override func resignFirstResponder() -> Bool {
        guard super.resignFirstResponder() else { return false }
        keyboard.releaseAll()
        return true
    }

    public func endKeyboardInput() {
        keyboard.releaseAll()
        keyboardEnabled = false
        resignFirstResponder()
        keyboardInterrupt = nil
    }

    private func modifiers(_ flags: UIKeyModifierFlags) -> [String] {
        [(.shift, "Shift"), (.control, "Control"), (.alternate, "Alt"), (.command, "Command")]
            .compactMap { (flag: UIKeyModifierFlags, name: String) in flags.contains(flag) ? name : nil }
    }

    private func forward(_ presses: Set<UIPress>, down: Bool) -> Set<UIPress> {
        guard keyboardEnabled, isFirstResponder else { return presses }
        return Set(presses.filter { press in
            guard let key = press.key else { return true }
            let code = key.keyCode.rawValue
            if down {
                return !keyboard.down(code: code,
                    key: PaxKeyboardKey.hid(code: code, characters: key.characters, fallback: key.charactersIgnoringModifiers),
                    modifiers: modifiers(key.modifierFlags))
            }
            return !keyboard.up(code: code, modifiers: modifiers(key.modifierFlags))
        })
    }

    open override func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        let remaining = forward(presses, down: true)
        if !remaining.isEmpty { super.pressesBegan(remaining, with: event) }
    }

    open override func pressesEnded(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        let remaining = forward(presses, down: false)
        if !remaining.isEmpty { super.pressesEnded(remaining, with: event) }
    }

    open override func pressesCancelled(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        let remaining = forward(presses, down: false)
        if !remaining.isEmpty { super.pressesCancelled(remaining, with: event) }
    }

    deinit { if let inactiveObserver { NotificationCenter.default.removeObserver(inactiveObserver) } }
}
#endif
