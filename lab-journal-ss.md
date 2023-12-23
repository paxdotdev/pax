# Sams Lab Journal

### 23 Dec, 2023: "ss/towards-dirty-dag" branch state and slot reintroduction.

The towards-dirty-dag branch works well for arbitrary nesting of for, if and
many other components/primitives, but is missing slots, some Z-index stuff, and scroller/
clipping. Overall I'm positive that this partial re-expansion approach is a
good one and should pave the way for complete dirty dag.

Some general changes and design choices:

 - ExpandedNode has recieved a lot more responsibilites. Most logic for
   modifying the tree, handling updates and rendering has been moved here.
	 The thought is that operations like "set_children" naturally should set either
	 themselves or their containing component as their new containing component, and
	 their parent to themselves.
 - Nodes are expanded immediately on creation, ie ExpandedNode::new creates an entire tree
 - Only the "recurse_update" method is expected to be called on the root ExpandedNode each
   frame. This both updates properties and re-expands subtrees iff the properties listened
	 to on a node-by-node basis needs it! (only if and for, so far, soon slot).
 - ExpandedNode has been moved to a separate module and the ExpandedNode
   children field has intentionally been set to private. The thought is
   that all modifications to be encapsulated though methods on the ExpandedNode struct.

If you'd like to understand it better I recommend to first look at the methods
in ExpandedNode, and then on for example in conditional.rs "recompute_children
and update methods.

Below are some thoughs around how to reintroduce slot when the Expanded node
tree isn't re-expanded each frame, but sub-trees are instead surgically
updated when a property that requires an update triggers.

Suppose this is the state of the ExpandedNode tree: (v chain denoting that the
stacker owns this sepparate "shadow" tree disconnected from the main one).

 root:  ExpandedNode (ComponentInstance)
 │
 │
 └───── stacker: ExpandedNode( ComponentInstance)
        │
        │
        └─── ExpandedNode (Repeat) ...
        v    │
        v    │
        v    └────────── ExpandedNode (SlotInstance) INDEX: 3
        v
        v
        stacker: shadowtree (from which slot_children are derived)
        │ 
        └──── ExpandedNode (RectangleInstance)
The           │        
Shadow tree   │
Is updated    ExpandedNode (RepeatInstance) for 0..4
by the stac-  │  
er each frame │   
(ie reacts to └───── ExpandedNode (Rectangle)
changes in
properties and possibly re-expands children.
This tree needs to NOT fire mount or dissmount
or native patch events.

However when a part of this shadow tree is mounted in a slot, mount needs to be
fired and updates triggered from the stacker shadow tree needs to send native
patches. Mount ALSO needs to be fired when the stacker updates the slot_children tree.

Current thought: introduce an "attached" property to ExpandedNode that is set
to true recursively when children are attached and it's parent is attached.
This allows for the shadow tree to update without firing events (only fire
if attached), without effecting the expansion of sub-trees that are active
(attached).

Something to think about: What is the expected behaviour if two slots reference
the same ExpandedNode? (ie slot(0) exists twice in a component). Long term it might
be nice to allow a component to for example be rendered twice with the same underlying
ExpandedNode. What would this require? Or maybe just dissallow this.
