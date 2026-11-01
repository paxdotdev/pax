import Foundation
import QuartzCore

// Local-plane homography. Native layout remains in local points; presentation
// and input share this mapping rather than flattening a 4x4 message to affine.
struct PaxNativeProjection: Equatable {
    let m: [CGFloat]

    init?(_ coefficients: [Float]) {
        switch coefficients.count {
        case 6:
            let c = coefficients.map(CGFloat.init)
            m = [c[0], c[1], 0, c[2], c[3], 0, c[4], c[5], 1]
        case 16:
            m = [0, 1, 3, 4, 5, 7, 12, 13, 15].map { CGFloat(coefficients[$0]) }
        default: return nil
        }
        guard m.allSatisfy(\.isFinite) else { return nil }
    }

    init(affine t: CGAffineTransform) {
        m = [t.a, t.b, 0, t.c, t.d, 0, t.tx, t.ty, 1]
    }

    private init(_ matrix: [CGFloat]) { m = matrix }

    static let identity = PaxNativeProjection(affine: .identity)
    static let collapsed = PaxNativeProjection(affine: CGAffineTransform(scaleX: 0, y: 0))

    func project(_ p: CGPoint) -> CGPoint? {
        let w = m[2] * p.x + m[5] * p.y + m[8]
        guard w.isFinite, w > 0 else { return nil }
        let result = CGPoint(x: (m[0] * p.x + m[3] * p.y + m[6]) / w,
                             y: (m[1] * p.x + m[4] * p.y + m[7]) / w)
        return result.x.isFinite && result.y.isFinite ? result : nil
    }

    func bounds(of rect: CGRect) -> CGRect? {
        let corners = [CGPoint(x: rect.minX, y: rect.minY), CGPoint(x: rect.maxX, y: rect.minY),
                       CGPoint(x: rect.maxX, y: rect.maxY), CGPoint(x: rect.minX, y: rect.maxY)]
        let points = corners.compactMap(project)
        guard points.count == 4 else { return nil }
        let xs = points.map(\.x), ys = points.map(\.y)
        return CGRect(x: xs.min()!, y: ys.min()!, width: xs.max()!-xs.min()!, height: ys.max()!-ys.min()!)
    }

    func inverse() -> Self? {
        let scale = m.map(abs).max() ?? 0
        guard scale > 0 else { return nil }
        let a = m.map { $0 / scale }
        let c = [a[4]*a[8]-a[7]*a[5], a[7]*a[2]-a[1]*a[8], a[1]*a[5]-a[4]*a[2],
                 a[6]*a[5]-a[3]*a[8], a[0]*a[8]-a[6]*a[2], a[3]*a[2]-a[0]*a[5],
                 a[3]*a[7]-a[6]*a[4], a[6]*a[1]-a[0]*a[7], a[0]*a[4]-a[3]*a[1]]
        let determinant = a[0]*c[0] + a[3]*c[1] + a[6]*c[2]
        guard determinant.isFinite, determinant != 0 else { return nil }
        let result = c.map { $0 / determinant / scale }
        return result.allSatisfy(\.isFinite) ? Self(result) : nil
    }

    static func * (lhs: Self, rhs: Self) -> Self {
        var result = Array(repeating: CGFloat(0), count: 9)
        for col in 0..<3 {
            for row in 0..<3 {
                for k in 0..<3 { result[col*3+row] += lhs.m[k*3+row] * rhs.m[col*3+k] }
            }
        }
        return Self(result)
    }

    var layerTransform: CATransform3D {
        var t = CATransform3DIdentity
        t.m11=m[0]; t.m12=m[1]; t.m14=m[2]
        t.m21=m[3]; t.m22=m[4]; t.m24=m[5]
        t.m41=m[6]; t.m42=m[7]; t.m44=m[8]
        return t
    }

