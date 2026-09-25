import XCTest
import CoreGraphics
import QuartzCore
@testable import Rendering

final class PaxNativeMaskRasterTests: XCTestCase {
    private func rect(_ x: CGFloat, _ y: CGFloat, _ width: CGFloat, _ height: CGFloat) -> CGPath {
        CGPath(rect: CGRect(x: x, y: y, width: width, height: height), transform: nil)
    }

    private func pixels(_ image: CGImage) -> [UInt8] {
        let context = CGContext(data: nil, width: image.width, height: image.height,
            bitsPerComponent: 8, bytesPerRow: image.width * 4,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue)!
        context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
        return Array(UnsafeBufferPointer(start: context.data!.assumingMemoryBound(to: UInt8.self), count: image.width * image.height * 4))
    }

    func testOverlappingHolesMultiplyAndClipsRestrictEachHole() {
        let payload = RasterizedNativeMaskPayload(signature: 1, size: CGSize(width: 16, height: 12), holes: [
            RasterizedMaskHolePayload(cgPath: rect(2, 0, 8, 12), clipCGPaths: [rect(0, 0, 6, 12)], opacity: 0.5),
            RasterizedMaskHolePayload(cgPath: rect(4, 0, 8, 12), clipCGPaths: [], opacity: 0.25),
        ])
        for scale: CGFloat in [1, 2, 3] {
            let image = rasterizedMaskImage(payload: payload, scale: scale)!
            let rgba = pixels(image)
            for (x, expected) in [(1, 255), (3, 128), (5, 96), (7, 191), (14, 255)] {
                let alpha = rgba[(6 * Int(scale) * image.width + x * Int(scale)) * 4 + 3]
                XCTAssertLessThanOrEqual(abs(Int(alpha) - expected), 1, "x=\(x), scale=\(scale)")
            }
        }
    }

    func testEmptyAndZeroOpacityMasksRemainOpaque() {
        for holes in [[], [RasterizedMaskHolePayload(cgPath: rect(0, 0, 9, 7), clipCGPaths: [], opacity: 0)]] {
            let image = rasterizedMaskImage(payload: RasterizedNativeMaskPayload(
                signature: 2, size: CGSize(width: 9, height: 7), holes: holes), scale: 2)!
            XCTAssertTrue(image.isMask)
            XCTAssertEqual(image.bitsPerPixel, 8)
            XCTAssertEqual(image.bytesPerRow, image.width)
            let rgba = pixels(image)
            for index in stride(from: 3, to: rgba.count, by: 4) { XCTAssertEqual(rgba[index], 255) }
        }
    }

    func testMaskImageWorksAsCoreAnimationAlphaCoverage() {
        let size = CGSize(width: 16, height: 12)
        let image = rasterizedMaskImage(payload: RasterizedNativeMaskPayload(signature: 3, size: size, holes: [
            RasterizedMaskHolePayload(cgPath: rect(4, 0, 4, 12), clipCGPaths: [], opacity: 1),
            RasterizedMaskHolePayload(cgPath: rect(8, 0, 4, 12), clipCGPaths: [], opacity: 0.5),
        ]), scale: 1)!
        let content = CALayer()
        content.frame = CGRect(origin: .zero, size: size)
        content.backgroundColor = CGColor(red: 1, green: 0, blue: 0, alpha: 1)
        let mask = CALayer()
        mask.frame = content.bounds
        mask.contents = image
        mask.contentsGravity = .resize
        content.mask = mask
        let context = CGContext(data: nil, width: 16, height: 12, bitsPerComponent: 8, bytesPerRow: 64,
            space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue)!
        content.render(in: context)
        let rgba = pixels(context.makeImage()!)
        for (x, expected) in [(2, 255), (6, 0), (10, 128), (14, 255)] {
            XCTAssertLessThanOrEqual(abs(Int(rgba[(6 * 16 + x) * 4 + 3]) - expected), 1)
        }
    }
}
