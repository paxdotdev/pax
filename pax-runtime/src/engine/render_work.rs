use super::{surface_work::*, *};

impl PaxEngine {
    /// Service complete rendering units from the current settled scene. No node
    /// list, render pass or property-graph guard survives this call. A chassis may
    /// drain synchronously or resume in a later host task, but must call
    /// `RenderWorkState::begin` after each ordinary tick before resuming drawing.
    /// Unsupported/unclippable paths remain one explicit atomic fallback.
    pub fn service_render_work(
        &mut self,
        rcs: &mut dyn RenderContext,
        state: &mut RenderWorkState,
        budget: &mut SurfaceWorkBudget,
        slice_ms: f64,
        now_ms: impl Fn() -> f64,
    ) -> RenderWorkReport {
        let start = now_ms();
        let mut report = RenderWorkReport::default();
        if matches!(
            self.runtime_context.application.context().phase(),
            pax_runtime_api::AppPhase::Closing | pax_runtime_api::AppPhase::Closed
        ) {
            report.blocked = true;
            report.blocker = Some("app_closing");
            return report;
        }
        let _graph = self.runtime_context.property_graph.enter();
        // Geometry may need the ordinary frame's setup, but reactive/occlusion
        // settlement is never smuggled into a continuation. After setup, even a
        // geometry-only change requires the next normal settlement opportunity.
        if let Some(blocker) = self.runtime_context.settled_surface_blocker() {
            if blocker != "geometry" || state.setup_complete {
                report.blocked = true;
                report.blocker = Some(blocker);
                return report;
            }
        }
        if !state.setup_complete {
            let setup = now_ms();
            let (layers, _, flush_layers) = self.reconcile_render_work(rcs, &mut |_| {});
            state.pending_lighting_flushes.extend(
                flush_layers
                    .into_iter()
                    .filter(|layer| !layers.contains(layer)),
            );
            state
                .pending_lighting_flushes
                .retain(|layer| *layer < self.runtime_context.layer_count.get());
            // Removals-only layers also remain authoritative pending work until
            // their complete draw/flush attempt. No copied work queue is kept.
            for layer in layers {
                self.runtime_context.set_canvas_dirty(layer);
            }
            state.setup_complete = true;
            report.max_unit_ms = now_ms() - setup;
            if now_ms() - start >= slice_ms {
                report.pending = true;
                report.elapsed_ms = now_ms() - start;
                return report;
            }
        }

        loop {
            if let Some(blocker) = self.runtime_context.settled_surface_blocker() {
                report.blocked = true;
                report.blocker = Some(blocker);
                break;
            }
            let unit_start = now_ms();
            // Finish visible surfaces before selecting ordinary or speculative
            // work. One tile (including missing captures) is atomic: a CPU
            // overrun is reported, never presented as a half-built replacement.
            let next = self.present_surfaces(rcs, budget, &now_ms, 1, &state.preparation, true);
            let prepared = next.initializations + next.submitted > 0;
            report.preparation.accumulate(next.clone());
            state.preparation.accumulate(next);
            if prepared {
                state.urgent_complete = false;
                report.units += 1;
            } else {
                let mut layers: Vec<_> = self
                    .runtime_context
                    .dirty_canvas_layers()
                    .into_iter()
                    .chain(state.pending_lighting_flushes.iter().copied())
                    .filter(|l| !state.attempted_layers.contains(l))
                    .collect();
                layers.sort_by_key(|l| (rcs.ordinary_layer_priority(*l).unwrap_or(i32::MAX), *l));
                layers.dedup();
                let visible = layers
                    .first()
                    .is_some_and(|l| rcs.ordinary_layer_priority(*l).is_some_and(|r| r <= 0));
                // Always offer one host boundary between urgent and optional
                // work, even if urgent work happened to complete very cheaply.
                if !visible && !state.urgent_complete {
                    state.urgent_complete = true;
                    // A previous slice already returned to the host. With no
                    // urgent work in this call, don't schedule another input/
                    // layout synchronization merely to discover idle work.
                    if report.units > 0 {
                        report.pending = true;
                        break;
                    }
                }
                if let Some(&layer) = layers.first() {
                    if visible {
                        state.urgent_complete = false;
                    }
                    let lighting_only = !self.runtime_context.is_canvas_dirty(&layer);
                    let nodes = if lighting_only {
                        Vec::new()
                    } else {
                        self.runtime_context.dirty_canvas_node_ids()
                    };
                    let separable = lighting_only
                        || (rcs.retains_canvas_nodes()
                            && rcs.ordinary_layer_priority(layer).is_some()
                            && (nodes.is_empty()
                                || self.root_expanded_node.as_ref().is_some_and(|root| {
                                    self.build_filtered_render_plan(root, &nodes).is_ok()
                                })));
                    if !separable {
                        // Do not weaken inherited masks/groups/unclippable
                        // semantics to obtain a smaller scheduling unit.
                        // The synchronous fallback flushes dirty content layers;
                        // clean lighting-only layers still need their own turn.
                        let dirty = self.runtime_context.dirty_canvas_layers();
                        state.attempted_layers.extend(dirty.iter().copied());
                        for layer in dirty {
                            state.pending_lighting_flushes.remove(&layer);
                        }
                        self.render_with_callbacks(rcs, false, &mut |_| {}, &mut |_, _, _| {});
                        report.atomic_fallbacks += 1;
                    } else {
                        let selected: Vec<_> = nodes
                            .into_iter()
                            .filter(|id| {
                                self.runtime_context
                                    .get_expanded_node_by_eid(*id)
                                    .is_some_and(|n| n.occlusion.get().render_layer_id == layer)
                            })
                            .collect();
                        self.draw_canvas_nodes(rcs, &selected);
                        self.runtime_context.dirty_canvases.borrow_mut()[layer] = false;
                        if self
                            .runtime_context
                            .has_canvas_node_removals_on_layer(layer)
                        {
                            self.runtime_context.set_canvas_dirty(layer);
                        }
                        rcs.flush(layer, Rc::clone(&self.runtime_context.dirty_canvases));
                        state.pending_lighting_flushes.remove(&layer);
                        state.attempted_layers.insert(layer);
                        report.ordinary_layers += 1;
                    }
                    report.units += 1;
                } else {
                    let next = self.present_surfaces(
                        rcs,
                        budget,
                        &now_ms,
                        i32::MAX,
                        &state.preparation,
                        true,
                    );
                    let prepared = next.initializations + next.submitted > 0;
                    report.preparation.accumulate(next.clone());
                    state.preparation.accumulate(next);
                    if !prepared {
                        let completed = state.preparation.initializations
                            + state.preparation.recorded
                            + state.preparation.submitted;
                        report.allowance_exhausted =
                            !budget.admits(completed, state.preparation.elapsed_ms);
                        report.blocked = self.runtime_context.has_canvas_render_work()
                            || !state.pending_lighting_flushes.is_empty()
                            || state.preparation.deferred > 0;
                        break;
                    }
                    report.units += 1;
                }
            }
            report.max_unit_ms = report.max_unit_ms.max(now_ms() - unit_start);
            if now_ms() - start >= slice_ms {
                report.pending = true;
                break;
            }
        }
        report.elapsed_ms = now_ms() - start;
        report
    }
}
