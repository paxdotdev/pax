import XCTest
import SwiftUI
@testable import Messages
import FlexBuffers
@testable import Rendering

#if os(macOS)
final class NativeTextTests: XCTestCase {
    @MainActor
    private func fixture(_ body: (NSHostingView<NativeRenderingLayer>, NSWindow, TextElement, NSTextField) throws -> Void) throws {
        PaxNativeHostState.reset()
        defer { PaxNativeHostState.reset() }
        NativeCullingState.shared.setAccessibilityActive(false)
        let model = TextElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        model.content = "A little anticipation. More words for wrapping."
        model.selectable = true
        model.size_x = 180
        model.size_y = 35
        model.textStyle.font_size = 24
        TextElements.singleton.add(element: model)
        NativeSceneInvalidation.singleton.invalidate()
        let host = NSHostingView(rootView: NativeRenderingLayer())
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 450, height: 450),
            styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = host
        defer { window.close() }
        flush(host)
        let field = try XCTUnwrap(descendants(host).compactMap { $0 as? NSTextField }.first)
        try body(host, window, model, field)
    }
    @MainActor
    private func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
    @MainActor
    private func flush(_ host: NSView) {
        host.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.04))
        host.layoutSubtreeIfNeeded()
    }
    @MainActor
    private func update(_ host: NSView) {
        NativeSceneInvalidation.singleton.invalidate(ids: [1], rebuild: false)
        flush(host)
    }

    @MainActor
    func testOrdinaryLabelHasNoPrivateScrollerOrDocumentTextView() throws {
        try fixture { host, _, model, field in
            XCTAssertTrue(field.isSelectable)
            XCTAssertFalse(field.isEditable)
            XCTAssertEqual(field.stringValue, model.content)
            XCTAssertEqual(descendants(host).filter { $0 is NSScrollView }.count, 0)
            XCTAssertEqual(descendants(host).filter { $0 is NSTextView }.count, 0)
            XCTAssertGreaterThan(field.frame.height, CGFloat(model.size_y))
            XCTAssertFalse(field.layer!.masksToBounds)
            model.clip = true
            update(host)
            let clipped = try XCTUnwrap(descendants(host).compactMap { $0 as? NSTextField }.first)
            XCTAssertLessThanOrEqual(clipped.frame.height, CGFloat(model.size_y))
            XCTAssertFalse(clipped.cell!.truncatesLastVisibleLine)
        }
    }

    @MainActor
    func testWidthChangesRemeasureAndUnwrappedTextOverflows() throws {
        try fixture { host, _, model, field in
            let firstHeight = field.frame.height
            model.size_x = 350
            update(host)
            XCTAssertLessThan(field.frame.height, firstHeight)
            model.wrap = false
            model.size_x = 40
            update(host)
            XCTAssertGreaterThan(field.frame.width, 40)
            XCTAssertLessThan(field.frame.height, firstHeight)
        }
    }

    @MainActor
    func testSelectionSurvivesScalingAndPinsColdLeaf() throws {
        try fixture { host, window, model, field in
            model.size_y = 200
            update(host)
            field.selectText(nil)
            let editor = try XCTUnwrap(field.currentEditor() as? NSTextView)
            let displayedFont = try XCTUnwrap(field.attributedStringValue.attribute(.font, at: 0, effectiveRange: nil) as? NSFont)
            let selectedFont = try XCTUnwrap(editor.textStorage?.attribute(.font, at: 0, effectiveRange: nil) as? NSFont)
            XCTAssertEqual(selectedFont, displayedFont)
            XCTAssertEqual(selectedFont.pointSize, model.textStyle.font_size)
            XCTAssertFalse(editor.isEditable)
            editor.setSelectedRange(NSRange(location: 2, length: 5))
            let size = field.frame.size
            for scale: Float in [0.92, 1.04, 0.98, 1] {
                model.transform[0] = scale
                model.transform[3] = scale
                update(host)
                XCTAssertTrue(field.currentEditor() === editor)
                XCTAssertEqual(editor.selectedRange(), NSRange(location: 2, length: 5))
                XCTAssertEqual(field.frame.size, size)
            }
            NativeCullingState.shared.apply(cull: [1], restore: [])
            update(host)
            XCTAssertNotNil(field.window)
            XCTAssertTrue(window.makeFirstResponder(nil))
            update(host)
            XCTAssertNil(field.window)
            NativeCullingState.shared.apply(cull: [], restore: [1])
            update(host)
            XCTAssertTrue(descendants(host).contains { $0 === field })
        }
    }

    @MainActor
    func testContentUpdateWhileSelectedReachesTheFieldEditor() throws {
        try fixture { host, _, model, field in
            field.selectText(nil)
            let editor = try XCTUnwrap(field.currentEditor() as? NSTextView)
            let displayedFont = try XCTUnwrap(field.attributedStringValue.attribute(.font, at: 0, effectiveRange: nil) as? NSFont)
            let selectedFont = try XCTUnwrap(editor.textStorage?.attribute(.font, at: 0, effectiveRange: nil) as? NSFont)
            XCTAssertEqual(selectedFont, displayedFont)
            XCTAssertEqual(selectedFont.pointSize, model.textStyle.font_size)
            XCTAssertFalse(editor.isEditable)
            editor.setSelectedRange(NSRange(location: 2, length: 5))
            model.content = "Changed text while selected"
            update(host)
            XCTAssertEqual(field.stringValue, model.content)
            XCTAssertEqual(editor.string, model.content)
            XCTAssertEqual(editor.selectedRange(), NSRange(location: 2, length: 5))
        }
    }

    @MainActor
    func testFieldAndExistingTextViewPaintTheSamePlainTextBounds() throws {
        try fixture { host, _, model, field in
            model.content = "A little anticipation. More words for wrapping."
            update(host)
            let rich = TextElement.makeDefault(id: 2, parentFrame: nil, renderLayerId: 0)
            rich.content = model.content
            rich.selectable = true
            rich.markdown = true
            rich.size_x = model.size_x
            rich.size_y = model.size_y
            rich.textStyle = model.textStyle
            TextElements.singleton.add(element: rich)
            NativeSceneInvalidation.singleton.invalidate()
            flush(host)
            let document = try XCTUnwrap(descendants(host).compactMap { $0 as? NSTextView }.first)
            XCTAssertEqual(field.alignmentRect(forFrame: field.frame).size, document.frame.size)
            @MainActor func inkBounds(_ view: NSView) throws -> CGRect {
                let rep = try XCTUnwrap(view.bitmapImageRepForCachingDisplay(in: view.bounds))
                view.cacheDisplay(in: view.bounds, to: rep)
                var x0 = rep.pixelsWide, y0 = rep.pixelsHigh, x1 = -1, y1 = -1
                for y in 0..<rep.pixelsHigh {
                    for x in 0..<rep.pixelsWide {
                        if let pixel = rep.colorAt(x: x, y: y), pixel.alphaComponent > 0.05 {
                            x0 = min(x0, x); y0 = min(y0, y); x1 = max(x1, x); y1 = max(y1, y)
                        }
                    }
                }
                XCTAssertGreaterThan(x1, x0)
                let scale = CGFloat(rep.pixelsWide) / view.bounds.width
                return CGRect(x: CGFloat(x0)/scale, y: CGFloat(y0)/scale,
                    width: CGFloat(x1-x0+1)/scale, height: CGFloat(y1-y0+1)/scale)
            }
            let labelInk = try inkBounds(field), documentInk = try inkBounds(document)
            XCTAssertEqual(labelInk.minX + field.frame.minX, documentInk.minX, accuracy: 0.5)
            XCTAssertEqual(labelInk.minY, documentInk.minY, accuracy: 0.5)
            XCTAssertEqual(labelInk.width, documentInk.width, accuracy: 0.5)
            XCTAssertEqual(labelInk.height, documentInk.height, accuracy: 0.5)
        }
    }

    @MainActor
    func testNativeLabelActuallyPaintsMultipleLines() throws {
        try fixture { host, _, model, field in
            model.clip = false
            update(host)
            let rep = try XCTUnwrap(field.bitmapImageRepForCachingDisplay(in: field.bounds))
            field.cacheDisplay(in: field.bounds, to: rep)
            var rowsWithInk = [Int]()
            for y in 0..<rep.pixelsHigh {
                var ink = false
                for x in 0..<rep.pixelsWide {
                    if let pixel = rep.colorAt(x: x, y: y), pixel.alphaComponent > 0.05 { ink = true; break }
                }
                if ink { rowsWithInk.append(y) }
            }
            XCTAssertGreaterThan(rowsWithInk.count, 30)
            XCTAssertGreaterThan(try XCTUnwrap(rowsWithInk.last) - (try XCTUnwrap(rowsWithInk.first)), 40)
        }
    }
    @MainActor
    func testRepresentationChangesKeepHostFocusAndSelection() throws {
        try fixture { host, window, model, field in
            let leaf = try XCTUnwrap(field.superview as? PaxNativeTextLeafView)
            let outer = try XCTUnwrap(leaf.superview)
            field.selectText(nil)
            let selected = NSRange(location: 2, length: 5)
            (field.currentEditor() as? NSTextView)?.setSelectedRange(selected)
            model.editable = true
            update(host)
            let document = try XCTUnwrap(descendants(leaf).compactMap { $0 as? NSTextView }.first)
            XCTAssertTrue(leaf.superview === outer)
            XCTAssertNil(field.superview)
            XCTAssertTrue(document.isEditable)
            XCTAssertTrue(window.firstResponder === document)
            XCTAssertEqual(document.selectedRange(), selected)
            // Turning editing off preserves an active selection before an optional downgrade.
            model.editable = false
            update(host)
            XCTAssertFalse(document.isEditable)
            XCTAssertTrue(window.firstResponder === document)
            XCTAssertEqual(document.selectedRange(), selected)
            document.setSelectedRange(NSRange(location: 0, length: 0))
            XCTAssertTrue(window.makeFirstResponder(nil))
            model.content = "A shorter label"
            update(host)
            XCTAssertTrue(leaf.superview === outer)
            XCTAssertEqual(descendants(leaf).filter { $0 is NSTextField }.count, 1)
            XCTAssertEqual(descendants(leaf).filter { $0 is NSScrollView || $0 is NSTextView }.count, 0)
            model.selectable = false
            update(host)
            XCTAssertTrue(leaf.subviews.isEmpty)
            XCTAssertNil(leaf.hitTest(.zero))
            XCTAssertEqual(leaf.accessibilityRole(), .staticText)
            XCTAssertEqual(leaf.accessibilityValue() as? String, model.content)
            let staticLayer = try XCTUnwrap(leaf.layer?.sublayers?.compactMap { $0 as? PaxImmediateTextLayer }.first)
            XCTAssertNil(staticLayer.action(forKey: "contents"))
            model.selectable = true
            model.markdown = true
            model.content = "A [link](https://pax.dev)"
            update(host)
            let rich = try XCTUnwrap(descendants(leaf).compactMap { $0 as? NSTextView }.first)
            XCTAssertNotNil(rich.textStorage?.attribute(.link, at: 3, effectiveRange: nil))
            XCTAssertFalse(rich.isEditable)
            XCTAssertTrue(rich.isSelectable)
            XCTAssertTrue(leaf.superview === outer)
        }
    }

    @MainActor
    func testReadOnlyDocumentForwardsWheelAndEditableDocumentDispatchesInput() throws {
        try fixture { host, _, model, _ in
            model.markdown = true
            update(host)
            let document = try XCTUnwrap(descendants(host).compactMap { $0 as? NSTextView }.first)
            let scroll = try XCTUnwrap(document.enclosingScrollView)
            let recorder = WheelRecorder()
            let next = scroll.nextResponder
            scroll.nextResponder = recorder
            defer { scroll.nextResponder = next }
            let event = try XCTUnwrap(NSEvent.otherEvent(with: .applicationDefined, location: .zero,
                modifierFlags: [], timestamp: 0, windowNumber: 0, context: nil,
                subtype: 0, data1: 0, data2: 0))
            // Exercise forwarding directly, without synthesizing system input.
            scroll.scrollWheel(with: event)
            XCTAssertTrue(recorder.received === event)
            model.editable = true
            update(host)
            var data: Data?
            let previous = NativeInterruptDispatcher.shared.sendData
            NativeInterruptDispatcher.shared.sendData = { data = $0 }
            defer { NativeInterruptDispatcher.shared.sendData = previous }
            document.string = "Edited text"
            document.didChangeText()
            let root = try XCTUnwrap(FlexBuffer.decode(data: try XCTUnwrap(data)))
            XCTAssertEqual(root["TextInput"]?["id"]?.asUInt64, UInt64(model.id))
            XCTAssertEqual(root["TextInput"]?["text"]?.asString, "Edited text")
        }
    }

    @MainActor
    func testEmptyEditorKeepsItsViewportDuringMarkedTextComposition() throws {
        try fixture { host, window, model, _ in
            model.content = ""
            model.editable = true
            model.clip = true
            model.size_y = 120
            update(host)
            let editor = try XCTUnwrap(descendants(host).compactMap { $0 as? NSTextView }.first)
            let scroll = try XCTUnwrap(editor.enclosingScrollView)
            XCTAssertEqual(scroll.frame.height, 120)
            XCTAssertEqual(editor.frame.height, 120)
            XCTAssertTrue(window.makeFirstResponder(editor))
            editor.setMarkedText("とうきょう", selectedRange: NSRange(location: 5, length: 0),
                replacementRange: NSRange(location: NSNotFound, length: 0))
            XCTAssertTrue(editor.hasMarkedText())
            // Native preedit has not reached the committed model. A layout update
            // must leave it intact and keep the editor's declared area available.
            XCTAssertEqual(model.content, "")
            model.size_y = 80
            update(host)
            XCTAssertTrue(editor.hasMarkedText())
            XCTAssertEqual(editor.string, "とうきょう")
            XCTAssertEqual(scroll.frame.height, 80)
            XCTAssertEqual(editor.frame.height, 80)
            editor.unmarkText()
        }
    }

    @MainActor
    func testStyleAlignmentUnicodeAndFontNotificationInvalidateCachedContent() throws {
        try fixture { host, _, model, field in
            model.content = "Café 🦊\nمرحبا بالعالم"
            model.textStyle.font = PaxFont(type: .system(PaxFont.SystemFont(family: "Helvetica", style: .italic, weight: .bold)))
            model.textStyle.underline = true
            model.textStyle.fill = .red
            model.size_y = 200
            update(host)
            let before = field.alignmentRect(forFrame: field.frame)
            let attributes = field.attributedStringValue.attributes(at: 0, effectiveRange: nil)
            XCTAssertEqual(field.stringValue, model.content)
            XCTAssertEqual(attributes[.underlineStyle] as? Int, NSUnderlineStyle.single.rawValue)
            XCTAssertEqual(attributes[.font] as? NSFont, model.textStyle.font.getNSFont(size: 24))
            XCTAssertEqual(field.textColor, NSColor(Color.red))
            model.textStyle.alignment = .bottomTrailing
            model.textStyle.alignmentMultiline = .trailing
            update(host)
            XCTAssertEqual(field.alignment, .right)
            XCTAssertGreaterThan(field.alignmentRect(forFrame: field.frame).minY, before.minY)
            model.size_y = -1
            update(host)
            let lastSize = try XCTUnwrap(model.lastMeasuredSize)
            // Simulate the font loader replacing a resolved fallback before broadcasting.
            let resolved = try XCTUnwrap(NSFont(name: "Courier", size: 24))
            model.textStyle.font.cachedNSFont = resolved
            NotificationCenter.default.post(name: .paxFontRegistered, object: nil)
            flush(host)
            XCTAssertEqual(field.font, resolved)
            XCTAssertNotEqual(model.lastMeasuredSize, lastSize)
        }
    }
}

private final class WheelRecorder: NSResponder {
    var received: NSEvent?
    override func scrollWheel(with event: NSEvent) { received = event }
}
#endif
