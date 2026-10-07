#if os(macOS)
import AppKit
import Messages

// NSTextView consumes mouse-up inside its selection tracking loop. Forward the
// completed click after that loop, so native caret/selection handling stays intact.
final class PaxTextboxTextView: NSTextView {
    weak var activationView: NSView?

    override func mouseDown(with event: NSEvent) {
        super.mouseDown(with: event)
        let hitView = activationView ?? self
        guard let completed = NSApp.currentEvent,
              Self.isActivation(from: event, to: completed),
              hitView.bounds.contains(hitView.convert(completed.locationInWindow, from: nil)),
              let point = NativeInterruptDispatcher.shared.convertWindowPoint(
                  completed.locationInWindow, in: window
              ) else { return }
        dispatchPointerMouseInterrupt(type: "Click", x: Double(point.x), y: Double(point.y))
    }

    static func isActivation(from start: NSEvent, to end: NSEvent) -> Bool {
        // Ignore cancellation, another window's event, and text-selection drags.
        start.type == .leftMouseDown && end.type == .leftMouseUp
            && start.windowNumber == end.windowNumber
            && end.timestamp >= start.timestamp
            && hypot(end.locationInWindow.x - start.locationInWindow.x,
                     end.locationInWindow.y - start.locationInWindow.y) <= 4
    }
}
#endif
