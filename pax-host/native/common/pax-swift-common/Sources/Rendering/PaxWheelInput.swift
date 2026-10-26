#if os(macOS)
import AppKit
import FlexBuffers
import Messages

struct PaxWheelEvent {
    let point: CGPoint
    let deltaX: Double
    let deltaY: Double
    let modifiers: [String]

    init(event: NSEvent, point: CGPoint) {
        self.point = point
        // AppKit reports content movement; Pax/web report the scroll direction.
        deltaX = -Double(event.scrollingDeltaX)
        deltaY = -Double(event.scrollingDeltaY)
        modifiers = [(.shift, "Shift"), (.control, "Control"), (.option, "Alt"), (.command, "Command")]
            .compactMap { (flag: NSEvent.ModifierFlags, name: String) in
                event.modifierFlags.contains(flag) ? name : nil
            }
    }

    func encoded() -> Data {
        try! FlexBufferBuilder.encodeMap { root in
            root.addMapWithStringKey("Wheel") { event in
                event.add("x", Double(point.x))
                event.add("y", Double(point.y))
                event.add("delta_x", deltaX)
                event.add("delta_y", deltaY)
                // Keep an untyped vector: Rust's enum decoder needs String, not Key values.
                event.addVector("modifiers") { vector in
                    for modifier in modifiers { vector.addString(modifier) }
                }
            }
        }.data
    }
}

/// Forwards AppKit wheel input once through Pax before the native scrolling default.
@MainActor
public enum PaxWheelInput {
    private static weak var lastEvent: NSEvent?
    private static var lastPrevented = false

    /// Returns true when a Pax handler prevented the native default.
    public static func forward(_ event: NSEvent, from view: NSView) -> Bool {
        guard let send = NativeInterruptDispatcher.shared.wheelInterrupt,
              let point = NativeInterruptDispatcher.shared.convertWindowPoint(
                event.locationInWindow, in: event.window ?? view.window
              ) else { return false }
        // An unhandled wheel can traverse nested NSScrollViews and then the canvas.
        // Keep event identity weak so this never retains the native input stream.
        if lastEvent === event { return lastPrevented }
        lastEvent = event
        lastPrevented = false
        lastPrevented = send(PaxWheelEvent(event: event, point: point).encoded())
        return lastPrevented
    }
}
#endif
