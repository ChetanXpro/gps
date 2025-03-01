use crate::tile::Tile;

pub struct QueryCalculations;

impl QueryCalculations {
    pub fn calculate_tile_bitmask(tile: &Tile, zoom_level_difference: u8) -> i32 {
        if zoom_level_difference == 1 {
            return Self::get_first_level_tile_bitmask(tile);
        }

        // calculate the XY numbers of the second level sub-tile
        let subtile_x = tile.tile_x >> (zoom_level_difference - 2);
        let subtile_y = tile.tile_y >> (zoom_level_difference - 2);

        // calculate the XY numbers of the parent tile
        let parent_tile_x = subtile_x >> 1;
        let parent_tile_y = subtile_y >> 1;

        // determine the correct bitmask for all 16 sub-tiles
        if parent_tile_x % 2 == 0 && parent_tile_y % 2 == 0 {
            Self::get_second_level_tile_bitmask_upper_left(subtile_x, subtile_y)
        } else if parent_tile_x % 2 == 1 && parent_tile_y % 2 == 0 {
            Self::get_second_level_tile_bitmask_upper_right(subtile_x, subtile_y)
        } else if parent_tile_x % 2 == 0 && parent_tile_y % 2 == 1 {
            Self::get_second_level_tile_bitmask_lower_left(subtile_x, subtile_y)
        } else {
            Self::get_second_level_tile_bitmask_lower_right(subtile_x, subtile_y)
        }
    }

    pub fn calculate_tile_bitmask_range(
        upper_left: &Tile,
        lower_right: &Tile,
        zoom_level_difference: u8,
    ) -> i32 {
        let mut bitmask = 0;
        for x in upper_left.tile_x..=lower_right.tile_x {
            for y in upper_left.tile_y..=lower_right.tile_y {
                let current = Tile::new(x, y, upper_left.zoom_level, upper_left.tile_size);
                bitmask |= Self::calculate_tile_bitmask(&current, zoom_level_difference);
            }
        }
        bitmask
    }

    fn get_first_level_tile_bitmask(tile: &Tile) -> i32 {
        if tile.tile_x % 2 == 0 && tile.tile_y % 2 == 0 {
            // upper left quadrant
            0xcc00
        } else if tile.tile_x % 2 == 1 && tile.tile_y % 2 == 0 {
            // upper right quadrant
            0x3300
        } else if tile.tile_x % 2 == 0 && tile.tile_y % 2 == 1 {
            // lower left quadrant
            0xcc
        } else {
            // lower right quadrant
            0x33
        }
    }

    fn get_second_level_tile_bitmask_lower_left(subtile_x: i64, subtile_y: i64) -> i32 {
        if subtile_x % 2 == 0 && subtile_y % 2 == 0 {
            // upper left sub-tile
            0x80
        } else if subtile_x % 2 == 1 && subtile_y % 2 == 0 {
            // upper right sub-tile
            0x40
        } else if subtile_x % 2 == 0 && subtile_y % 2 == 1 {
            // lower left sub-tile
            0x8
        } else {
            // lower right sub-tile
            0x4
        }
    }

    fn get_second_level_tile_bitmask_lower_right(subtile_x: i64, subtile_y: i64) -> i32 {
        if subtile_x % 2 == 0 && subtile_y % 2 == 0 {
            // upper left sub-tile
            0x20
        } else if subtile_x % 2 == 1 && subtile_y % 2 == 0 {
            // upper right sub-tile
            0x10
        } else if subtile_x % 2 == 0 && subtile_y % 2 == 1 {
            // lower left sub-tile
            0x2
        } else {
            // lower right sub-tile
            0x1
        }
    }

    fn get_second_level_tile_bitmask_upper_left(subtile_x: i64, subtile_y: i64) -> i32 {
        if subtile_x % 2 == 0 && subtile_y % 2 == 0 {
            // upper left sub-tile
            0x8000
        } else if subtile_x % 2 == 1 && subtile_y % 2 == 0 {
            // upper right sub-tile
            0x4000
        } else if subtile_x % 2 == 0 && subtile_y % 2 == 1 {
            // lower left sub-tile
            0x800
        } else {
            // lower right sub-tile
            0x400
        }
    }

    fn get_second_level_tile_bitmask_upper_right(subtile_x: i64, subtile_y: i64) -> i32 {
        if subtile_x % 2 == 0 && subtile_y % 2 == 0 {
            // upper left sub-tile
            0x2000
        } else if subtile_x % 2 == 1 && subtile_y % 2 == 0 {
            // upper right sub-tile
            0x1000
        } else if subtile_x % 2 == 0 && subtile_y % 2 == 1 {
            // lower left sub-tile
            0x200
        } else {
            // lower right sub-tile
            0x100
        }
    }
}
