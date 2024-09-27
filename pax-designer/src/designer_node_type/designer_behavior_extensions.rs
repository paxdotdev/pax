use pax_engine::{
    api::Color,
    math::{Generic, Transform2},
    NodeInterface, NodeLocal,
};

use crate::model::action::{Action, ActionContext};
pub mod designer_stacker_behavior;

/// This trait could at some point be exposed to users, allowing to override behavior of objects in the designer.
/// Note that some designer specific things are used here that would need to be replaced/thought about (ActionContext, etc)
pub trait DesignerComponentBehaviorExtensions {
    fn get_intents(&self, _ctx: &mut ActionContext, _node: &NodeInterface) -> IntentState {
        IntentState::default()
    }
}

#[derive(Default)]
pub struct IntentState {
    pub intent_areas: Vec<IntentDefinition>,
}

pub struct IntentDefinition {
    pub area: Transform2<NodeLocal>,
    pub fill: Color,
    pub stroke: Option<(f64, Color)>,
    pub intent_drop_behavior_factory: Box<dyn Fn(&[NodeInterface]) -> Box<dyn Action>>,
}
