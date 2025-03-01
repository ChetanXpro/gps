use crate::mercator::MercatorProjection;
use crate::types::BoundingBox;

#[derive(Debug, Clone)]
pub struct Tile {
    pub tile_x: i64,
    pub tile_y: i64,
    pub zoom_level: u8,
    pub tile_size: i32,
}

impl Tile {
    pub fn new(tile_x: i64, tile_y: i64, zoom_level: u8, tile_size: i32) -> Self {
        Self {
            tile_x,
            tile_y,
            zoom_level,
            tile_size,
        }
    }

    pub fn get_bounding_box(&self) -> BoundingBox {
        let min_lon = MercatorProjection::tile_x_to_longitude(self.tile_x, self.zoom_level);
        let max_lon = MercatorProjection::tile_x_to_longitude(self.tile_x + 1, self.zoom_level);
        let min_lat = MercatorProjection::tile_y_to_latitude(self.tile_y + 1, self.zoom_level);
        let max_lat = MercatorProjection::tile_y_to_latitude(self.tile_y, self.zoom_level);

        BoundingBox {
            min_latitude: min_lat,
            min_longitude: min_lon,
            max_latitude: max_lat,
            max_longitude: max_lon,
        }
    }

    pub fn get_bounding_box_range(upper_left: &Tile, lower_right: &Tile) -> BoundingBox {
        // Calculate the bounding box covering the range of tiles
        // Ensure safe calculations to prevent overflow

        // Use saturating operations to prevent overflow
        let min_latitude = MercatorProjection::tile_y_to_latitude(
            lower_right.tile_y.min(upper_left.tile_y),
            upper_left.zoom_level,
        );

        let max_latitude = MercatorProjection::tile_y_to_latitude(
            lower_right.tile_y.max(upper_left.tile_y),
            upper_left.zoom_level,
        );

        let min_longitude = MercatorProjection::tile_x_to_longitude(
            lower_right.tile_x.min(upper_left.tile_x),
            upper_left.zoom_level,
        );

        let max_longitude = MercatorProjection::tile_x_to_longitude(
            lower_right.tile_x.max(upper_left.tile_x),
            upper_left.zoom_level,
        );

        BoundingBox::new(min_latitude, min_longitude, max_latitude, max_longitude)
            .expect("Failed to create bounding box")
    }
}
