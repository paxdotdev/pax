use pax_kit::*;

pax_kit::runnable! { root: crate::DesktopSmoke, init: crate::initialize }

#[pax]
#[file("lib.pax")]
pub struct DesktopSmoke {
    pub title: Property<String>,
    pub input: Property<String>,
    pub echoed: Property<String>,
    pub clicks: Property<usize>,
    pub checked: Property<bool>,
    pub rotation: Property<f64>,
    pub panel_rotation: Property<f64>,
}

pub fn initialize(context: &InitContext) -> AppResult<DesktopSmoke> {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    if context.startup_data() == b"pax-test-prepare-drop-async" {
        context
            .lifetime()
            .retain_cleanup(std::rc::Rc::new(DelayedPreparationCleanup))?;
    }
    let title = if context.startup_data().is_empty() {
        "Pax native desktop".to_owned()
    } else {
        std::str::from_utf8(context.startup_data())?.to_owned()
    };
    Ok(DesktopSmoke {
        title: Property::new(title),
        input: Property::new("Native selection and IME".into()),
        echoed: Property::new("Edit the native textbox".into()),
        ..Default::default()
    })
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
struct DelayedPreparationCleanup;

#[cfg(any(target_os = "windows", target_os = "linux"))]
impl application_shutdown::Cleanup for DelayedPreparationCleanup {
    fn begin_shutdown(&self) -> ShutdownTicket {
        let (ticket, complete) = ShutdownTicket::pending();
        println!("prepare-drop-async: cleanup begin");
        match std::thread::Builder::new()
            .name("pax-prepare-drop-ack".into())
            .spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(250));
                println!("prepare-drop-async: cleanup acknowledged");
                complete.complete(Ok(()));
            }) {
            Ok(_) => ticket,
            Err(error) => ShutdownTicket::ready(Err(format!(
                "Could not start diagnostic cleanup worker: {error}"
            ))),
        }
    }
}

impl DesktopSmoke {
    pub fn increment(&mut self, _context: &NodeContext, _event: Event<ButtonClick>) {
        self.clicks.set(self.clicks.get() + 1);
        self.panel_rotation.local().ease_to(
            if self.clicks.get() % 2 == 0 {
                -3.0
            } else {
                3.0
            },
            60,
            EasingCurve::OutQuad,
        );
        self.rotation
            .local()
            .ease_to(self.clicks.get() as f64 * 90.0, 60, EasingCurve::OutQuad);
    }

    pub fn edit(&mut self, _context: &NodeContext, event: Event<TextboxInput>) {
        self.echoed.set(format!("Rust received: {}", event.text));
    }
}
