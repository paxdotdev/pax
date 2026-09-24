import XCTest
@testable import Rendering

final class PaxSurfaceVisibilityTests: XCTestCase {
    func testCatalogAllocationDoesNotGrowWithOffscreenShelfCount() {
        let viewport = CGRect(x: 0, y: 0, width: 440, height: 956)
        func activeShelves(_ count: Int, scroll: CGFloat) -> [Int] {
            (0..<count).filter { index in
                let shelf = CGRect(x: 0, y: 468 + CGFloat(index) * 303 - scroll, width: 440, height: 213)
                let surface = CGRect(x: 0, y: shelf.minY, width: 1365, height: 213)
                return PaxSurfaceVisibility.intersectsPrewarmRegion(
                    surface: surface, clippingBounds: [shelf, viewport], padding: 384
                )
            }
        }
        XCTAssertEqual(activeShelves(16, scroll: 0), [0, 1, 2])
        XCTAssertEqual(activeShelves(160, scroll: 0), [0, 1, 2])
        XCTAssertTrue(activeShelves(16, scroll: 3030).contains(10))
        XCTAssertFalse(activeShelves(16, scroll: 3030).contains(0))
        XCTAssertEqual(activeShelves(16, scroll: 0), [0, 1, 2])
    }

    func testNestedClipsLimitSurfacesEvenInsideTheScreen() {
        let viewport = CGRect(x: 0, y: 0, width: 440, height: 956)
        let parentClip = CGRect(x: 0, y: 100, width: 440, height: 200)
        XCTAssertFalse(PaxSurfaceVisibility.intersectsPrewarmRegion(
            surface: CGRect(x: 0, y: 700, width: 440, height: 100),
            clippingBounds: [viewport, parentClip], padding: 64
        ))
        XCTAssertTrue(PaxSurfaceVisibility.intersectsPrewarmRegion(
            surface: CGRect(x: 0, y: 320, width: 440, height: 100),
            clippingBounds: [viewport, parentClip], padding: 64
        ))
    }

    func testHorizontalTilesReenterAndRotationUsesNewViewport() {
        let portrait = CGRect(x: 0, y: 0, width: 440, height: 956)
        let landscape = CGRect(x: 0, y: 0, width: 956, height: 440)
        let tile = CGRect(x: 900, y: 0, width: 465, height: 213)
        XCTAssertFalse(PaxSurfaceVisibility.intersectsPrewarmRegion(
            surface: tile, clippingBounds: [portrait], padding: 384
        ))
        XCTAssertTrue(PaxSurfaceVisibility.intersectsPrewarmRegion(
            surface: tile, clippingBounds: [landscape], padding: 384
        ))
        XCTAssertTrue(PaxSurfaceVisibility.intersectsPrewarmRegion(
            surface: tile.offsetBy(dx: -800, dy: 0), clippingBounds: [portrait], padding: 384
        ))
    }

    func testUnclippedOverflowAndInvalidGeometry() {
        let viewport = CGRect(x: 0, y: 0, width: 440, height: 956)
        XCTAssertTrue(PaxSurfaceVisibility.intersectsPrewarmRegion(
            surface: CGRect(x: -100, y: 40, width: 200, height: 100),
            clippingBounds: [viewport], padding: 0
        ))
        for surface in [CGRect.zero, CGRect.infinite, CGRect.null] {
            XCTAssertFalse(PaxSurfaceVisibility.intersectsPrewarmRegion(
                surface: surface, clippingBounds: [viewport], padding: 384
            ))
        }
    }
}
