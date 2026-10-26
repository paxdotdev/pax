#if os(macOS)
import AppKit
import Messages

// NSTextView consumes mouse-up inside its selection tracking loop. Bracket that
// loop with the complete Pax sequence while leaving caret/selection and focus native.
final class PaxTextboxTextView: NSTextView {
    weak var activationView: NSView?

    override func mouseDown(with event: NSEvent) {
        forwardMouseSequence(from: event) {
            super.mouseDown(with: event)
            return NSApp.currentEvent
        }
    }

    func forwardMouseSequence(from start: NSEvent, trackSelection: () -> NSEvent?) {
        guard start.type == .leftMouseDown,
              let startPoint = NativeInterruptDispatcher.shared.convertWindowPoint(
                  start.locationInWindow, in: window
              ) else {
            _ = trackSelection()
            return
        }
        dispatchPointerMouseInterrupt(type: "MouseDown", x: Double(startPoint.x),
                                      y: Double(startPoint.y), focusCanvas: false)
        guard let completed = trackSelection(), Self.isRelease(from: start, to: completed),
              let point = NativeInterruptDispatcher.shared.convertWindowPoint(
                  completed.locationInWindow, in: window
              ) else {
            dispatchPointerMouseInterrupt(type: "MouseCancel", x: Double(startPoint.x),
                                          y: Double(startPoint.y), focusCanvas: false)
            return
        }
        dispatchPointerMouseInterrupt(type: "MouseUp", x: Double(point.x), y: Double(point.y),
                                      focusCanvas: false)
        let hitView = activationView ?? self
        guard Self.isActivation(from: start, to: completed),
              hitView.bounds.contains(hitView.convert(completed.locationInWindow, from: nil))
        else { return }
        dispatchPointerMouseInterrupt(type: "Click", x: Double(point.x), y: Double(point.y))
    }

    private static func isRelease(from start: NSEvent, to end: NSEvent) -> Bool {
        start.type == .leftMouseDown && end.type == .leftMouseUp
            && start.windowNumber == end.windowNumber
            && end.timestamp >= start.timestamp
    }

    static func isActivation(from start: NSEvent, to end: NSEvent) -> Bool {
        // A completed selection drag releases capture but does not activate.
        isRelease(from: start, to: end)
            && hypot(end.locationInWindow.x - start.locationInWindow.x,
                     end.locationInWindow.y - start.locationInWindow.y) <= 4
    }
}
#endif
