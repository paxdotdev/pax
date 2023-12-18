use pax_manifest::PaxManifest;

pub mod handlers;
pub mod settings;
pub mod template;

pub trait Request {
    type Response: Response;
}

pub trait Response {
    fn set_id(&mut self, id: usize);
}

pub trait Command<R: Request> {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<R::Response, String>;
    fn is_mutative(&mut self) -> bool {
        return true;
    }
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String>;
    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String>;
}

pub struct PaxManifestORM<R: Request> {
    manifest: PaxManifest,
    undo_stack: Vec<Box<dyn Command<R>>>,
    redo_stack: Vec<Box<dyn Command<R>>>,
    next_command_id: usize,
}

impl<R: Request> PaxManifestORM<R> {
    pub fn execute_command(
        &mut self,
        mut command: Box<dyn Command<R>>,
    ) -> Result<R::Response, String> {
        let mut response = command.execute(&mut self.manifest)?;
        if command.is_mutative() {
            self.undo_stack.push(command);
            self.redo_stack.clear();
        }
        response.set_id(self.next_command_id);
        self.next_command_id += 1;
        Ok(response)
    }

    pub fn undo(&mut self) -> Result<(), String> {
        if let Some(mut command) = self.undo_stack.pop() {
            command.undo(&mut self.manifest)?;
            self.redo_stack.push(command);
        }
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), String> {
        if let Some(mut command) = self.redo_stack.pop() {
            command.redo(&mut self.manifest)?;
            self.undo_stack.push(command);
        }
        Ok(())
    }
}

// // Template Node Operations
// pub fn add_template_node(&mut self, node: TemplateNodeDefinition) { /* implementation */ }
// pub fn delete_template_node(&mut self, node_id: usize) { /* implementation */ }
// pub fn update_template_node(&mut self, node: TemplateNodeDefinition) { /* implementation */ }
// pub fn get_template_node(&self, node_id: usize) -> Option<&TemplateNodeDefinition> { /* implementation */ }
// pub fn get_all_template_nodes(&self) -> Vec<&TemplateNodeDefinition> { /* implementation */ }

// // Settings Block Operations (Selector)
// pub fn add_selector_block(&mut self, selector: SettingsBlockElement) { /* implementation */ }
// pub fn delete_selector_block(&mut self, selector_id: usize) { /* implementation */ }
// pub fn update_selector_block(&mut self, selector: SettingsBlockElement) { /* implementation */ }
// pub fn get_selector_block(&self, selector_id: usize) -> Option<&SettingsBlockElement> { /* implementation */ }
// pub fn get_all_selectors(&self) -> Vec<&SettingsBlockElement> { /* implementation */ }

// // Handler Block Operations
// pub fn add_handler_block(&mut self, handler: HandlersBlockElement) { /* implementation */ }
// pub fn delete_handler_block(&mut self, handler_id: usize) { /* implementation */ }
// pub fn update_handler_block(&mut self, handler: HandlersBlockElement) { /* implementation */ }
// pub fn get_handler_block(&self, handler_id: usize) -> Option<&HandlersBlockElement> { /* implementation */ }
// pub fn get_all_handlers(&self) -> Vec<&HandlersBlockElement> { /* implementation */ }
