import CoreGraphics

/// Conservatively bounds native GPU backing allocation in a common coordinate space.
/// Native views and their state remain mounted when their GPU surfaces are outside this region.
public enum PaxSurfaceVisibility {
    public static func intersectsPrewarmRegion(
        surface: CGRect,
        clippingBounds: [CGRect],
        padding: CGFloat
    ) -> Bool {
        // Invalid transient geometry must not turn into an allocation of an unbounded surface.
        guard isFinite(surface), !surface.isEmpty else { return false }
        let padding = padding.isFinite ? max(0, padding) : 0
        var intersection = surface
        for clip in clippingBounds {
            guard isFinite(clip), !clip.isEmpty else { return false }
            intersection = intersection.intersection(clip.insetBy(dx: -padding, dy: -padding))
            if intersection.isNull || intersection.isEmpty { return false }
        }
        return true
    }

    private static func isFinite(_ rect: CGRect) -> Bool {
        !rect.isInfinite && !rect.isNull && rect.origin.x.isFinite && rect.origin.y.isFinite
            && rect.width.isFinite && rect.height.isFinite
    }
}
