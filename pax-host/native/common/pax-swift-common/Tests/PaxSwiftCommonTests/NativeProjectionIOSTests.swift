#if os(iOS)
import UIKit
import SwiftUI
import XCTest
import Messages
@testable import Rendering

@MainActor
final class NativeProjectionIOSTests: XCTestCase {
    private let size = CGSize(width: 240, height: 160)

    private func projection(_ angle: CGFloat, x: CGFloat = 170, y: CGFloat = 180) throws -> PaxNativeProjection {
        let radians = angle * .pi / 180
        let c: CGFloat = abs(angle) == 90 ? 0 : cos(radians)
        let s = sin(radians) / 700
        // A centered card viewed from 700 points away. Both faces remain live;
        // only the exactly edge-on plane loses its inverse.
        return try XCTUnwrap(PaxNativeProjection([
            Float(c + x*s), Float(y*s), 0, Float(s),
            0, 1, 0, 0, 0, 0, 1, 0,
            Float(x - size.width/2 * (c + x*s)),
            Float(y - size.height/2 - size.width/2*y*s), 0,
            Float(1 - size.width/2*s)
        ]))
    }

    private func assertPoint(_ actual: CGPoint, _ expected: CGPoint,
                             file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertEqual(actual.x, expected.x, accuracy: 0.001, file: file, line: line)
        XCTAssertEqual(actual.y, expected.y, accuracy: 0.001, file: file, line: line)
    }

    func testUIKitConversionsAndHitTestingFollowThePlaneInBothDirections() throws {
        let root = UIView(frame: CGRect(x: 0, y: 0, width: 500, height: 600))
        let host = NativeRenderingLayer.PlatformContainerView(frame: .zero)
        root.addSubview(host)
        let button = UIButton(frame: CGRect(x: 30, y: 35, width: 120, height: 50))
        button.setTitle("Live lesson", for: .normal)
        host.addSubview(button)
        for angle: CGFloat in [0, 30, 70, 89, 91, 120, 180, -30, -70, -89, -91, -120, -180, 0] {
            let p = try projection(angle)
            host.applyGeometry(size: size, localTransform: .identity, projection: p, zIndex: 4, opacity: 1)
            let local = CGPoint(x: 80, y: 60)
            let screen = try XCTUnwrap(p.project(local))
            assertPoint(host.convert(local, to: root), screen)
            assertPoint(host.convert(screen, from: root), local)
            XCTAssertTrue(root.hitTest(screen, with: nil) === button, "angle \(angle)")
            XCTAssertEqual(host.bounds.size, size)
            XCTAssertTrue(button.superview === host)
        }
    }

    func testAffineProjectiveTransitionsDoNotAccumulateFrameOrLayoutChanges() throws {
        let root = UIView(frame: CGRect(x: 0, y: 0, width: 500, height: 600))
        let host = NativeRenderingLayer.PlatformContainerView(frame: .zero)
        root.addSubview(host)
        let affine = CGAffineTransform(a: 0.8, b: 0.3, c: -0.2, d: 1.1, tx: 70, ty: 110)
        for _ in 0..<4 {
            host.applyGeometry(size: size, localTransform: .identity,
                               projection: try projection(65), zIndex: 1, opacity: 1)
            for t in [affine, affine.translatedBy(x: 30, y: 40), .identity] {
                host.applyGeometry(size: size, localTransform: t, zIndex: 1, opacity: 1)
                let point = CGPoint(x: 90, y: 100)
                assertPoint(host.convert(point, to: root), point.applying(t))
                XCTAssertEqual(host.bounds, CGRect(origin: .zero, size: size))
            }
        }
    }

