#if os(macOS)
import AppKit

// Keep TextKit's editing and composition state native. Geometry crossing the
// window/screen boundary is adapted once; native internal layout stays local.
class PaxProjectedTextView: NSTextView {
    weak var projectionFocusOwner: NSView?

    private func withLogicalGeometry<T>(_ action: () -> T) -> T {
        PaxNativeLayoutScope.perform(action)
    }

    override func convert(_ point: NSPoint, from view: NSView?) -> NSPoint {
        guard view == nil, !PaxNativeLayoutScope.isActive, paxHasNativeProjection else {
            return super.convert(point, from: view)
        }
        guard let logical = paxLogicalWindowPoint(point) else { return NSPoint(x: CGFloat.infinity, y: CGFloat.infinity) }
        return super.convert(logical, from: nil)
    }

    override func convert(_ size: NSSize, from view: NSView?) -> NSSize {
        PaxNativeLayoutScope.perform { super.convert(size, from: view) }
    }

    override func convert(_ rect: NSRect, from view: NSView?) -> NSRect {
        PaxNativeLayoutScope.perform { super.convert(rect, from: view) }
    }

    override func becomeFirstResponder() -> Bool {
        let accepted = super.becomeFirstResponder()
        if accepted { paxUpdateProjectedFocus(projectionFocusOwner ?? enclosingScrollView ?? self, focused: true) }
        return accepted
    }

    override func resignFirstResponder() -> Bool {
        let accepted = super.resignFirstResponder()
        if accepted { paxUpdateProjectedFocus(projectionFocusOwner ?? enclosingScrollView ?? self, focused: false) }
        return accepted
    }

    override func firstRect(forCharacterRange range: NSRange, actualRange: NSRangePointer?) -> NSRect {
        guard !PaxNativeLayoutScope.isActive, paxHasNativeProjection else {
            return super.firstRect(forCharacterRange: range, actualRange: actualRange)
        }
        return paxVisualScreenRect(withLogicalGeometry {
            super.firstRect(forCharacterRange: range, actualRange: actualRange)
        })
    }

    override func characterIndex(for point: NSPoint) -> Int {
        guard !PaxNativeLayoutScope.isActive, paxHasNativeProjection else { return super.characterIndex(for: point) }
        guard let logical = paxLogicalScreenPoint(point) else { return NSNotFound }
        return withLogicalGeometry { super.characterIndex(for: logical) }
    }

    override func accessibilityFrame() -> NSRect {
        guard !PaxNativeLayoutScope.isActive, paxHasNativeProjection else { return super.accessibilityFrame() }
        return paxVisualScreenRect(withLogicalGeometry { super.accessibilityFrame() })
    }

    override func accessibilityFrame(for range: NSRange) -> NSRect {
        guard !PaxNativeLayoutScope.isActive, paxHasNativeProjection else { return super.accessibilityFrame(for: range) }
        return paxVisualScreenRect(withLogicalGeometry { super.accessibilityFrame(for: range) })
    }

    override func accessibilityRange(for point: NSPoint) -> NSRange {
        guard !PaxNativeLayoutScope.isActive, paxHasNativeProjection else { return super.accessibilityRange(for: point) }
        guard let logical = paxLogicalScreenPoint(point) else { return NSRange(location: NSNotFound, length: 0) }
        return withLogicalGeometry { super.accessibilityRange(for: logical) }
    }

    override func accessibilityHitTest(_ point: NSPoint) -> Any? {
        guard !PaxNativeLayoutScope.isActive, paxHasNativeProjection else { return super.accessibilityHitTest(point) }
        guard let logical = paxLogicalScreenPoint(point) else { return nil }
        return withLogicalGeometry { super.accessibilityHitTest(logical) }
    }

}
#endif
