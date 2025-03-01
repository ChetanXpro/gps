pub struct MercatorProjection;

// Constants
const EARTH_RADIUS: f64 = 6_378_137.0;
const EARTH_CIRCUMFERENCE: f64 = 40075016.686;
const LATITUDE_MAX: f64 = 85.05112877980659;
const LATITUDE_MIN: f64 = -LATITUDE_MAX;
const TILE_SIZE: i32 = 256; // Standard tile size
const PI: f64 = std::f64::consts::PI;

impl MercatorProjection {
    // Your existing methods can stay the same
    pub fn tile_x_to_longitude(tile_x: i64, zoom_level: u8) -> f64 {
        let n = 1i64 << zoom_level;
        (tile_x as f64 * 360.0 / n as f64) - 180.0
    }

    pub fn tile_y_to_latitude(tile_y: i64, zoom_level: u8) -> f64 {
        let n = 1i64 << zoom_level;
        let y = 0.5 - (tile_y as f64 / n as f64);
        90.0 - 360.0 * ((-y * (2.0 * PI)).exp().atan()) / PI
    }

    pub fn longitude_to_tile_x(longitude: f64, zoom_level: u8) -> i64 {
        let n = 1i64 << zoom_level;
        ((longitude + 180.0) / 360.0 * n as f64).floor() as i64
    }

    pub fn latitude_to_tile_y(latitude: f64, zoom_level: u8) -> i64 {
        // Clamp latitude to valid range
        let latitude = latitude.max(LATITUDE_MIN).min(LATITUDE_MAX);

        let n = 1i64 << zoom_level;

        // Use a more stable formula that handles latitude=0 correctly
        let lat_rad = latitude.to_radians();

        // Mercator projection formula
        let y = 0.5 - (lat_rad.sin().atanh() / (2.0 * PI));

        // Handle potential numerical issues near the poles
        let tile_y = (y * n as f64).floor() as i64;

        // Ensure result is within valid range
        tile_y.clamp(0, n - 1)
    }

    // Use TILE_SIZE instead of passing it as parameter if not needed
    pub fn latitude_to_pixel_y(latitude: f64, zoom_level: u8) -> f64 {
        let map_size = Self::get_map_size(zoom_level);
        let sin_latitude = latitude.to_radians().sin();
        let pixel_y = (0.5 - ((1.0 + sin_latitude) / (1.0 - sin_latitude)).ln() / (4.0 * PI))
            * map_size as f64;
        pixel_y.min(map_size as f64).max(0.0)
    }

    pub fn longitude_to_pixel_x(longitude: f64, zoom_level: u8) -> f64 {
        let map_size = Self::get_map_size(zoom_level);
        (longitude + 180.0) / 360.0 * map_size as f64
    }

    pub fn get_map_size(zoom_level: u8) -> i64 {
        if zoom_level as i32 >= 0 {
            (TILE_SIZE as i64) << zoom_level
        } else {
            0
        }
    }

    // Your other methods remain the same
    pub fn meters_per_pixel(latitude: f64, zoom_level: u8) -> f64 {
        let lat_rad = latitude.to_radians();
        let circumference = 2.0 * PI * EARTH_RADIUS * lat_rad.cos();
        let distance_per_tile = circumference / (1u32 << zoom_level) as f64;
        distance_per_tile / TILE_SIZE as f64
    }

    pub fn tile_count(zoom_level: u8) -> i64 {
        1i64 << zoom_level
    }
}
