use pax_runtime_api::{LocalProperty, Property, PropertyGraph};
use std::{hint::black_box, time::Instant};

fn measure(name: &str, iterations: usize, mut work: impl FnMut()) {
    let start = Instant::now();
    for _ in 0..iterations {
        work();
    }
    println!(
        "{name}: {:.1} ns/op ({iterations} iterations)",
        start.elapsed().as_nanos() as f64 / iterations as f64
    );
}

fn main() {
    let iterations = 200_000;
    let local = LocalProperty::new(1_usize);
    let shared = Property::new(1_usize);
    measure("local get", iterations, || {
        black_box(local.get());
    });
    measure("shared get", iterations, || {
        black_box(shared.get());
    });
    measure("local set", iterations, || local.set(black_box(1)));
    measure("shared set, unattached", iterations, || {
        shared.set(black_box(1))
    });
    measure("shared update", iterations, || shared.update(|n| *n += 1));
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let source = shared.local();
    let dependents: Vec<_> = (0..100)
        .map(|_| {
            let source = source.clone();
            let dependencies = [source.untyped()];
            LocalProperty::computed(move || source.get() + 1, &dependencies)
        })
        .collect();
    measure("publish/import/read 100 dependents", 2_000, || {
        shared.set(black_box(1));
        graph.import();
        for dependent in &dependents {
            black_box(dependent.get());
        }
    });
    measure("100 coalesced worker-style writes/import", 2_000, || {
        for n in 0..100 {
            shared.set(black_box(n));
        }
        graph.import();
    });
    println!(
        "handle bytes (excludes allocations): local={}, shared={}",
        std::mem::size_of_val(&local),
        std::mem::size_of_val(&shared)
    );
    graph.close();
    drop(_entered);
    allocations("local literal", || LocalProperty::new(0_usize));
    allocations("shared unattached", || Property::new(0_usize));
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    allocations("shared + local projection", || {
        let shared = Property::new(0_usize);
        shared.local();
        shared
    });
    graph.collect();
    graph.close();
}

// Allocation accounting is confined to this diagnostic binary.
struct CountingAllocator;
static LIVE_BYTES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static ALLOCATION_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let pointer = std::alloc::System.alloc(layout);
        if !pointer.is_null() {
            LIVE_BYTES.fetch_add(layout.size(), std::sync::atomic::Ordering::Relaxed);
            ALLOCATION_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: std::alloc::Layout) {
        LIVE_BYTES.fetch_sub(layout.size(), std::sync::atomic::Ordering::Relaxed);
        std::alloc::System.dealloc(pointer, layout);
    }
    unsafe fn realloc(
        &self,
        pointer: *mut u8,
        layout: std::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        let result = std::alloc::System.realloc(pointer, layout, new_size);
        if !result.is_null() {
            LIVE_BYTES.fetch_add(new_size, std::sync::atomic::Ordering::Relaxed);
            LIVE_BYTES.fetch_sub(layout.size(), std::sync::atomic::Ordering::Relaxed);
            ALLOCATION_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        result
    }
}

fn allocations<T>(name: &str, make: impl Fn() -> T) {
    const COUNT: usize = 2_000;
    let mut values = Vec::with_capacity(COUNT);
    let before = LIVE_BYTES.load(std::sync::atomic::Ordering::Relaxed);
    let calls = ALLOCATION_CALLS.load(std::sync::atomic::Ordering::Relaxed);
    for _ in 0..COUNT {
        values.push(make());
    }
    let retained = LIVE_BYTES.load(std::sync::atomic::Ordering::Relaxed) - before;
    let calls = ALLOCATION_CALLS.load(std::sync::atomic::Ordering::Relaxed) - calls;
    println!(
        "{name}: {:.1} retained bytes/property, {:.2} allocations/property (batch {COUNT})",
        retained as f64 / COUNT as f64,
        calls as f64 / COUNT as f64
    );
    drop(values);
}
