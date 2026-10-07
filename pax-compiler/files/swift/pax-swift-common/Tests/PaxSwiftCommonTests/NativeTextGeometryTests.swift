#if os(macOS)
import AppKit
import SwiftUI
import XCTest
import Messages
import FlexBuffers
@testable import Rendering

@MainActor
final class NativeTextGeometryTests: XCTestCase {
    func testNativeSceneAppliesGeometryBeforeInvalidationReturns() throws {
        _ = NSApplication.shared
        let element = TextElement.makeDefault(id: 999_999, parentFrame: nil, renderLayerId: 0)
        element.content = "Synchronous native geometry"
        element.selectable = true
        element.size_x = 240
        element.size_y = 40
        element.textStyle.font_size = 20
        TextElements.singleton.add(element: element)
        NativeSceneInvalidation.singleton.invalidate()
        defer {
            TextElements.singleton.remove(id: element.id)
            NativeSceneInvalidation.singleton.invalidate()
        }
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 600, height: 600),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let hostingView = NSHostingView(rootView: NativeRenderingLayer())
        window.contentView = hostingView
        hostingView.layoutSubtreeIfNeeded()
        let text = try XCTUnwrap(descendant(NSTextField.self, in: hostingView))
        let host = try XCTUnwrap(text.superview?.superview)
        for step in 1...20 {
            let angle = Double(step) * 0.001
            element.transform = [Float(cos(angle)), Float(sin(angle)), Float(-sin(angle)),
                                 Float(cos(angle)), Float(step) + 0.125, 40.25]
            NativeSceneInvalidation.singleton.invalidate()
            // No run-loop/layout flush: the GPU frame is submitted immediately after publish.
            XCTAssertEqual(host.frame.origin.x, CGFloat(element.transform[4]), accuracy: 0.001)
            XCTAssertEqual(host.frameRotation, angle * 180 / .pi, accuracy: 0.001)
        }
    }

    func testMaskedSnapshotPreservesUnclippedTextOverflow() throws {
        _ = NSApplication.shared
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 600, height: 600),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let root = NativeRenderingLayer.PlatformContainerView(frame: window.contentView!.bounds)
        window.contentView = root
        let host = NativeRenderingLayer.PlatformMaskedLeafView(frame: .zero)
        root.addSubview(host)
        let size = CGSize(width: 7, height: 12)
        let element = TextElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        element.content = "SCROLL"
        element.selectable = true
        element.wrap = false
        element.size_x = Float(size.width)
        element.size_y = Float(size.height)
        element.textStyle.font_size = 20
        element.textStyle.alignment = .center
        element.textStyle.alignmentMultiline = .center
        host.applyGeometry(size: size, localTransform: CGAffineTransform(translationX: 250, y: 250),
                           zIndex: 0, opacity: 1)
        func update(mask: ResolvedNativeMask?) {
            host.update(item: NativeRenderingLayer.NativeRenderItem(
                id: 1, zIndex: 0, parentFrame: nil, localTransform: .identity,
                size: size, opacity: 1, kind: .text(element), mask: mask))
        }
        update(mask: nil)
        let text = try XCTUnwrap(descendant(NSTextField.self, in: host))
        let contentFrame = text.frame
        // An occluder outside the glyphs must not crop overflow just by enabling a mask.
        let path = Path(CGRect(x: 100, y: 100, width: 5, height: 5))
        let mask = ResolvedNativeMask(signature: 123, size: size, holes: [
            ResolvedMaskHole(signature: 123, path: path, clips: [], cgPath: path.cgPath,
                             clipCGPaths: [], opacity: 1)
        ])
        update(mask: mask)
        let snapshot = try XCTUnwrap(host.layer?.sublayers?.first { $0.zPosition == 1_000 })
        XCTAssertTrue(snapshot.frame.contains(contentFrame), "snapshot \(snapshot.frame), text \(contentFrame)")
        let image = try XCTUnwrap(snapshot.contents) as! CGImage
        XCTAssertGreaterThan(image.width, Int(size.width * 2))
        let visible = try alphaInHalves(image)
        XCTAssertGreaterThan(visible.0, 1_000)
        XCTAssertGreaterThan(visible.1, 1_000)
        let leftHalf = Path(CGRect(x: -100, y: -100, width: 103.5, height: 200))
        let halfMask = ResolvedNativeMask(signature: 124, size: size, holes: [
            ResolvedMaskHole(signature: 124, path: leftHalf, clips: [], cgPath: leftHalf.cgPath,
                             clipCGPaths: [], opacity: 1)
        ])
        update(mask: halfMask)
        let halfVisible = try alphaInHalves(try XCTUnwrap(snapshot.contents) as! CGImage)
        XCTAssertLessThan(halfVisible.0, visible.0 / 10)
        XCTAssertGreaterThan(halfVisible.1, visible.1 * 9 / 10)
        element.textStyle.fill = .red
        update(mask: halfMask)
        let recolored = try alphaInHalves(try XCTUnwrap(snapshot.contents) as! CGImage)
        XCTAssertLessThan(recolored.0, visible.0 / 10)
        update(mask: nil)
        XCTAssertEqual(text.superview?.alphaValue, 1)
        XCTAssertNil(snapshot.superlayer)
        element.selectable = false
        update(mask: mask)
        let staticPixels = try alphaInHalves(try XCTUnwrap(snapshot.contents) as! CGImage)
        XCTAssertGreaterThan(staticPixels.0, 1_000)
        XCTAssertGreaterThan(staticPixels.1, 1_000)
        element.clip = true
        update(mask: mask)
        XCTAssertEqual(snapshot.frame, CGRect(origin: .zero, size: size))
    }

    func testFineGrainedRotationPreservesTextGeometry() throws {
        _ = NSApplication.shared
        let window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 600, height: 600),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let root = NativeRenderingLayer.PlatformContainerView(frame: window.contentView!.bounds)
        window.contentView = root
        let host = NativeRenderingLayer.PlatformContainerView(frame: .zero)
        root.addSubview(host)
        let size = CGSize(width: 7.03, height: 12.26)
        let leaf = PaxNativeTextLeafView(frame: CGRect(origin: .zero, size: size))
        host.addSubview(leaf)
        let element = TextElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        element.content = "SCROLL"
        element.selectable = true
        element.wrap = false
        element.size_x = Float(size.width)
        element.size_y = Float(size.height)
        element.textStyle.font_size = 20
        element.textStyle.alignment = .center
        element.textStyle.alignmentMultiline = .center
        leaf.apply(element: element, size: size)
        let text = try XCTUnwrap(descendant(NSTextField.self, in: leaf))
        for step in 0..<3600 {
            let angle = CGFloat(step) * 0.1
            let transform = CGAffineTransform(translationX: 250.13 + angle * 0.07, y: 250.27)
                .rotated(by: angle * .pi / 180).scaledBy(x: 1.25, y: 0.75)
            host.applyGeometry(size: size, localTransform: transform, zIndex: 0, opacity: 1)
            root.layoutSubtreeIfNeeded()
            window.displayIfNeeded()
            for point in [CGPoint.zero, CGPoint(x: 30, y: 10)] {
                let local = CGPoint(x: text.frame.minX + point.x, y: text.frame.minY + point.y)
                let expected = local.applying(transform)
                let actual = text.convert(point, to: root)
                XCTAssertEqual(actual.x, expected.x, accuracy: 0.001, "angle \(angle)")
                XCTAssertEqual(actual.y, expected.y, accuracy: 0.001, "angle \(angle)")
                let layerPoint = try XCTUnwrap(text.layer).convert(point, to: root.layer)
                XCTAssertEqual(layerPoint.x, expected.x, accuracy: 0.001, "layer angle \(angle)")
                XCTAssertEqual(layerPoint.y, expected.y, accuracy: 0.001, "layer angle \(angle)")
            }
        }
    }

    func testRotationDoesNotChangeWrappedTextLayout() throws {
        _ = NSApplication.shared
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 800, height: 600),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let root = NativeRenderingLayer.PlatformContainerView(frame: window.contentView!.bounds)
        window.contentView = root
        let host = NativeRenderingLayer.PlatformContainerView(frame: .zero)
        root.addSubview(host)
        let size = CGSize(width: 7.03, height: 12.26)
        let element = TextElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        element.content = "SCROLL"
        element.selectable = true
        element.markdown = true
        element.size_x = Float(size.width)
        element.size_y = Float(size.height)
        element.textStyle.font_size = 20
        element.textStyle.alignment = .center
        element.textStyle.alignmentMultiline = .center
        let leaf = PaxNativeTextLeafView(frame: CGRect(origin: .zero, size: size))
        host.addSubview(leaf)
        host.applyGeometry(size: size, localTransform: CGAffineTransform(translationX: 100, y: 100),
                           zIndex: 0, opacity: 1)
        leaf.apply(element: element, size: size)
        let text = try XCTUnwrap(descendant(NSTextView.self, in: leaf))
        let textContainer = try XCTUnwrap(text.textContainer)
        let layout = try XCTUnwrap(text.layoutManager)
        layout.ensureLayout(for: textContainer)
        let originalWidth = textContainer.containerSize.width
        let originalGlyphBounds = layout.usedRect(for: textContainer)
        let originalOrigin = text.convert(CGPoint.zero, to: leaf)
        XCTAssertGreaterThan(originalGlyphBounds.height, 100)
        for angle in [1.0, 30, 60, 90, 135, 180, 270, 330, 0] {
            let transform = CGAffineTransform(translationX: 100, y: 100)
                .rotated(by: angle * .pi / 180)
                .scaledBy(x: 1.25, y: 0.75)
            host.applyGeometry(size: size, localTransform: transform, zIndex: 0, opacity: 1)
            root.layoutSubtreeIfNeeded()
            window.displayIfNeeded()
            layout.ensureLayout(for: textContainer)
            XCTAssertEqual(textContainer.containerSize.width, originalWidth, accuracy: 0.01, "angle \(angle)")
            XCTAssertEqual(layout.usedRect(for: textContainer).height, originalGlyphBounds.height,
                           accuracy: 0.01, "angle \(angle)")
            // NSScrollView can align its viewport to backing pixels; the glyph
            // origin must stay within one backing pixel and must not follow the
            // enlarged viewport's center. Its wrapping constraint remains exact.
            let origin = text.convert(CGPoint.zero, to: leaf)
            let tolerance = 1 / window.backingScaleFactor
            XCTAssertEqual(origin.x, originalOrigin.x, accuracy: tolerance, "angle \(angle)")
            XCTAssertEqual(origin.y, originalOrigin.y, accuracy: tolerance, "angle \(angle)")
        }

        // A real layout-width change must still reflow the paragraph.
        let widerSize = CGSize(width: 200, height: size.height)
        element.size_x = Float(widerSize.width)
        leaf.apply(element: element, size: widerSize)
        layout.ensureLayout(for: textContainer)
        XCTAssertEqual(textContainer.containerSize.width, widerSize.width, accuracy: 0.01)
        XCTAssertLessThan(layout.usedRect(for: textContainer).height, originalGlyphBounds.height)
    }

    func testUnclippedSingleLineOverflowKeepsItsAlignment() throws {
        _ = NSApplication.shared
        let size = CGSize(width: 7, height: 12)
        for selectable in [false, true] {
            for alignment in [Alignment.leading, .center, .trailing] {
                let element = TextElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
                element.content = "SCROLL"
                element.size_x = Float(size.width)
                element.size_y = Float(size.height)
                element.selectable = selectable
                element.wrap = false
                element.textStyle.font_size = 20
                element.textStyle.alignment = alignment
                element.textStyle.alignmentMultiline = alignment == .center ? .center :
                    (alignment == .trailing ? .trailing : .leading)
                let leaf = PaxNativeTextLeafView(frame: CGRect(origin: .zero, size: size))
                leaf.apply(element: element, size: size)
                let contentFrame: CGRect
                if selectable {
                    let text = try XCTUnwrap(descendant(NSTextField.self, in: leaf))
                    contentFrame = text.alignmentRect(forFrame: text.frame)
                    XCTAssertLessThan(contentFrame.height, 30)
                } else {
                    contentFrame = try XCTUnwrap(leaf.layer?.sublayers?.compactMap { $0 as? CATextLayer }.first).frame
                }
                XCTAssertGreaterThan(contentFrame.width, size.width)
                XCTAssertEqual(contentFrame.midY, size.height * 0.5, accuracy: 0.01)
                if alignment == .center {
                    XCTAssertEqual(contentFrame.midX, size.width * 0.5, accuracy: 0.01)
                } else if alignment == .trailing {
                    XCTAssertEqual(contentFrame.maxX, size.width, accuracy: 0.01)
                } else {
                    XCTAssertEqual(contentFrame.minX, 0, accuracy: 0.01)
                }
                element.clip = true
                leaf.apply(element: element, size: size)
                XCTAssertEqual(leaf.layer?.masksToBounds, true)
                if selectable {
                    let text = try XCTUnwrap(descendant(NSTextField.self, in: leaf))
                    XCTAssertEqual(text.alignmentRect(forFrame: text.frame).width,
                                   size.width, accuracy: 0.01)
                }
            }
        }
    }

    func testSelectableAndEditableTextRetainNativeInteraction() throws {
        _ = NSApplication.shared
        let size = CGSize(width: 240, height: 60)
        let window = NSWindow(contentRect: CGRect(origin: .zero, size: size),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let element = TextElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        element.content = "Select or edit this text"
        element.selectable = true
        element.markdown = true
        element.textStyle.font_size = 20
        element.size_x = Float(size.width)
        element.size_y = Float(size.height)
        let leaf = PaxNativeTextLeafView(frame: CGRect(origin: .zero, size: size))
        let root = NativeRenderingLayer.PlatformContainerView(frame: CGRect(origin: .zero, size: size))
        window.contentView = root
        root.addSubview(leaf)
        for editable in [false, true] {
            element.editable = editable
            leaf.apply(element: element, size: size)
            XCTAssertEqual(NativeRenderingLayer.NativeLeafKind.text(element).contentKey,
                           "text")
            let text = try XCTUnwrap(descendant(NSTextView.self, in: leaf))
            XCTAssertTrue(text.isSelectable)
            XCTAssertEqual(text.isEditable, editable)
            XCTAssertTrue(leaf.hitTest(text.convert(CGPoint(x: 10, y: 10), to: root)) === text)
            let selection = NSRange(location: 2, length: 4)
            text.setSelectedRange(selection)
            leaf.apply(element: element, size: size)
            XCTAssertEqual(text.selectedRange(), selection)
            if editable {
                var sentData: Data?
                let previousSender = NativeInterruptDispatcher.shared.sendData
                NativeInterruptDispatcher.shared.sendData = { sentData = $0 }
                defer { NativeInterruptDispatcher.shared.sendData = previousSender }
                text.insertText("new", replacementRange: NSRange(location: 0, length: 6))
                XCTAssertEqual(text.string, "new or edit this text")
                let message = try XCTUnwrap(FlexBuffer.decode(data: XCTUnwrap(sentData)))
                XCTAssertEqual(message["TextInput"]?["id"]?.asUInt64, 1)
                XCTAssertEqual(message["TextInput"]?["text"]?.asString, text.string)
            }
        }
        element.editable = false
        element.selectable = false
        leaf.apply(element: element, size: size)
        XCTAssertEqual(NativeRenderingLayer.NativeLeafKind.text(element).contentKey, "text")
        XCTAssertNil(leaf.hitTest(CGPoint(x: 10, y: 10)))
    }

    private func descendant<T: NSView>(_ type: T.Type, in view: NSView) -> T? {
        if let match = view as? T { return match }
        return view.subviews.lazy.compactMap { self.descendant(type, in: $0) }.first
    }

    private func alphaInHalves(_ image: CGImage) throws -> (Int, Int) {
        let context = try XCTUnwrap(CGContext(data: nil, width: image.width, height: image.height,
            bitsPerComponent: 8, bytesPerRow: image.width * 4, space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue))
        context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
        let data = try XCTUnwrap(context.data).assumingMemoryBound(to: UInt8.self)
        var left = 0
        var right = 0
        for y in 0..<image.height {
            for x in 0..<image.width {
                let alpha = Int(data[(y * image.width + x) * 4 + 3])
                if x < image.width / 2 { left += alpha } else { right += alpha }
            }
        }
        return (left, right)
    }
}
#endif
