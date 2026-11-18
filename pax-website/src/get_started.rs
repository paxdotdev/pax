use crate::SiteTheme;
use pax_kit::*;
use rand::Rng;

const CLI_COMMANDS: &str = "cargo install pax-cli\npax-cli create my-app\ncd my-app && pax-cli run";

#[pax]
#[file("get_started.pax")]
pub struct GetStarted {
    pub ai: Property<bool>,
    pub compact: Property<bool>,
    pub content_height: Property<f64>,
    pub prompt: Property<String>,
    pub status: Property<String>,
    pub copying: Property<bool>,
    pub copy_tip: Property<bool>,
    pub _height_bound: Property<bool>,
    pub _idea_index: Property<usize>,
    pub _copy_revision: Property<u64>,
}

impl GetStarted {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self._height_bound.set(false);
        crate::section_layout::bind_compact(ctx, &self.compact, 480.0);
        let index = rand::thread_rng().gen_range(0..crate::ai_prompt::IDEAS.len());
        self.set_idea(index);
    }

    pub fn measure(&mut self, ctx: &NodeContext) {
        if !self._height_bound.get() {
            self._height_bound
                .set(crate::section_layout::bind_height_with_padding(
                    ctx,
                    "cta_content",
                    &self.content_height,
                    &Property::new(0.0),
                ));
        }
    }

    fn reset_copy(&self) {
        // Ignore a clipboard completion from an older tab or starter idea.
        self._copy_revision
            .set(self._copy_revision.get().wrapping_add(1));
        self.copying.set(false);
        self.copy_tip.set(false);
        self.status.set(String::new());
    }

    fn set_idea(&self, index: usize) {
        self._idea_index.set(index);
        self.prompt
            .set(crate::ai_prompt::prompt(crate::ai_prompt::IDEAS[index]));
        self.reset_copy();
    }

    pub fn select_cli(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.ai.set(false);
        self.reset_copy();
    }

    pub fn select_ai(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.ai.set(true);
        self.reset_copy();
    }

    pub fn another_idea(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        let draw = rand::thread_rng().gen_range(0..crate::ai_prompt::IDEAS.len() - 1);
        self.set_idea(crate::ai_prompt::different_index(
            self._idea_index.get(),
            draw,
        ));
    }

    pub fn show_copy_tip(&mut self, _ctx: &NodeContext, _event: Event<MouseOver>) {
        self.copy_tip.set(true);
    }

    pub fn hide_copy_tip(&mut self, _ctx: &NodeContext, _event: Event<MouseOut>) {
        self.copy_tip.set(false);
    }

    pub fn touch_copy_tip(&mut self, _ctx: &NodeContext, _event: Event<TouchStart>) {
        self.copy_tip.set(true);
    }

    pub fn end_copy_touch(&mut self, _ctx: &NodeContext, _event: Event<TouchEnd>) {
        self.copy_tip.set(false);
    }

    fn active_text(&self) -> String {
        if self.ai.get() {
            self.prompt.get()
        } else {
            CLI_COMMANDS.into()
        }
    }

    pub fn copy(&mut self, ctx: &NodeContext, _event: Event<ButtonClick>) {
        if self.copying.get() {
            return;
        }
        self.copying.set(true);
        self.status.set("Copying…".into());
        let revision = self._copy_revision.get();
        let current_revision = self._copy_revision.clone();
        let status = self.status.clone();
        let copying = self.copying.clone();
        let result = ctx.write_clipboard_text(&self.active_text(), move |result| {
            if current_revision.get() != revision {
                return;
            }
            copying.set(false);
            status.set(
                match result {
                    Ok(()) => "Copied to clipboard.",
                    Err(_) => "Copy was blocked. Select the text to copy it manually.",
                }
                .into(),
            );
        });
        if result.is_err() {
            self.copying.set(false);
            self.status
                .set("Couldn't copy. Select the text to copy it manually.".into());
        }
    }

    pub fn open_codex(&mut self, ctx: &NodeContext, _event: Event<ButtonClick>) {
        ctx.navigate_to(
            &crate::ai_prompt::codex_url(crate::ai_prompt::IDEAS[self._idea_index.get()]),
            NavigationTarget::Current,
        );
        self.status
            .set("Codex opens a draft to review. No app? Copy the prompt instead.".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_is_the_default_and_matches_the_visible_commands() {
        let cta = GetStarted::default();
        assert!(!cta.ai.get());
        assert_eq!(cta.active_text(), CLI_COMMANDS);
        assert!(include_str!("get_started.pax").contains(CLI_COMMANDS));
    }

    #[test]
    fn prompt_is_one_read_only_string_for_copy_and_handoff() {
        let cta = GetStarted::default();
        cta.ai.set(true);
        for index in 0..25 {
            cta.set_idea(index);
            assert_eq!(
                cta.active_text(),
                crate::ai_prompt::prompt(crate::ai_prompt::IDEAS[index])
            );
        }
        let template = include_str!("get_started.pax");
        assert!(template.contains("selectable=true markdown=false"));
        assert!(!template.contains("<Textbox"));
    }

    #[test]
    fn new_idea_invalidates_stale_copy_feedback() {
        let cta = GetStarted::default();
        cta.status.set("Copied".into());
        cta.copying.set(true);
        let revision = cta._copy_revision.get();
        cta.set_idea(7);
        assert_ne!(cta._copy_revision.get(), revision);
        assert!(!cta.copying.get());
        assert!(cta.status.get().is_empty());
    }
}
