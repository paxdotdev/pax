import Foundation
import Messages

// A native input surface must keep UIKit hit ownership while forwarding to Pax's
// scene hit test. In particular, a modal underlay must not swallow the GPU panel
// above it or allow a drag/cancellation to become a backdrop click.
final class PaxTouchSequence {
    enum Phase { case began, moved, ended, cancelled }

    struct Contact {
        let identifier: Int64
        let x: Double
        let y: Double
    }

    private var active: [Int64: Contact] = [:]
    private var tapStart: Contact?
    private let tapMovementTolerance = 10.0

    func forward(_ phase: Phase, contacts: [Contact]) {
        let changed = contacts.sorted { $0.identifier < $1.identifier }.filter {
            phase == .began || active[$0.identifier] != nil
        }
        guard !changed.isEmpty else { return }

        if phase == .began {
            tapStart = active.isEmpty && changed.count == 1 ? changed.first : nil
        }
        if phase == .cancelled {
            tapStart = nil
        }
        if let start = tapStart,
           let current = changed.first(where: { $0.identifier == start.identifier }),
           hypot(current.x - start.x, current.y - start.y) > tapMovementTolerance {
            tapStart = nil
        }

        var messages = changed.map { contact in
            let previous = active[contact.identifier] ?? contact
            active[contact.identifier] = contact
            return TouchInterruptMessage(
                x: contact.x, y: contact.y, identifier: contact.identifier,
                deltaX: contact.x - previous.x, deltaY: contact.y - previous.y
            )
        }

        switch phase {
        case .began, .moved:
            let changedIds = Set(changed.map(\.identifier))
            messages += active.values.filter { !changedIds.contains($0.identifier) }
                .sorted { $0.identifier < $1.identifier }.map {
                    TouchInterruptMessage(
                        x: $0.x, y: $0.y, identifier: $0.identifier, deltaX: 0, deltaY: 0
                    )
                }
            if phase == .began {
                dispatchTouchStart(touches: messages)
            } else {
                dispatchTouchMove(touches: messages)
            }
        case .ended, .cancelled:
            for contact in changed {
                active.removeValue(forKey: contact.identifier)
            }
            if phase == .cancelled {
                dispatchTouchCancel(touches: messages)
            } else {
                dispatchTouchEnd(touches: messages)
                if active.isEmpty, changed.count == 1,
                   let contact = changed.first, contact.identifier == tapStart?.identifier {
                    dispatchTap(x: contact.x, y: contact.y)
                }
            }
            tapStart = nil
        }
    }
}
