#![allow(unused_imports)]
use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("todo_list.pax")]
pub struct TodoList {
    pub tasks: Property<Vec<Task>>,
    pub new_task: Property<String>,
    pub task_count: Property<usize>,
}

#[pax]
pub struct Task {
    pub id: usize,
    pub description: String,
    pub completed: bool,
}

impl TodoList {
    pub fn add_task(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        let mut tasks = self.tasks.get();
        let new_task = self.new_task.get();
        if !new_task.trim().is_empty() {
            tasks.push(Task {
                id: tasks.len(),
                description: new_task,
                completed: false,
            });
            self.tasks.set(tasks);
            self.new_task.set(String::new());
            self.update_task_count();
        }
    }

    pub fn toggle_task(&mut self, ctx: &NodeContext, args: Event<CheckboxChange>) {
        if let Some(id) = ctx.slot_index.get() {
            let mut tasks = self.tasks.get();
            if let Some(task) = tasks.get_mut(id) {
                task.completed = args.checked;
                self.tasks.set(tasks);
            }
        }
    }

    pub fn remove_task(&mut self, ctx: &NodeContext, _args: Event<ButtonClick>) {
        if let Some(id) = ctx.slot_index.get() {
            let mut tasks = self.tasks.get();
            tasks.retain(|task| task.id != id);
            self.tasks.set(tasks);
            self.update_task_count();
        }
    }

    pub fn update_new_task(&mut self, _ctx: &NodeContext, args: Event<TextboxChange>) {
        self.new_task.set(args.text.clone());
    }

    fn update_task_count(&mut self) {
        let count = self.tasks.get().len();
        self.task_count.set(count);
    }
}