    func projectPath(_ source: CGPath) -> CGPath? {
        let output = CGMutablePath()
        var current = CGPoint.zero, start = CGPoint.zero, valid = true
        func curve(_ points: [CGPoint], depth: Int = 0) {
            let projected = points.compactMap(project)
            guard projected.count == points.count else { valid = false; return }
            let a = projected.first!, b = projected.last!
            let dx: CGFloat = b.x-a.x, dy: CGFloat = b.y-a.y
            let lengthSquared: CGFloat = dx*dx + dy*dy
            var error: CGFloat = 0
            for p in projected.dropFirst().dropLast() {
                let offsetX: CGFloat = p.x-a.x, offsetY: CGFloat = p.y-a.y
                let dot: CGFloat = offsetX*dx + offsetY*dy
                let t: CGFloat = lengthSquared > 0 ? min(max(dot/lengthSquared, 0), 1) : 0
                let errorX: CGFloat = offsetX-t*dx, errorY: CGFloat = offsetY-t*dy
                error = max(error, (errorX*errorX + errorY*errorY).squareRoot())
            }
            if error <= 0.1 { output.addLine(to: b); return }
            guard depth < 24 else { valid = false; return }
            var row = points, left = [points.first!], right = [points.last!]
            while row.count > 1 {
                row = zip(row, row.dropFirst()).map { a, b -> CGPoint in
                    let x: CGFloat = (a.x+b.x)/2, y: CGFloat = (a.y+b.y)/2
                    return CGPoint(x: x, y: y)
                }
                left.append(row.first!); right.append(row.last!)
            }
            curve(left, depth: depth+1); curve(Array(right.reversed()), depth: depth+1)
        }
        source.applyWithBlock { pointer in
            let element = pointer.pointee
            switch element.type {
            case .moveToPoint:
                current=element.points[0]; start=current
                if let point=self.project(current) { output.move(to: point) } else { valid=false }
            case .addLineToPoint:
                current=element.points[0]
                if let point=self.project(current) { output.addLine(to: point) } else { valid=false }
            case .addQuadCurveToPoint:
                curve([current, element.points[0], element.points[1]]); current=element.points[1]
            case .addCurveToPoint:
                curve([current, element.points[0], element.points[1], element.points[2]]); current=element.points[2]
            case .closeSubpath: output.closeSubpath(); current=start
            @unknown default: valid=false
            }
        }
        return valid ? output : nil
    }
}

#if os(macOS)
import AppKit

protocol PaxProjectedNativeHost: NSView {
    var nativeProjection: PaxNativeProjection? { get }
    func updateProjectedFocus(_ owner: NSView?, focused: Bool)
}

extension NSView {
    private var paxProjectionHosts: [any PaxProjectedNativeHost] {
        var result: [any PaxProjectedNativeHost] = []
        var current: NSView? = self
        while let view = current {
            if let host = view as? any PaxProjectedNativeHost, host.nativeProjection != nil {
                result.append(host)
            }
            current = view.superview
        }
        return result
    }

    var paxHasNativeProjection: Bool { !paxProjectionHosts.isEmpty }

    // AppKit frames conservatively cover the projected plane for visibility
    // checks. Replace their affine mapping at each host boundary; ordinary
    // descendants, including scrolling clip views, retain native conversion.
    func paxLogicalWindowPoint(_ point: NSPoint) -> NSPoint? {
        var result = point
        for host in paxProjectionHosts.reversed() {
            guard let parent = host.superview, let inverse = host.nativeProjection?.inverse(),
                  let local = inverse.project(parent.convert(result, from: nil)) else { return nil }
            result = host.convert(local, to: nil)
        }
        return result
    }

    func paxVisualWindowPoint(_ point: NSPoint) -> NSPoint? {
        var result = point
        for host in paxProjectionHosts {
            guard let parent = host.superview,
                  let projected = host.nativeProjection?.project(host.convert(result, from: nil)) else { return nil }
            result = parent.convert(projected, to: nil)
        }
        return result
    }

    func paxLogicalScreenPoint(_ point: NSPoint) -> NSPoint? {
        guard let window, let local = paxLogicalWindowPoint(window.convertPoint(fromScreen: point)) else { return nil }
        return window.convertPoint(toScreen: local)
    }

    func paxVisualScreenRect(_ rect: NSRect) -> NSRect {
        guard let window else { return rect }
        let r = window.convertFromScreen(rect)
        let corners = [NSPoint(x:r.minX,y:r.minY), NSPoint(x:r.maxX,y:r.minY),
                       NSPoint(x:r.maxX,y:r.maxY), NSPoint(x:r.minX,y:r.maxY)]
        let points = corners.compactMap(paxVisualWindowPoint)
        guard points.count == 4 else { return .zero }
        let xs=points.map(\.x), ys=points.map(\.y)
        return window.convertToScreen(NSRect(x:xs.min()!, y:ys.min()!,
            width:xs.max()!-xs.min()!, height:ys.max()!-ys.min()!))
    }

    func paxUpdateProjectedFocus(_ owner: NSView, focused: Bool) {
        paxProjectionHosts.first?.updateProjectedFocus(owner, focused: focused)
    }
}

// NSTextView's selection tracker forwards autoscroll events to its clip view.
// Correcting only the text view leaves that tracker using an affine viewport.
final class PaxProjectedClipView: NSClipView {
    override func convert(_ point: NSPoint, from view: NSView?) -> NSPoint {
        guard view == nil, paxHasNativeProjection else { return super.convert(point, from: view) }
        guard let logical = paxLogicalWindowPoint(point) else { return NSPoint(x: CGFloat.infinity, y: CGFloat.infinity) }
        return super.convert(logical, from: nil)
    }
}
#endif
