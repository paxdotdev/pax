use crate::{AtlasStore, LayerState, LineView, StationView, TrainView};
use pax_kit::*;

// The metadata repeat stays unchanged. Each mounted layer publishes only its
// own revision, so changing a line cannot invalidate the entire network list.
// Mount registers shared property handles with the atlas; one frame handler
// publishes changes without scheduling a separate tick for every layer.
#[pax]
#[file("route_paths.pax")]
pub struct RoutePaths {
    pub line: Property<LineView>,
    pub elements: Property<Vec<PathElement>>,
    pub line_alpha: Property<f64>,
    pub stroke_width: Property<f64>,
    pub casing_width: Property<f64>,
    pub revision: Property<u64>,
}
impl RoutePaths {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let id = self.line.get().id;
        ctx.with_store(|s: &mut AtlasStore| {
            s.layers[id].path_view = Some(self.clone());
            s.publish_layer(id);
        })
        .expect("route layer needs the atlas store");
    }
    pub(crate) fn publish(&self, layer: &LayerState, alpha: f64, scale: f64, isolated: bool) {
        if self.revision.get() != layer.path_revision {
            self.elements.set(layer.paths.clone());
            self.revision.set(layer.path_revision);
        }
        self.line_alpha.set_if_neq(alpha);
        self.stroke_width.set_if_neq(if isolated {
            12_f64.max(4.8 / scale)
        } else if self.line.get().id == 13 {
            8.
        } else {
            13.
        });
        self.casing_width.set_if_neq(if isolated {
            18_f64.max(7. / scale)
        } else if self.line.get().id == 13 {
            14.
        } else {
            19.
        });
    }
}

#[pax]
#[file("route_stations.pax")]
pub struct RouteStations {
    pub line: Property<LineView>,
    pub stations: Property<Vec<StationView>>,
    pub line_alpha: Property<f64>,
    pub isolated: Property<bool>,
    pub revision: Property<u64>,
}
impl RouteStations {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let id = self.line.get().id;
        ctx.with_store(|s: &mut AtlasStore| {
            s.layers[id].station_view = Some(self.clone());
            s.publish_layer(id);
        })
        .expect("route layer needs the atlas store");
    }
    pub(crate) fn publish(&self, layer: &LayerState, alpha: f64, isolated: bool) {
        if self.revision.get() != layer.station_revision {
            self.stations.set(layer.stations.clone());
            self.revision.set(layer.station_revision);
        }
        self.line_alpha.set_if_neq(alpha);
        self.isolated.set_if_neq(isolated);
    }
}

#[pax]
#[file("route_trains.pax")]
pub struct RouteTrains {
    pub line: Property<LineView>,
    pub trains: Property<Vec<TrainView>>,
    pub selected_train: Property<i64>,
    pub line_alpha: Property<f64>,
    pub train_width: Property<f64>,
    pub train_height: Property<f64>,
    pub stroke_width: Property<f64>,
    pub revision: Property<u64>,
}
impl RouteTrains {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let id = self.line.get().id;
        ctx.with_store(|s: &mut AtlasStore| {
            s.layers[id].train_view = Some(self.clone());
            s.publish_layer(id);
        })
        .expect("route layer needs the atlas store");
    }
    pub(crate) fn publish(&self, layer: &LayerState, alpha: f64, scale: f64, isolated: bool) {
        if self.revision.get() != layer.train_revision {
            self.trains.set(layer.trains.clone());
            self.revision.set(layer.train_revision);
        }
        self.line_alpha.set_if_neq(alpha);
        self.train_width
            .set_if_neq(if isolated { 18. / scale } else { 24. });
        self.train_height
            .set_if_neq(if isolated { 8. / scale } else { 10. });
        self.stroke_width
            .set_if_neq(if isolated { 1.3 / scale } else { 2.2 });
    }
}
