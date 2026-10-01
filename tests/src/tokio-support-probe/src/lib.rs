//! Native executor and property ownership checks.
//!
//! UI-local values cannot be captured by a Tokio worker:
//! ```compile_fail
//! use pax_runtime_api::LocalProperty;
//! let value = LocalProperty::new(1_u32);
//! tokio::spawn(async move { value.get() });
//! ```

#[cfg(test)]
mod tests {
    use pax_runtime_api::{LocalProperty, Property, PropertyGraph};
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc,
        },
        thread,
        time::Duration,
    };
    use tokio::{runtime::Builder, task::LocalSet};

    #[test]
    fn native_worker_returns_data_and_owner_updates_reactive_graph() {
        let owner = thread::current().id();
        let value = Property::new(0u32);
        let graph = PropertyGraph::new(|| {});
        let _entered = graph.enter();
        let source = value.local();
        let derived = LocalProperty::computed(move || source.get() * 2, &[value.untyped()]);
        assert_eq!(derived.get(), 0);
        let rt = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_time()
            .build()
            .unwrap();
        let (tx, rx) = mpsc::sync_channel(1);
        let task = rt.spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            tx.send((thread::current().id(), 21u32)).unwrap();
        });
        // This blocks only the test harness. A real chassis needs a wakeup/inbox.
        let (worker, result) = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_ne!(owner, worker);
        value.set(result);
        graph.import();
        assert_eq!(derived.get(), 42);
        rt.block_on(task).unwrap();
    }

    #[test]
    fn localset_can_update_property_on_its_owning_thread_when_driven() {
        let owner = thread::current().id();
        let rt = Builder::new_current_thread().enable_time().build().unwrap();
        let local = LocalSet::new();
        let value = Property::new(0u32);
        let output = value.clone();
        let task = local.spawn_local(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            assert_eq!(owner, thread::current().id());
            output.set(42);
        });
        local.block_on(&rt, task).unwrap();
        assert_eq!(value.get(), 42);
    }

    #[test]
    fn entered_current_thread_runtime_does_not_run_background_tasks() {
        let rt = Builder::new_current_thread().build().unwrap();
        let ran = Arc::new(AtomicBool::new(false));
        let task_ran = Arc::clone(&ran);
        let task = {
            let _entered = rt.enter();
            tokio::spawn(async move {
                task_ran.store(true, Ordering::SeqCst);
            })
        };
        assert!(!ran.load(Ordering::SeqCst));
        rt.block_on(task).unwrap();
        assert!(ran.load(Ordering::SeqCst));
    }

    #[test]
    fn explicit_abort_drops_pending_local_future_without_updating_state() {
        struct Dropped(Rc<Cell<bool>>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let rt = Builder::new_current_thread().build().unwrap();
        let local = LocalSet::new();
        let value = Property::new(0u32);
        let output = value.clone();
        let dropped = Rc::new(Cell::new(false));
        let guard = Dropped(Rc::clone(&dropped));
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let task = local.spawn_local(async move {
            let _guard = guard;
            started_tx.send(()).unwrap();
            std::future::pending::<()>().await;
            output.set(99);
        });
        local.block_on(&rt, async {
            started_rx.await.unwrap();
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        });
        assert!(dropped.get());
        assert_eq!(value.get(), 0);
    }

    #[test]
    fn worker_observes_closed_receiver_without_touching_properties() {
        let rt = Builder::new_multi_thread()
            .worker_threads(1)
            .build()
            .unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel::<u32>(1);
        drop(rx);
        let task = rt.spawn(async move { tx.send(42).await.is_err() });
        assert!(rt.block_on(task).unwrap());
    }
}
