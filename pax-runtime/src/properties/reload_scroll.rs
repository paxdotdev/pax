//! Keep a reading position across a template remount, not across app navigation.
use super::*;
use pax_runtime_api::PaxValue;

#[derive(Hash, PartialEq, Eq)]
enum Site {
    Named(pax_manifest::TypeId, String),
    Anonymous(UniqueTemplateNodeIdentifier),
}

// The same declaration can occur under different component instances or repeat
// keys. Never use a freshly allocated expanded-node id to match a replacement.
type Path = Vec<(Site, Option<String>)>;

fn path(node: &Rc<ExpandedNode>) -> Option<Path> {
    let mut result = Vec::new();
    let mut current = Some(node.clone());
    while let Some(n) = current {
        let instance = n.instance_node.borrow();
        if let Some(source) = &instance.base().template_node_identifier {
            let site = match n.common_properties.borrow().borrow().id.get() {
                Some(id) => Site::Named(source.get_containing_component_type_id(), id),
                None => Site::Anonymous(source.clone()),
            };
            let repeat_key =
                n.stack
                    .resolve_symbol("@repeat_key")
                    .and_then(|v| match v.get_as_pax_value() {
                        PaxValue::String(key) => Some(key),
                        _ => None,
                    });
            result.push((site, repeat_key));
        }
        current = n.containing_component.upgrade();
    }
    (!result.is_empty()).then_some(result)
}

fn unique_scrollers<T>(
    ctx: &RuntimeContext,
    mut value: impl FnMut(Rc<ExpandedNode>, &MountedScroller) -> T,
) -> HashMap<Path, Option<T>> {
    let mut out = HashMap::new();
    for (&id, state) in ctx.scroller_surface_states.borrow().iter() {
        let Some(node) = ctx.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) else {
            continue;
        };
        let Some(key) = path(&node) else { continue };
        // Unkeyed copies of the same component can be indistinguishable.
        out.entry(key)
            .and_modify(|slot| *slot = None)
            .or_insert_with(|| Some(value(node, state)));
    }
    out
}

struct Bookmark {
    old_mount: (ExpandedNodeIdentifier, u64),
    initial: [f64; 2],
    offset: [f64; 2],
}
pub(crate) struct ScrollBookmarks(HashMap<Path, Option<Bookmark>>);

impl ScrollBookmarks {
    pub(crate) fn capture(ctx: &RuntimeContext) -> Self {
        Self(unique_scrollers(ctx, |node, mounted| Bookmark {
            old_mount: (node.id, node.reference_epoch.get()),
            initial: mounted.initial_offset,
            offset: [mounted.state.scroll_x, mounted.state.scroll_y],
        }))
    }

    pub(crate) fn restore(self, ctx: &RuntimeContext) {
        let mounted = unique_scrollers(ctx, |node, m| (node, m.initial_offset));
        let mut changed = false;
        for (path, bookmark) in self.0 {
            let (Some(saved), Some(Some((node, initial)))) = (bookmark, mounted.get(&path)) else {
                continue;
            };
            if saved.old_mount == (node.id, node.reference_epoch.get()) {
                continue;
            }
            let scope = node.properties_scope.borrow();
            for (axis, name) in ["scroll_pos_x", "scroll_pos_y"].into_iter().enumerate() {
                // Respect an edit to the authored starting position on either axis.
                let offset = saved.offset[axis];
                if initial[axis] != saved.initial[axis] || !offset.is_finite() {
                    continue;
                }
                if let Some(var) = scope.get(name) {
                    // Use the logical property (including its two-way binding),
                    // never the projected/tile offset or a simulated host event.
                    changed |=
                        LocalProperty::<f64>::new_from_untyped(var.get_untyped_property().clone())
                            .set_if_neq(offset);
                }
            }
        }
        if changed {
            ctx.drain_node_effects();
        }
    }
}
