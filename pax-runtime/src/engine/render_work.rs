use super::{surface_work::*, *};

impl PaxEngine {
    pub(super) fn build_layer_render_plan(
        &self,
        layer: usize,
    ) -> Result<Option<FilteredRenderPlan>, &'static str> {
        let mut selected = Vec::new();
        for id in self.runtime_context.dirty_canvas_node_ids() {
            let node = self
                .runtime_context
                .get_expanded_node_by_eid(id)
                .ok_or("missing_dirty_node")?;
            if node.occlusion.get().render_layer_id == layer {
                selected.push(id);
            }
        }
        if selected.is_empty() {
            // A removal-only layer still needs submission, but no traversal.
            return Ok(None);
        }
        let root = self.root_expanded_node.as_ref().ok_or("missing_root")?;
        self.build_filtered_render_plan(root, &selected).map(Some)
    }

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
        let diagnostics = self.runtime_context.occlusion_state.borrow().diagnostics;
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
            let dirty_before = diagnostics.then(|| self.runtime_context.dirty_canvas_layers());
            let (layers, _, flush_layers) = self.reconcile_render_work(rcs, &mut |_| {}, false);
            if let Some(dirty_before) = dirty_before {
                let mut counts = HashMap::<usize, usize>::new();
                for id in self.runtime_context.dirty_canvas_node_ids() {
                    if let Some(node) = self.runtime_context.get_expanded_node_by_eid(id) {
                        *counts
                            .entry(node.occlusion.get().render_layer_id)
                            .or_default() += 1;
                    }
                }
                report.layer_reconciliation = layers
                    .iter()
                    .take(32)
                    .map(|layer| {
                        (
                            *layer,
                            dirty_before.contains(layer),
                            counts.get(layer).copied().unwrap_or(0),
                        )
                    })
                    .collect();
            }
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
            report.setup_ms = report.max_unit_ms;
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
                    let ordinary_start = now_ms();
                    if visible {
                        state.urgent_complete = false;
                    }
                    let lighting_only = !self.runtime_context.is_canvas_dirty(&layer);
                    // Validate only this unit, then consume that exact plan.
                    // Unrelated warm layers need no ancestor walks here; their
                    // current paths are checked when they are actually selected.
                    let plan = if lighting_only {
                        Ok(None)
                    } else if rcs.retains_canvas_nodes()
                        && rcs.ordinary_layer_priority(layer).is_some()
                    {
                        self.build_layer_render_plan(layer)
                    } else {
                        Err("inseparable_layer")
                    };
                    if let Ok(plan) = plan {
                        let draws_content = plan.is_some();
                        let traced_nodes = diagnostics.then(|| {
                            plan.as_ref()
                                .map(|plan| plan.dirty_nodes.iter().copied().collect::<Vec<_>>())
                                .unwrap_or_default()
                        });
                        if let Some(plan) = plan {
                            self.draw_filtered_render_plan(
                                rcs,
                                self.root_expanded_node.as_ref().unwrap(),
                                &plan,
                            );
                        }
                        self.runtime_context.recurse_flush_queued_renders(rcs);
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
                        if let Some(nodes) = traced_nodes {
                            if report.layer_draws.len() < 32 {
                                report.layer_draws.push((
                                    layer,
                                    rcs.ordinary_layer_priority(layer),
                                    nodes.len(),
                                    nodes
                                        .iter()
                                        .filter(|id| self.runtime_context.is_canvas_node_dirty(id))
                                        .count(),
                                ));
                            }
                        }
                        report.ordinary_layers += 1;
                        if draws_content {
                            report.ordinary_drawn_layers += 1;
                            report.ordinary_drawn_ms += now_ms() - ordinary_start;
                        } else {
                            report.ordinary_flush_only_layers += 1;
                            report.ordinary_flush_only_ms += now_ms() - ordinary_start;
                        }
                    } else {
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
                    }
                    report.units += 1;
                    report.ordinary_ms += now_ms() - ordinary_start;
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
