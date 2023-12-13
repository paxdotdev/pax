#[cfg_attr(debug_assertions, derive(Debug))]
pub enum FormEvent {
    Toggle { state: bool },
}