    func testProjectedClipRejectsTouchesOutsideItsAperture() throws {
        let root = UIView(frame: CGRect(x: 0, y: 0, width: 500, height: 600))
        let host = NativeRenderingLayer.PlatformContainerView(frame: .zero)
        root.addSubview(host)
        let button = UIButton(frame: CGRect(origin: .zero, size: size))
        host.addSubview(button)
        let p = try projection(55)
        host.applyGeometry(size: size, localTransform: .identity, projection: p, zIndex: 1, opacity: 1)
        let aperture = CGPath(ellipseIn: CGRect(x: 50, y: 40, width: 130, height: 100), transform: nil)
        host.applyClip(path: aperture, signature: 1, borderRadius: 0, clipContent: true)
        XCTAssertTrue(root.hitTest(try XCTUnwrap(p.project(CGPoint(x: 100, y: 80))), with: nil) === button)
        XCTAssertFalse(root.hitTest(try XCTUnwrap(p.project(CGPoint(x: 20, y: 20))), with: nil) === button)
        XCTAssertEqual((host.layer.mask as? CAShapeLayer)?.path, aperture)
    }

    private func coefficients(_ p: PaxNativeProjection) -> [Float] {
        let m = p.m.map(Float.init)
        return [m[0],m[1],0,m[2], m[3],m[4],0,m[5], 0,0,1,0, m[6],m[7],0,m[8]]
    }

    private func descendants(_ view: UIView) -> [UIView] { [view] + view.subviews.flatMap(descendants) }

