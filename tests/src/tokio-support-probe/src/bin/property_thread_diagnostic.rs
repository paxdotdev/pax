use pax_runtime_api::{LocalProperty, Property, PropertyGraph};

fn assert_send_sync<T: Send + Sync>() {}

fn main() {
    assert_send_sync::<Property<u32>>();
    let (wake, awakened) = std::sync::mpsc::channel();
    let graph = PropertyGraph::new(move || {
        let _ = wake.send(());
    });
    let _entered = graph.enter();
    let value = Property::new(7_u32);
    let local = value.local();
    let source = local.clone();
    let doubled = LocalProperty::computed(move || source.get() * 2, &[local.untyped()]);
    assert_eq!(doubled.get(), 14);
    let worker_value = value.clone();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()
        .unwrap();
    runtime
        .block_on(runtime.spawn(async move {
            let unrelated = LocalProperty::new(99_u32);
            assert_eq!(worker_value.get(), 7);
            worker_value.set(21);
            assert_eq!(unrelated.get(), 99);
        }))
        .unwrap();
    awakened
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    assert_eq!(value.get(), 21);
    assert_eq!(local.get(), 7);
    graph.import();
    assert_eq!(doubled.get(), 42);
    graph.close();
    println!("Worker publication and drop are safe; the owner graph observes 21 -> 42.");
}
