import Foundation

/// Bounded, non-destructive change history for independently mounted native scene hosts.
/// A host that misses retained history reconciles from the authoritative element models.
public struct NativeSceneChangeJournal {
    public struct Changes {
        public var ids: Set<PaxNodeId> = []
        public var rebuild = false
    }

    public private(set) var generation: UInt64 = 0
    private var batches: [(generation: UInt64, changes: Changes)] = []
    private let capacity: Int

    public init(capacity: Int = 64) {
        self.capacity = max(1, capacity)
    }

    public mutating func append(ids: Set<PaxNodeId>, rebuild: Bool) {
        generation &+= 1
        batches.append((generation, Changes(ids: ids, rebuild: rebuild)))
        if batches.count > capacity { batches.removeFirst() }
    }

    public func changes(since previous: UInt64?) -> Changes {
        guard let previous else { return Changes(rebuild: true) }
        if previous == generation { return Changes() }
        guard previous < generation,
              let first = batches.first,
              previous >= first.generation - 1 else { return Changes(rebuild: true) }
        var result = Changes()
        for batch in batches where batch.generation > previous {
            if batch.changes.rebuild { return Changes(rebuild: true) }
            result.ids.formUnion(batch.changes.ids)
        }
        return result
    }
}
