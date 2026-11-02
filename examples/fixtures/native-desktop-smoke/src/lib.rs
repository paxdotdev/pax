use pax_kit::{cursor::CursorStyle, *};

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
    pub key_down_count: Property<usize>,
    pub key_up_count: Property<usize>,
    pub key_press_count: Property<usize>,
    pub key_repeat_count: Property<usize>,
    pub last_key_down: Property<String>,
    pub last_key_up: Property<String>,
    pub last_key_press: Property<String>,
    pub pointer_modifiers: Property<String>,
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
        last_key_down: Property::new("KeyDown: none".into()),
        last_key_up: Property::new("KeyUp: none".into()),
        last_key_press: Property::new("KeyPress: none".into()),
        pointer_modifiers: Property::new("Pointer: none".into()),
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
    fn modifiers(modifiers: &[ModifierKey]) -> String {
        modifiers
            .iter()
            .map(|modifier| match modifier {
                ModifierKey::Shift => "Shift",
                ModifierKey::Control => "Control",
                ModifierKey::Alt => "Alt",
                ModifierKey::Command => "Command",
            })
            .collect::<Vec<_>>()
            .join("+")
    }

    fn keyboard_description(phase: &str, keyboard: &KeyboardEventArgs) -> String {
        format!(
            "{phase}: key={} mods={} repeat={}",
            keyboard.key,
            Self::modifiers(&keyboard.modifiers),
            keyboard.is_repeat
        )
    }

    pub fn key_down(&mut self, _context: &NodeContext, event: Event<KeyDown>) {
        self.key_down_count.set(self.key_down_count.get() + 1);
        if event.keyboard.is_repeat {
            self.key_repeat_count.set(self.key_repeat_count.get() + 1);
        }
        self.last_key_down
            .set(Self::keyboard_description("KeyDown", &event.keyboard));
        // A printable cancellation proves that legacy KeyPress is suppressed too.
        if event.keyboard.key == "q" || event.keyboard.key == "Escape" {
            event.prevent_default();
        }
    }

    pub fn key_up(&mut self, _context: &NodeContext, event: Event<KeyUp>) {
        self.key_up_count.set(self.key_up_count.get() + 1);
        self.last_key_up
            .set(Self::keyboard_description("KeyUp", &event.keyboard));
        // Close can retire the native labels before another paint. This opt-in
        // fixture marker proves the final synthetic release reached Rust.
        #[cfg(target_os = "windows")]
        if std::env::var("PAX_NATIVE_INPUT_DIAGNOSTICS").as_deref() == Ok("1") {
            eprintln!(
                "pax-key-up: key={:?} modifiers={:?} count={}",
                event.keyboard.key,
                Self::modifiers(&event.keyboard.modifiers),
                self.key_up_count.get()
            );
        }
    }

    pub fn key_press(&mut self, _context: &NodeContext, event: Event<KeyPress>) {
        self.key_press_count.set(self.key_press_count.get() + 1);
        self.last_key_press
            .set(Self::keyboard_description("KeyPress", &event.keyboard));
    }

    pub fn mouse_down(&mut self, _context: &NodeContext, event: Event<MouseDown>) {
        self.pointer_modifiers.set(format!(
            "Pointer: mods={}",
            Self::modifiers(&event.mouse.modifiers)
        ));
    }

    pub fn cursor_enter(&mut self, context: &NodeContext, _event: Event<MouseOver>) {
        context.set_cursor(CursorStyle::Pointer);
    }

    pub fn cursor_leave(&mut self, context: &NodeContext, _event: Event<MouseOut>) {
        context.set_cursor(CursorStyle::Auto);
    }

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
