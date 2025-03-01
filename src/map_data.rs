use crate::types::{LatLong, Tag};

#[derive(Debug, Clone)]
pub struct PointOfInterest {
    pub layer: i8,
    pub tags: Vec<Tag>,
    pub position: LatLong,
}

impl PointOfInterest {
    pub fn new(layer: i8, tags: Vec<Tag>, position: LatLong) -> Self {
        Self {
            layer,
            tags,
            position,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Way {
    pub layer: i8,
    pub tags: Vec<Tag>,
    pub way_nodes: Vec<Vec<LatLong>>, // Equivalent to LatLong[][] in Java
    pub label_position: Option<LatLong>,
}

impl Way {
    pub fn new(
        layer: i8,
        tags: Vec<Tag>,
        way_nodes: Vec<Vec<LatLong>>,
        label_position: Option<LatLong>,
    ) -> Self {
        Self {
            layer,
            tags,
            way_nodes,
            label_position,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct PoiWayBundle {
    pub pois: Vec<PointOfInterest>,
    pub ways: Vec<Way>,
}

impl PoiWayBundle {
    pub fn new(pois: Vec<PointOfInterest>, ways: Vec<Way>) -> Self {
        Self { pois, ways }
    }
}

#[derive(Debug, Default, Clone)]
pub struct MapReadResult {
    pub poi_way_bundles: Vec<PoiWayBundle>,
    pub is_water: bool,
}

impl MapReadResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, bundle: PoiWayBundle) {
        self.poi_way_bundles.push(bundle);
    }
}
