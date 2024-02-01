


pub struct ActionManager {}

// "central hub" of actions.
// often triggered by FSM / input manager
// Can perform selection, can edit ORM
// Tracks undo stack for every action (including selection)
// Can reach into engine to perform ray-casting (and likely other) operations
//

// Decide: enum or trait for actions?
//  The action itself could either be inside a large match statement (enum)
//  or an impl fn on the trait

// Some known actions:
// ResolveTargets (under mouse coordinates, requires raycasting)
// PerformSelection
// ClearSelection

// Regarding undo, some actions may need to be batched — e.g. deselecting and then
// selecting elements.  The alternative would be to have atomic actions (DeselectAllAndSelect) that
// manage multiple logical things.
// Seems cleaner to start with the latter, and possibly to extend the undo mechanism to handle multiple actions
// at some future time.