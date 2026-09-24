import XCTest
import QuartzCore
@testable import Rendering

final class PaxNativePresentationTests: XCTestCase {
    func testTextRedrawDoesNotAcquireImplicitActionsOutsideUpdateTransaction() {
        let layer = PaxImmediateTextLayer()
        let keys = ["contents", "string", "foregroundColor", "fontSize", "bounds", "position", "opacity"]
        // Simulate an enclosing native animation transaction, including a contents
        // action supplied by the host. Pax's sampled values still apply immediately.
        layer.actions = Dictionary(uniqueKeysWithValues: keys.map {
            ($0, CABasicAnimation(keyPath: $0) as CAAction)
        })
        CATransaction.begin()
        CATransaction.setDisableActions(false)
        defer { CATransaction.commit() }
        for key in keys {
            XCTAssertNil(layer.action(forKey: key), key)
        }
        layer.string = "Light theme"
        layer.foregroundColor = CGColor(gray: 0, alpha: 1)
        layer.displayIfNeeded()
        layer.string = "Dark theme"
        layer.foregroundColor = CGColor(gray: 1, alpha: 1)
        layer.displayIfNeeded()
        XCTAssertTrue(layer.animationKeys()?.isEmpty ?? true)
    }

    func testNativeScenePublicationIsSynchronousAndSeesNewGeneration() {
        let scene = NativeSceneInvalidation()
        var generations: [UInt64] = []
        let observer = NotificationCenter.default.addObserver(
            forName: NativeSceneInvalidation.didInvalidate, object: nil, queue: nil
        ) { notification in
            guard let published = notification.object as? NativeSceneInvalidation,
                  published === scene else { return }
            generations.append(published.generation)
        }
        defer { NotificationCenter.default.removeObserver(observer) }
        scene.invalidate()
        XCTAssertEqual(generations, [1])
        scene.invalidate()
        XCTAssertEqual(generations, [1, 2])
    }
}
