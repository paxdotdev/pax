#if os(macOS)
import AppKit

// AppKit controls retain their own tracking loops. Only window-space event
// coordinates and outward accessibility bounds cross the projective adapter.
class PaxProjectedButton: NSButton {
    override func convert(_ point: NSPoint, from view: NSView?) -> NSPoint {
        guard view == nil, !PaxNativeLayoutScope.isActive, paxHasNativeProjection else { return super.convert(point, from: view) }
        guard let logical = paxLogicalWindowPoint(point) else { return NSPoint(x: CGFloat.infinity, y: CGFloat.infinity) }
        return super.convert(logical, from: nil)
    }

    override func convert(_ size: NSSize, from view: NSView?) -> NSSize {
        PaxNativeLayoutScope.perform { super.convert(size, from: view) }
    }

    override func convert(_ rect: NSRect, from view: NSView?) -> NSRect {
        PaxNativeLayoutScope.perform { super.convert(rect, from: view) }
    }

    override func accessibilityFrame() -> NSRect {
        let rect = super.accessibilityFrame()
        return paxHasNativeProjection ? paxVisualScreenRect(rect) : rect
    }

    override func becomeFirstResponder() -> Bool {
        let accepted = super.becomeFirstResponder()
        if accepted { paxUpdateProjectedFocus(self, focused: true) }
        return accepted
    }

    override func resignFirstResponder() -> Bool {
        let accepted = super.resignFirstResponder()
        if accepted { paxUpdateProjectedFocus(self, focused: false) }
        return accepted
    }
}

class PaxProjectedSlider: NSSlider {
    override func convert(_ point: NSPoint, from view: NSView?) -> NSPoint {
        guard view == nil, !PaxNativeLayoutScope.isActive, paxHasNativeProjection else { return super.convert(point, from: view) }
        guard let logical = paxLogicalWindowPoint(point) else { return NSPoint(x: CGFloat.infinity, y: CGFloat.infinity) }
        return super.convert(logical, from: nil)
    }

    override func convert(_ size: NSSize, from view: NSView?) -> NSSize {
        PaxNativeLayoutScope.perform { super.convert(size, from: view) }
    }

    override func convert(_ rect: NSRect, from view: NSView?) -> NSRect {
        PaxNativeLayoutScope.perform { super.convert(rect, from: view) }
    }

    override func accessibilityFrame() -> NSRect {
        let rect = super.accessibilityFrame()
        return paxHasNativeProjection ? paxVisualScreenRect(rect) : rect
    }

    override func becomeFirstResponder() -> Bool {
        let accepted = super.becomeFirstResponder()
        if accepted { paxUpdateProjectedFocus(self, focused: true) }
        return accepted
    }

    override func resignFirstResponder() -> Bool {
        let accepted = super.resignFirstResponder()
        if accepted { paxUpdateProjectedFocus(self, focused: false) }
        return accepted
    }
}

class PaxProjectedPopUpButton: NSPopUpButton {
    override func convert(_ point: NSPoint, from view: NSView?) -> NSPoint {
        guard view == nil, !PaxNativeLayoutScope.isActive, paxHasNativeProjection else { return super.convert(point, from: view) }
        guard let logical = paxLogicalWindowPoint(point) else { return NSPoint(x: CGFloat.infinity, y: CGFloat.infinity) }
        return super.convert(logical, from: nil)
    }

    override func convert(_ size: NSSize, from view: NSView?) -> NSSize {
        PaxNativeLayoutScope.perform { super.convert(size, from: view) }
    }

    override func convert(_ rect: NSRect, from view: NSView?) -> NSRect {
        PaxNativeLayoutScope.perform { super.convert(rect, from: view) }
    }

    override func accessibilityFrame() -> NSRect {
        let rect = super.accessibilityFrame()
        return paxHasNativeProjection ? paxVisualScreenRect(rect) : rect
    }

    override func becomeFirstResponder() -> Bool {
        let accepted = super.becomeFirstResponder()
        if accepted { paxUpdateProjectedFocus(self, focused: true) }
        return accepted
    }

    override func resignFirstResponder() -> Bool {
        let accepted = super.resignFirstResponder()
        if accepted { paxUpdateProjectedFocus(self, focused: false) }
        return accepted
    }
}

class PaxProjectedTextField: NSTextField {
    override func convert(_ point: NSPoint, from view: NSView?) -> NSPoint {
        guard view == nil, !PaxNativeLayoutScope.isActive, paxHasNativeProjection else { return super.convert(point, from: view) }
        guard let logical = paxLogicalWindowPoint(point) else { return NSPoint(x: CGFloat.infinity, y: CGFloat.infinity) }
        return super.convert(logical, from: nil)
    }

    override func convert(_ size: NSSize, from view: NSView?) -> NSSize {
        PaxNativeLayoutScope.perform { super.convert(size, from: view) }
    }

    override func convert(_ rect: NSRect, from view: NSView?) -> NSRect {
        PaxNativeLayoutScope.perform { super.convert(rect, from: view) }
    }

    override func accessibilityFrame() -> NSRect {
        let rect = super.accessibilityFrame()
        return paxHasNativeProjection ? paxVisualScreenRect(rect) : rect
    }

    override func becomeFirstResponder() -> Bool {
        let accepted = super.becomeFirstResponder()
        if accepted { paxUpdateProjectedFocus(self, focused: true) }
        return accepted
    }

    override func resignFirstResponder() -> Bool {
        let accepted = super.resignFirstResponder()
        if accepted { paxUpdateProjectedFocus(self, focused: false) }
        return accepted
    }
}

class PaxProjectedImageView: NSImageView {
    override func convert(_ point: NSPoint, from view: NSView?) -> NSPoint {
        guard view == nil, !PaxNativeLayoutScope.isActive, paxHasNativeProjection else { return super.convert(point, from: view) }
        guard let logical = paxLogicalWindowPoint(point) else { return NSPoint(x: CGFloat.infinity, y: CGFloat.infinity) }
        return super.convert(logical, from: nil)
    }

    override func convert(_ size: NSSize, from view: NSView?) -> NSSize {
        PaxNativeLayoutScope.perform { super.convert(size, from: view) }
    }

    override func convert(_ rect: NSRect, from view: NSView?) -> NSRect {
        PaxNativeLayoutScope.perform { super.convert(rect, from: view) }
    }

    override func accessibilityFrame() -> NSRect {
        let rect = super.accessibilityFrame()
        return paxHasNativeProjection ? paxVisualScreenRect(rect) : rect
    }

    override func becomeFirstResponder() -> Bool {
        let accepted = super.becomeFirstResponder()
        if accepted { paxUpdateProjectedFocus(self, focused: true) }
        return accepted
    }

    override func resignFirstResponder() -> Bool {
        let accepted = super.resignFirstResponder()
        if accepted { paxUpdateProjectedFocus(self, focused: false) }
        return accepted
    }
}

#endif