    private func flush(_ view: UIView) {
        view.layoutIfNeeded()
        RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.04))
        view.layoutIfNeeded()
    }

    func testRetainedScenePreservesEditorMasksAndScreenGeometryThroughViewportChanges() throws {
        PaxNativeHostState.reset()
        defer { setResolvedNativeMask(id: 1, mask: nil); PaxNativeHostState.reset() }
        let model = TextboxElement.makeDefault(id: 1, parentFrame: nil, renderLayerId: 0)
        model.size_x = 240; model.size_y = 42
        model.text = "Editing survives a turn"
        model.transform = coefficients(try projection(35))
        TextboxElements.singleton.add(element: model)
        let hole = Path(CGRect(x: 0, y: 0, width: 45, height: 42))
        setResolvedNativeMask(id: 1, mask: ResolvedNativeMask(signature: 1051,
            size: CGSize(width: 240, height: 42), holes: [ResolvedMaskHole(signature: 1051,
                path: hole, clips: [], cgPath: hole.cgPath, clipCGPaths: [], opacity: 1)]))
        NativeSceneInvalidation.singleton.invalidate()
        let scene = UIHostingController(rootView: NativeRenderingLayer())
        let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 500, height: 700))
        window.rootViewController = scene
        window.makeKeyAndVisible()
        defer { window.isHidden = true; window.rootViewController = nil }
        flush(scene.view)
        let field = try XCTUnwrap(descendants(scene.view).compactMap { $0 as? UITextField }.first)
        let leaf = try XCTUnwrap(field.superview as? NativeRenderingLayer.PlatformMaskedLeafView)
        XCTAssertTrue(field.becomeFirstResponder())
        let start = try XCTUnwrap(field.position(from: field.beginningOfDocument, offset: 3))
        let end = try XCTUnwrap(field.position(from: start, offset: 5))
        field.selectedTextRange = field.textRange(from: start, to: end)
        let parent = try XCTUnwrap(leaf.superview)
        for (index, angle) in [CGFloat(60), -60, 89, -89, 110, -110, 0].enumerated() {
            let p = try projection(angle, x: 210, y: index % 2 == 0 ? 170 : 230)
            model.transform = coefficients(p)
            window.bounds.size.height = index % 2 == 0 ? 440 : 700
            NativeSceneInvalidation.singleton.invalidate(ids: [1], rebuild: false)
            flush(scene.view)
            XCTAssertTrue(descendants(scene.view).contains { $0 === field })
            XCTAssertTrue(field.isFirstResponder)
            let selection = try XCTUnwrap(field.selectedTextRange)
            XCTAssertEqual(field.offset(from: field.beginningOfDocument, to: selection.start), 3)
            XCTAssertEqual(field.offset(from: selection.start, to: selection.end), 5)
            XCTAssertEqual(field.text, "Editing survives a turn")
            XCTAssertNotNil(leaf.layer.mask)
            let covered = try XCTUnwrap(p.project(CGPoint(x: 20, y: 20)))
            let visible = try XCTUnwrap(p.project(CGPoint(x: 160, y: 20)))
            XCTAssertNil(leaf.hitTest(leaf.convert(covered, from: parent), with: nil))
            XCTAssertNotNil(leaf.hitTest(leaf.convert(visible, from: parent), with: nil))
            let expected = parent.convert(try XCTUnwrap(p.bounds(of: field.frame)), to: window)
            let screen = field.convert(field.bounds, to: window)
            XCTAssertEqual(screen.minX, expected.minX, accuracy: 0.01)
            XCTAssertEqual(screen.minY, expected.minY, accuracy: 0.01)
            XCTAssertEqual(screen.width, expected.width, accuracy: 0.01)
            XCTAssertEqual(screen.height, expected.height, accuracy: 0.01)
        }
        field.resignFirstResponder()
    }

    func testCollapsedAndClippedPlanesRejectInputAndRecoverWithoutReplacingContent() throws {
        let root = UIView(frame: CGRect(x: 0, y: 0, width: 500, height: 600))
        let host = NativeRenderingLayer.PlatformContainerView(frame: .zero)
        root.addSubview(host)
        let field = UITextField(frame: CGRect(origin: .zero, size: size))
        field.text = "Retained near the camera"
        host.addSubview(field)
        let p = try projection(45)
        let visible = CGPoint(x: 100, y: 60)
        let outsideClip = CGPoint(x: 20, y: 60)
        for _ in 0..<3 {
            host.applyGeometry(size: size, localTransform: .identity, projection: .collapsed, zIndex: 0, opacity: 1)
            XCTAssertNil(host.hitTest(visible, with: nil))
            // Engine near/far clipping arrives as a local aperture. It can be
            // partial or empty while the native control remains mounted.
            host.applyGeometry(size: size, localTransform: .identity, projection: p, zIndex: 0, opacity: 1)
            host.applyClip(path: CGPath(rect: CGRect(x: 80, y: 0, width: 160, height: 160), transform: nil),
                           signature: 1, borderRadius: 0, clipContent: true)
            XCTAssertTrue(root.hitTest(try XCTUnwrap(p.project(visible)), with: nil) === field)
            XCTAssertFalse(root.hitTest(try XCTUnwrap(p.project(outsideClip)), with: nil) === field)
            host.applyClip(path: CGMutablePath(), signature: 2, borderRadius: 0, clipContent: true)
            XCTAssertNil(host.hitTest(visible, with: nil))
            host.applyClip(path: nil, signature: 3, borderRadius: 0, clipContent: false)
            XCTAssertTrue(root.hitTest(try XCTUnwrap(p.project(outsideClip)), with: nil) === field)
            XCTAssertEqual(field.text, "Retained near the camera")
            XCTAssertTrue(field.superview === host)
            XCTAssertEqual(host.bounds.size, size)
        }
        let behindCamera = try XCTUnwrap(PaxNativeProjection(
            [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,-1] as [Float]))
        host.applyGeometry(size: size, localTransform: .identity, projection: behindCamera, zIndex: 0, opacity: 1)
        XCTAssertNil(host.hitTest(visible, with: nil))
    }

    func testNestedFrameScrollerAndRichTextKeepTheirLocalCoordinates() throws {
        PaxNativeHostState.reset()
        defer { setResolvedNativeMask(id: 2, mask: nil); PaxNativeHostState.reset() }
        let frame = FrameElement.makeDefault(id: 1, parentFrame: nil)
        frame.size_x = 300; frame.size_y = 260
        frame.clipContent = true
        FrameElements.singleton.add(element: frame)
        let scroller = ScrollerElement.makeDefault(id: 2, parentFrame: 1, renderLayerId: 0)
        scroller.size_x = 240; scroller.size_y = 180
        scroller.sizeInnerPaneX = 240; scroller.sizeInnerPaneY = 1000
        scroller.scrollEnabledY = true
        ScrollerElements.singleton.add(element: scroller)
        let hole = Path(CGRect(x: 170, y: 0, width: 70, height: 180))
        setResolvedNativeMask(id: 2, mask: ResolvedNativeMask(signature: 1052,
            size: CGSize(width: 240, height: 180), holes: [ResolvedMaskHole(signature: 1052,
                path: hole, clips: [], cgPath: hole.cgPath, clipCGPaths: [], opacity: 1)]))
        let text = TextElement.makeDefault(id: 3, parentFrame: 2, renderLayerId: 0)
        text.size_x = 200; text.size_y = 500
        text.selectable = true; text.clip = true
        text.textStyle.font_size = 18
        text.content = "<p><strong>Live rich text.</strong> A short heading.</p>" +
            String(repeating: "<p>A longer paragraph stays mounted while the card turns.</p>", count: 8)
        TextElements.singleton.add(element: text)
        let scene = UIHostingController(rootView: NativeRenderingLayer())
        let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 500, height: 700))
        window.rootViewController = scene
        window.makeKeyAndVisible()
        defer { window.isHidden = true; window.rootViewController = nil }
        var retained: UITextView?
        for angle: CGFloat in [35, 70, -70, 110, -110, 0] {
            let p = try projection(angle, x: 170, y: 200)
            let localScroll = PaxNativeProjection(affine: CGAffineTransform(translationX: 15, y: 20))
            let localText = PaxNativeProjection(affine: CGAffineTransform(translationX: 10, y: 100))
            frame.transform = coefficients(p)
            let aperture = try [CGPoint(x: 60, y: 0), CGPoint(x: 300, y: 0),
                                CGPoint(x: 300, y: 260), CGPoint(x: 60, y: 260)]
                .map { try XCTUnwrap(p.project($0)) }
            // Frame messages transport the aperture in projected world space.
            frame.clipPath = aperture.enumerated().map { i, point in
                "\(i == 0 ? "M" : "L")\(point.x) \(point.y)"
            }.joined(separator: " ") + " Z"
            scroller.transform = coefficients(p * localScroll)
            text.transform = coefficients(p * localScroll * localText)
            NativeSceneInvalidation.singleton.invalidate()
            flush(scene.view)
            let nativeText = try XCTUnwrap(descendants(scene.view).compactMap { $0 as? UITextView }.first)
            if let retained { XCTAssertTrue(retained === nativeText) } else { retained = nativeText }
            let scroll = try XCTUnwrap(descendants(scene.view).compactMap { $0 as? UIScrollView }
                .first { !($0 is UITextView) })
            let scrollHost = try XCTUnwrap(scroll.superview as? NativeRenderingLayer.PlatformContainerView)
            XCTAssertNil(scrollHost.hitTest(CGPoint(x: 20, y: 60), with: nil), "inherited aperture")
            XCTAssertNil(scrollHost.hitTest(CGPoint(x: 200, y: 60), with: nil), "opaque occluder")
            XCTAssertNotNil(scrollHost.hitTest(CGPoint(x: 100, y: 60), with: nil))
            scroll.contentOffset = CGPoint(x: 0, y: 80)
            flush(scene.view)
            let leaf = try XCTUnwrap(nativeText.superview?.superview as? NativeRenderingLayer.PlatformMaskedLeafView)
            let frameHost = try XCTUnwrap(scroll.superview?.superview?.superview as? NativeRenderingLayer.PlatformContainerView)
            // Compare in the native root, accounting for the scroll view's local
            // offset once, without flattening or double-applying perspective.
            let point = CGPoint(x: 60, y: 30)
            let expected = try XCTUnwrap((p * localScroll * PaxNativeProjection(affine:
                CGAffineTransform(translationX: 10, y: 20))).project(point))
            assertPoint(leaf.convert(point, to: frameHost), expected)
            XCTAssertEqual(scroll.contentOffset.y, 80, accuracy: 0.01)
            XCTAssertTrue(nativeText.attributedText.string.contains("Live rich text."))
            XCTAssertTrue(nativeText.attributedText.string.contains("longer paragraph"))
        }
    }
}
#endif
