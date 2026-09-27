//! Example-owned loader: shared requests, retained cache, and revocable consumers.
//! Only plain data crosses the worker channel; callbacks stay on the Pax thread.
use std::{collections::HashMap, sync::mpsc};

pub type Page = Result<Vec<String>, String>;
type Callback = Box<dyn FnOnce(u64, Page)>;

#[derive(Default)]
pub struct Cache {
    next_lease: u64,
    pages: HashMap<usize, Option<Page>>,
    consumers: HashMap<u64, (usize, Callback)>,
    ready: Vec<u64>,
}

impl Cache {
    /// Returns a lease and whether the caller must start a new request.
    pub fn acquire(&mut self, page: usize, callback: Callback) -> (u64, bool) {
        self.next_lease += 1;
        let lease = self.next_lease;
        let start = !self.pages.contains_key(&page);
        let cached = self.pages.entry(page).or_insert(None).is_some();
        self.consumers.insert(lease, (page, callback));
        if cached {
            self.ready.push(lease);
        }
        (lease, start)
    }

    pub fn release(&mut self, lease: u64) {
        self.consumers.remove(&lease);
    }

    pub fn complete(&mut self, page: usize, result: Page) {
        self.pages.insert(page, Some(result));
        self.ready.extend(
            self.consumers
                .iter()
                .filter_map(|(&lease, (key, _))| (*key == page).then_some(lease)),
        );
    }

    pub fn deliveries(&mut self) -> Vec<(u64, Callback, Page)> {
        std::mem::take(&mut self.ready)
            .into_iter()
            .filter_map(|lease| {
                let (page, callback) = self.consumers.remove(&lease)?;
                let result = self.pages.get(&page)?.as_ref()?.clone();
                Some((lease, callback, result))
            })
            .collect()
    }
}

pub struct Loader {
    pub cache: Cache,
    sender: mpsc::Sender<(usize, Page)>,
    receiver: mpsc::Receiver<(usize, Page)>,
}
impl Default for Loader {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            cache: Cache::default(),
            sender,
            receiver,
        }
    }
}
impl Loader {
    pub fn acquire(&mut self, page: usize, callback: impl FnOnce(u64, Page) + 'static) -> u64 {
        let (lease, start) = self.cache.acquire(page, Box::new(callback));
        if start {
            load(page, self.sender.clone());
        }
        lease
    }
    pub fn poll(&mut self) -> Vec<(u64, Callback, Page)> {
        for (page, result) in self.receiver.try_iter() {
            self.cache.complete(page, result);
        }
        self.cache.deliveries()
    }
}

fn decode(text: &str) -> Page {
    let notes: Vec<_> = text.lines().map(str::to_owned).collect();
    if notes.len() == 6 {
        Ok(notes)
    } else {
        Err("Expected six field notes".into())
    }
}

// Native uses a bundled fixture on a worker, so the example needs no server.
// Replace this transport with an application HTTP client; the lease policy is shared.
#[cfg(not(target_arch = "wasm32"))]
fn load(page: usize, sender: mpsc::Sender<(usize, Page)>) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(350));
        let _ = sender.send((page, decode(include_str!("../public/field-notes.txt"))));
    });
}

#[cfg(target_arch = "wasm32")]
fn load(page: usize, sender: mpsc::Sender<(usize, Page)>) {
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::{spawn_local, JsFuture};
    spawn_local(async move {
        let result = async {
            let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
            // Deliberate latency makes departure before completion easy to exercise.
            let delay = js_sys::Promise::new(&mut |resolve, reject| {
                if let Err(error) =
                    window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 350)
                {
                    let _ = reject.call1(&JsValue::NULL, &error);
                }
            });
            JsFuture::from(delay).await?;
            let response =
                JsFuture::from(window.fetch_with_str(&format!("field-notes.txt?page={page}")))
                    .await?
                    .dyn_into::<web_sys::Response>()?;
            if !response.ok() {
                return Err(JsValue::from_str(&format!("HTTP {}", response.status())));
            }
            let text = JsFuture::from(response.text()?)
                .await?
                .as_string()
                .ok_or_else(|| JsValue::from_str("Expected text response"))?;
            decode(&text).map_err(|error| JsValue::from_str(&error))
        }
        .await
        .map_err(|error: JsValue| format!("{error:?}"));
        let _ = sender.send((page, result));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    fn subscribe(cache: &mut Cache, page: usize, results: &Rc<RefCell<Vec<u64>>>) -> (u64, bool) {
        let results = results.clone();
        cache.acquire(
            page,
            Box::new(move |lease, _| results.borrow_mut().push(lease)),
        )
    }
    fn deliver(cache: &mut Cache) {
        for (lease, callback, result) in cache.deliveries() {
            callback(lease, result);
        }
    }
    #[test]
    fn deduplicates_requests_and_revokes_stale_consumers() {
        let mut cache = Cache::default();
        let results = Rc::new(RefCell::new(Vec::new()));
        let (old, start) = subscribe(&mut cache, 0, &results);
        assert!(start);
        let (other, start) = subscribe(&mut cache, 0, &results);
        assert!(!start);
        cache.release(old); // departure/unmount while the shared request continues
        let (new, start) = subscribe(&mut cache, 0, &results);
        assert!(!start);
        cache.complete(0, decode(include_str!("../public/field-notes.txt")));
        deliver(&mut cache);
        let mut got = results.borrow().clone();
        got.sort();
        assert_eq!(got, [other, new]);
        assert!(cache.consumers.is_empty());
        let (cached, start) = subscribe(&mut cache, 0, &results);
        assert!(!start);
        deliver(&mut cache);
        assert_eq!(results.borrow().last(), Some(&cached));
    }
    #[test]
    fn release_after_completion_before_delivery_still_suppresses_the_write() {
        let mut cache = Cache::default();
        let results = Rc::new(RefCell::new(Vec::new()));
        let (lease, _) = subscribe(&mut cache, 0, &results);
        cache.complete(0, Err("offline".into()));
        cache.release(lease);
        deliver(&mut cache);
        assert!(results.borrow().is_empty());
        assert!(cache.consumers.is_empty());
    }
    #[test]
    fn late_old_page_cannot_complete_a_rebound_consumer() {
        let mut cache = Cache::default();
        let results = Rc::new(RefCell::new(Vec::new()));
        let (old, _) = subscribe(&mut cache, 0, &results);
        cache.release(old);
        let (new, _) = subscribe(&mut cache, 1, &results);
        cache.complete(0, Ok(vec![]));
        deliver(&mut cache);
        assert!(results.borrow().is_empty());
        cache.complete(1, Ok(vec![]));
        deliver(&mut cache);
        assert_eq!(*results.borrow(), [new]);
    }
}
