use crate::header::SubFileParameter;
use crate::query_calculations::QueryCalculations;
use crate::tile::Tile;

#[derive(Debug, Clone)]
pub struct QueryParameters {
    pub from_base_tile_x: i64,
    pub from_base_tile_y: i64,
    pub from_block_x: i64,
    pub from_block_y: i64,
    pub query_tile_bitmask: i32,
    pub query_zoom_level: i32,
    pub to_base_tile_x: i64,
    pub to_base_tile_y: i64,
    pub to_block_x: i64,
    pub to_block_y: i64,
    pub use_tile_bitmask: bool,
}

impl QueryParameters {
    pub fn new() -> Self {
        Self {
            from_base_tile_x: 0,
            from_base_tile_y: 0,
            from_block_x: 0,
            from_block_y: 0,
            query_tile_bitmask: 0,
            query_zoom_level: 0,
            to_base_tile_x: 0,
            to_base_tile_y: 0,
            to_block_x: 0,
            to_block_y: 0,
            use_tile_bitmask: false,
        }
    }

    pub fn calculate_base_tiles(
        &mut self,
        upper_left: &Tile,
        lower_right: &Tile,
        sub_file_parameter: &SubFileParameter,
    ) {
        if upper_left.zoom_level < sub_file_parameter.base_zoom_level {
            let zoom_level_difference = sub_file_parameter.base_zoom_level - upper_left.zoom_level;
            self.from_base_tile_x = upper_left.tile_x << zoom_level_difference as i64;
            self.from_base_tile_y = upper_left.tile_y << zoom_level_difference as i64;
            self.to_base_tile_x = (lower_right.tile_x << zoom_level_difference as i64)
                + (1 << zoom_level_difference as i64)
                - 1;
            self.to_base_tile_y = (lower_right.tile_y << zoom_level_difference as i64)
                + (1 << zoom_level_difference as i64)
                - 1;
            self.use_tile_bitmask = false;
        } else if upper_left.zoom_level > sub_file_parameter.base_zoom_level {
            let zoom_level_difference = upper_left.zoom_level - sub_file_parameter.base_zoom_level;
            self.from_base_tile_x = upper_left.tile_x >> zoom_level_difference as i64;
            self.from_base_tile_y = upper_left.tile_y >> zoom_level_difference as i64;
            self.to_base_tile_x = lower_right.tile_x >> zoom_level_difference as i64;
            self.to_base_tile_y = lower_right.tile_y >> zoom_level_difference as i64;
            self.use_tile_bitmask = true;
            self.query_tile_bitmask = QueryCalculations::calculate_tile_bitmask_range(
                upper_left,
                lower_right,
                zoom_level_difference,
            );
        } else {
            self.from_base_tile_x = upper_left.tile_x;
            self.from_base_tile_y = upper_left.tile_y;
            self.to_base_tile_x = lower_right.tile_x;
            self.to_base_tile_y = lower_right.tile_y;
            self.use_tile_bitmask = false;
        }
    }

    pub fn calculate_base_tiles_range(
        &mut self,
        upper_left: &Tile,
        lower_right: &Tile,
        sub_file_parameter: &SubFileParameter,
    ) {
        if upper_left.zoom_level < sub_file_parameter.base_zoom_level {
            // here we need to combine multiple base tiles
            let zoom_level_difference = sub_file_parameter.base_zoom_level - upper_left.zoom_level;
            self.from_base_tile_x = upper_left.tile_x << zoom_level_difference;
            self.from_base_tile_y = upper_left.tile_y << zoom_level_difference;
            self.to_base_tile_x =
                (lower_right.tile_x << zoom_level_difference) + (1 << zoom_level_difference) - 1;
            self.to_base_tile_y =
                (lower_right.tile_y << zoom_level_difference) + (1 << zoom_level_difference) - 1;
            self.use_tile_bitmask = false;
        } else if upper_left.zoom_level > sub_file_parameter.base_zoom_level {
            // we might have more than just one base tile as we might span boundaries
            let zoom_level_difference = upper_left.zoom_level - sub_file_parameter.base_zoom_level;
            self.from_base_tile_x = upper_left.tile_x >> zoom_level_difference;
            self.from_base_tile_y = upper_left.tile_y >> zoom_level_difference;
            self.to_base_tile_x = lower_right.tile_x >> zoom_level_difference;
            self.to_base_tile_y = lower_right.tile_y >> zoom_level_difference;
            self.use_tile_bitmask = true;
            self.query_tile_bitmask = QueryCalculations::calculate_tile_bitmask_range(
                upper_left,
                lower_right,
                zoom_level_difference,
            );
        } else {
            // we are on the base zoom level, so we just need all tiles in range
            self.from_base_tile_x = upper_left.tile_x;
            self.from_base_tile_y = upper_left.tile_y;
            self.to_base_tile_x = lower_right.tile_x;
            self.to_base_tile_y = lower_right.tile_y;
            self.use_tile_bitmask = false;
        }
    }

