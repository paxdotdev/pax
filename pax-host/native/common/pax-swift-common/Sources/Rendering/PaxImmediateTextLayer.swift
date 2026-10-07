import QuartzCore

/// Pax samples its own transitions; Core Animation must not animate text redraws.
final class PaxImmediateTextLayer: CATextLayer {
    override func action(forKey event: String) -> CAAction? {
        // CATextLayer can replace contents during a later display pass, after the
        // property update's disabled-actions transaction has already committed.
        nil
    }
}
