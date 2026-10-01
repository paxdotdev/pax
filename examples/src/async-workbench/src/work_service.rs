//! Platform I/O lives outside the declarative template and synchronous handlers.
use pax_kit::*;

#[cfg(not(target_arch = "wasm32"))]
pub use pax_tokio::{BoundTasks, TokioService as Executor};
#[cfg(target_arch = "wasm32")]
pub use pax_web_async::{BoundTasks, BrowserService as Executor};

pub fn configure(app: &mut AppBuilder) -> AppResult {
    #[cfg(not(target_arch = "wasm32"))]
    app.provide_managed(Executor::owned(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?,
    )?)?;
    #[cfg(target_arch = "wasm32")]
    app.provide_managed(Executor::default())?;
    Ok(())
}

pub fn record(history: &Property<Vec<String>>, message: String) {
    history.update(|entries| {
        entries.push(format!("{:.0} ms · {message}", monotonic_millis()));
        if entries.len() > 100 {
            entries.drain(..entries.len() - 100);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn delay(millis: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(millis as u64)).await;
}
#[cfg(target_arch = "wasm32")]
pub async fn delay(millis: u32) {
    use wasm_bindgen::{closure::Closure, JsCast};
    struct Timer {
        id: i32,
        _callback: Closure<dyn FnMut()>,
    }
    impl std::ops::Drop for Timer {
        fn drop(&mut self) {
            web_sys::window()
                .unwrap()
                .clear_timeout_with_handle(self.id);
        }
    }
    let mut timer = None;
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let callback = Closure::wrap(Box::new(move || {
            let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
        }) as Box<dyn FnMut()>);
        let id = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                millis as i32,
            )
            .unwrap();
        timer = Some(Timer {
            id,
            _callback: callback,
        });
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    drop(timer);
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn local_io() -> Result<String, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = async {
            let (mut stream, _) = listener.accept().await?;
            stream.write_all(b"Hello from Tokio loopback I/O").await
        };
        let client = async {
            let mut stream = tokio::net::TcpStream::connect(address).await?;
            let mut response = String::new();
            stream.read_to_string(&mut response).await?;
            Ok::<_, std::io::Error>(response)
        };
        let (_, result) = tokio::try_join!(server, client)?;
        Ok::<_, std::io::Error>(result)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}
#[cfg(target_arch = "wasm32")]
pub async fn local_io() -> Result<String, String> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().unwrap();
    struct AbortFetch(web_sys::AbortController);
    impl std::ops::Drop for AbortFetch {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let abort = AbortFetch(web_sys::AbortController::new().map_err(|error| format!("{error:?}"))?);
    let init = web_sys::RequestInit::new();
    init.set_signal(Some(&abort.0.signal()));
    let request = web_sys::Request::new_with_str_and_init("sample.json", &init)
        .map_err(|error| format!("{error:?}"))?;
    let result = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|error| format!("{error:?}"))?;
    let response: web_sys::Response = result.dyn_into().map_err(|_| "invalid response")?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    let body = wasm_bindgen_futures::JsFuture::from(
        response.text().map_err(|error| format!("{error:?}"))?,
    )
    .await
    .map_err(|error| format!("{error:?}"))?;
    Ok(body.as_string().unwrap_or_default())
}

fn monotonic_millis() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        ORIGIN
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
            * 1000.0
    }
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().unwrap().performance().unwrap().now()
    }
}