    pub fn calculate_blocks(&mut self, sub_file_parameter: &SubFileParameter) {
        // Calculate the blocks in the file which need to be read using wrapping operations
        // to mimic Java's behavior with integer overflow

        // For from_block_x
        let from_x_diff = self
            .from_base_tile_x
            .wrapping_sub(sub_file_parameter.boundary_tile_left);
        self.from_block_x = i64::max(from_x_diff, 0);

        // For from_block_y
        let from_y_diff = self
            .from_base_tile_y
            .wrapping_sub(sub_file_parameter.boundary_tile_top);
        self.from_block_y = i64::max(from_y_diff, 0);

        // For to_block_x
        let to_x_diff = self
            .to_base_tile_x
            .wrapping_sub(sub_file_parameter.boundary_tile_left);
        let blocks_width_minus_one = sub_file_parameter.blocks_width.wrapping_sub(1);
        self.to_block_x = i64::min(to_x_diff, blocks_width_minus_one);

        // For to_block_y
        let to_y_diff = self
            .to_base_tile_y
            .wrapping_sub(sub_file_parameter.boundary_tile_top);
        let blocks_height_minus_one = sub_file_parameter.blocks_height.wrapping_sub(1);
        self.to_block_y = i64::min(to_y_diff, blocks_height_minus_one);
    }
}

impl PartialEq for QueryParameters {
    fn eq(&self, other: &Self) -> bool {
        self.from_base_tile_x == other.from_base_tile_x
            && self.from_block_x == other.from_block_x
            && self.from_base_tile_y == other.from_base_tile_y
            && self.from_block_y == other.from_block_y
            && self.query_tile_bitmask == other.query_tile_bitmask
            && self.query_zoom_level == other.query_zoom_level
            && self.to_base_tile_x == other.to_base_tile_x
            && self.to_base_tile_y == other.to_base_tile_y
            && self.to_block_x == other.to_block_x
            && self.to_block_y == other.to_block_y
            && self.use_tile_bitmask == other.use_tile_bitmask
    }
}

impl Eq for QueryParameters {}

impl std::hash::Hash for QueryParameters {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut result = 7i32;
        result = 31 * result + (self.from_base_tile_x ^ (self.from_base_tile_x >> 16)) as i32;
        result = 31 * result + (self.from_base_tile_y ^ (self.from_base_tile_y >> 16)) as i32;
        result = 31 * result + (self.to_base_tile_x ^ (self.to_base_tile_x >> 16)) as i32;
        result = 31 * result + (self.to_base_tile_y ^ (self.to_base_tile_y >> 16)) as i32;
        result = 31 * result + (self.from_block_x ^ (self.from_block_x >> 16)) as i32;
        result = 31 * result + (self.from_block_y ^ (self.from_block_y >> 16)) as i32;
        result = 31 * result + (self.to_block_x ^ (self.to_block_x >> 16)) as i32;
        result = 31 * result + (self.to_block_y ^ (self.to_block_y >> 16)) as i32;
        result = 31 * result + self.query_zoom_level;
        result = 31 * result + self.query_tile_bitmask;
        result.hash(state);
    }
}
