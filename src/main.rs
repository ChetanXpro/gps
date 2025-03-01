use minifb::{Key, Window, WindowOptions};
use reader::{
    MapFile,
    MapReadResult, // This should now consistently refer to one type
    MercatorProjection,
    PoiWayBundle, // Same here
    Tile,
};
use std::cmp::max;
use std::cmp::min;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

const WINDOW_WIDTH: usize = 800;
const WINDOW_HEIGHT: usize = 600;
const INITIAL_ZOOM_LEVEL: u8 = 14;
const TILE_SIZE: usize = 256;

// Initial view center coordinates
const INITIAL_LAT: f64 = 26.7428831;
const INITIAL_LON: f64 = 93.9074701;

// Cache structure for map data
struct TileCache {
    tile_x: i64,
    tile_y: i64,
    zoom: u8,
    data: reader::MapReadResult,
}

struct RenderState {
    width: usize,
    height: usize,
    center_lat: f64,
    center_lon: f64,
    zoom: u8,
    map_data: reader::MapReadResult,
    way_styles: HashMap<String, (u32, u8)>,
    area_styles: HashMap<String, u32>,
}

struct MapRenderer {
    window: Window,
    buffer: Vec<u32>,
    map_file: MapFile,
    center_lat: f64,
    center_lon: f64,
    zoom: u8,
    way_styles: HashMap<String, (u32, u8)>, // (color, width)
    area_styles: HashMap<String, u32>,      // color for filled areas
    tile_cache: Option<TileCache>,
    last_frame_time: Instant,
    frame_count: usize,
}

impl MapRenderer {
    fn new(map_path: &Path) -> Result<Self, String> {
        // Initialize minifb window
        let mut window = Window::new(
            "MapForge Renderer",
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            WindowOptions {
                resize: true,
                ..WindowOptions::default()
            },
        )
        .map_err(|e| e.to_string())?;

        // Limit to max ~60 fps
        window.limit_update_rate(Some(Duration::from_micros(16600)));

        // Create a buffer to draw into
        let buffer = vec![0; WINDOW_WIDTH * WINDOW_HEIGHT];

        // Open map file
        let map_file = MapFile::new(map_path.to_str().unwrap())
            .map_err(|e| format!("Failed to open map file: {}", e))?;

        // Define road styles (color, width)
        let mut way_styles = HashMap::new();
        way_styles.insert("highway=path".to_string(), (0x00CC5500, 2)); // Brown for hiking paths
        way_styles.insert("highway=track".to_string(), (0x00996600, 2)); // Darker brown for tracks
        way_styles.insert("highway=footway".to_string(), (0x00CC5500, 1)); // Also brown for footways
        way_styles.insert("waterway=river".to_string(), (0x0033AAFF, 3)); // Brighter blue for rivers
        way_styles.insert("waterway=stream".to_string(), (0x0033AAFF, 2)); // Blue for streams
        way_styles.insert("highway=trunk".to_string(), (0x00FF6600, 5)); // Orange for trunk roads
        way_styles.insert("highway=secondary".to_string(), (0x00FFAA00, 4)); // Yellow for secondary roads

        // Define area styles
        let mut area_styles = HashMap::new();
        area_styles.insert("natural=water".to_string(), 0x0099CCFF); // Brighter blue for water
        area_styles.insert("natural=sea".to_string(), 0x0077AAEE); // Slightly darker blue for sea
        area_styles.insert("area=yes natural=sea".to_string(), 0x0077AAEE); // Some maps use this combo
        area_styles.insert("landuse=forest".to_string(), 0x0089C283); // Greener forest
        area_styles.insert("natural=wood".to_string(), 0x0089C283); // Alternative forest tag
        area_styles.insert("landuse=quarry".to_string(), 0x00C5C5C5); // Light gray for quarries
        area_styles.insert("landuse=industrial".to_string(), 0x00DBDBDB); // Darker gray for industrial
        area_styles.insert("natural=nosea".to_string(), 0x00F0F0E8); // Off-white for land

        Ok(MapRenderer {
            window,
            buffer,
            map_file,
            center_lat: INITIAL_LAT,
            center_lon: INITIAL_LON,
            zoom: INITIAL_ZOOM_LEVEL,
            way_styles,
            area_styles,
            tile_cache: None,
            last_frame_time: Instant::now(),
            frame_count: 0,
        })
    }

    // Function to prepare rendering state without borrowing conflicts
    fn prepare_render_state(&mut self) -> Result<RenderState, String> {
        // Get the current window dimensions
        let (width, height) = self.window.get_size();

        // Resize buffer if needed
        if width * height != self.buffer.len() {
            self.buffer = vec![0; width * height];
        }

        // Clear the buffer (light gray background)
        for pixel in self.buffer.iter_mut() {
            *pixel = 0x00F0F0F0;
        }

        // Calculate current tile
        let tile_x = MercatorProjection::longitude_to_tile_x(self.center_lon, self.zoom);
        let tile_y = MercatorProjection::latitude_to_tile_y(self.center_lat, self.zoom);

        // Check if we have this tile cached
        let map_data = if let Some(cache) = &self.tile_cache {
            if cache.tile_x == tile_x && cache.tile_y == tile_y && cache.zoom == self.zoom {
                // Use cached data
                cache.data.clone()
            } else {
                // Need to load new data
                self.load_new_tile(tile_x, tile_y)?
            }
        } else {
            // First tile load
            self.load_new_tile(tile_x, tile_y)?
        };

        // Create and return the render state
        Ok(RenderState {
            width,
            height,
            center_lat: self.center_lat,
            center_lon: self.center_lon,
            zoom: self.zoom,
            map_data,
            way_styles: self.way_styles.clone(),
            area_styles: self.area_styles.clone(),
        })
    }

    // Function to load a new tile and update cache
    fn load_new_tile(&mut self, tile_x: i64, tile_y: i64) -> Result<reader::MapReadResult, String> {
        println!(
            "Loading new tile: x={}, y={}, zoom={}",
            tile_x, tile_y, self.zoom
        );
        let tile = Tile::new(tile_x, tile_y, self.zoom, TILE_SIZE as i32);

        match self.map_file.read_map_data(&tile) {
            Ok(data) => {
                // We need to convert the return type to the expected type
                // Create a new MapReadResult with the data from the original
                let map_data = reader::MapReadResult {
                    poi_way_bundles: data.poi_way_bundles.clone(),
                    is_water: data.is_water,
                };

                // Update cache with the same converted data
                self.tile_cache = Some(TileCache {
                    tile_x,
                    tile_y,
                    zoom: self.zoom,
                    data: reader::MapReadResult {
                        poi_way_bundles: data.poi_way_bundles,
                        is_water: data.is_water,
                    },
                });

                Ok(map_data)
            }
            Err(e) => Err(format!("Error reading map data: {}", e)),
        }
    }

    fn render(&mut self) -> Result<(), String> {
        // Split the rendering process into two separate steps to avoid borrow conflicts
        let state = self.prepare_render_state()?;
        self.render_map_data(state)
    }

    fn handle_input(&mut self) {
        // Pan with arrow keys - variable speed based on zoom level
        let pan_factor = 0.005 * (1.0 / (1 << (self.zoom - 10) as i32) as f64).max(0.001);

        if self.window.is_key_down(Key::Left) {
            self.center_lon -= pan_factor;
        }
        if self.window.is_key_down(Key::Right) {
            self.center_lon += pan_factor;
        }
        if self.window.is_key_down(Key::Up) {
            self.center_lat += pan_factor;
        }
        if self.window.is_key_down(Key::Down) {
            self.center_lat -= pan_factor;
        }

        // Zoom with plus and minus keys
        if self
            .window
            .is_key_pressed(Key::Equal, minifb::KeyRepeat::No)
        {
            if self.zoom < 18 {
                self.zoom += 1;
                println!("Zooming in to level {}", self.zoom);
            }
        }
        if self
            .window
            .is_key_pressed(Key::Minus, minifb::KeyRepeat::No)
        {
            if self.zoom > 1 {
                self.zoom -= 1;
                println!("Zooming out to level {}", self.zoom);
            }
        }
    }
    // Change the fill_polygon method to take a buffer directly instead of using self

    fn fill_polygon(
        points: &[(i32, i32)],
        color: u32,
        buffer: &mut [u32],
        width: usize,
        height: usize,
    ) {
        if points.len() < 3 {
            return; // Need at least 3 points for a polygon
        }

        // Find the bounding box of the polygon
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        for &(_, y) in points {
            min_y = min(min_y, y);
            max_y = max(max_y, y);
        }

        // Clip to screen bounds
        min_y = max(0, min_y);
        max_y = min(height as i32 - 1, max_y);

        // For each scanline
        for y in min_y..=max_y {
            let mut nodes = Vec::new();

            // Find intersections with polygon edges
            for i in 0..points.len() {
                let j = (i + 1) % points.len();
                let (x1, y1) = points[i];
                let (x2, y2) = points[j];

                // Check if the edge crosses this scanline
                if (y1 <= y && y2 > y) || (y2 <= y && y1 > y) {
                    // Calculate x-coordinate of intersection
                    let x = x1 + ((y - y1) as f64 * (x2 - x1) as f64 / (y2 - y1) as f64) as i32;
                    nodes.push(x);
                }
            }

            // Sort intersections
            nodes.sort();

            // Fill pixel pairs
            for i in (0..nodes.len()).step_by(2) {
                if i + 1 < nodes.len() {
                    let start_x = max(0, nodes[i]);
                    let end_x = min(width as i32 - 1, nodes[i + 1]);

                    for x in start_x..=end_x {
                        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                            buffer[(y as usize) * width + (x as usize)] = color;
                        }
                    }
                }
            }
        }
    }

    fn darken_color(color: u32, factor: f64) -> u32 {
        let r = ((color >> 16) & 0xFF) as f64 * factor;
        let g = ((color >> 8) & 0xFF) as f64 * factor;
        let b = (color & 0xFF) as f64 * factor;

        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }

    // Update the render_map_data function to use polygon filling
    fn render_map_data(&mut self, state: RenderState) -> Result<(), String> {
        let start_time = Instant::now();

        // Unpack the render state
        let RenderState {
            width,
            height,
            center_lat,
            center_lon,
            zoom,
            map_data,
            way_styles,
            area_styles,
        } = state;

        // Calculate screen center point
        let center_x = width as i32 / 2;
        let center_y = height as i32 / 2;

        // Calculate pixels per degree at current zoom level
        let pixels_per_degree_lon = 256.0 * (1 << zoom) as f64 / 360.0;
        let pixels_per_degree_lat = 256.0 * (1 << zoom) as f64 / 180.0;

        // Function to convert lat/lon to screen coordinates
        let to_screen = |lat: f64, lon: f64| -> (i32, i32) {
            let dx = (lon - center_lon) * pixels_per_degree_lon;
            let dy = (center_lat - lat) * pixels_per_degree_lat;
            (center_x + dx as i32, center_y + dy as i32)
        };

        // Function to set a pixel if it's within bounds
        let set_pixel = |x: i32, y: i32, color: u32, buffer: &mut [u32], width: usize| {
            if x >= 0 && x < width as i32 && y >= 0 && y < buffer.len() as i32 / width as i32 {
                buffer[(y as usize) * width + (x as usize)] = color;
            }
        };

        // Function to draw a thick line
        let draw_thick_line = |x0: i32,
                               y0: i32,
                               x1: i32,
                               y1: i32,
                               color: u32,
                               width: u8,
                               buffer: &mut [u32],
                               buffer_width: usize| {
            // Draw a basic line using Bresenham's algorithm
            let dx = (x1 - x0).abs();
            let dy = -(y1 - y0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;

            let mut x = x0;
            let mut y = y0;

            // For thickness, draw pixels in a square pattern around each point
            let thickness = width as i32 / 2;

            loop {
                // Draw a square around the current point for thickness
                for dy in -thickness..=thickness {
                    for dx in -thickness..=thickness {
                        set_pixel(x + dx, y + dy, color, buffer, buffer_width);
                    }
                }

                if x == x1 && y == y1 {
                    break;
                }

                let e2 = 2 * err;
                if e2 >= dy {
                    if x == x1 {
                        break;
                    }
                    err += dy;
                    x += sx;
                }
                if e2 <= dx {
                    if y == y1 {
                        break;
                    }
                    err += dx;
                    y += sy;
                }
            }
        };

        // Clear the buffer (light gray background)
        for pixel in self.buffer.iter_mut() {
            *pixel = 0x00F0F0F0;
        }

        let mut has_natural_features = false;
        let mut has_hiking_trails = false;
        let mut has_water_features = false;
        let mut has_any_areas = false;
        let mut is_hiking_path = false;

        // First pass: Render all areas
        for bundle in &map_data.poi_way_bundles {
            for way in &bundle.ways {
                // Check if this is an area way
                let mut is_area = false;
                let mut area_color = 0x00C8C8C8; // Default gray

                // Check tags to determine if it's an area and what color to use
                for tag in &way.tags {
                    // Debug logging for features
                    if tag.key == "natural" || tag.key == "landuse" {
                        has_natural_features = true;
                        println!("Found natural feature: {}={}", tag.key, tag.value);
                    }
                    if tag.key == "waterway" {
                        has_water_features = true;
                        println!("Found water feature: {}={}", tag.key, tag.value);
                    }
                    if tag.key == "area" && tag.value == "yes" {
                        has_any_areas = true;
                        println!("Found area feature");
                        is_area = true;
                    }

                    // Check standard area tags
                    let tag_key = format!("{}={}", tag.key, tag.value);
                    if let Some(&color) = area_styles.get(&tag_key) {
                        is_area = true;
                        area_color = color;
                    }

                    // Some special cases for area detection
                    if (tag.key == "natural" && (tag.value == "sea" || tag.value == "water"))
                        || (tag.key == "landuse"
                            && (tag.value == "forest"
                                || tag.value == "industrial"
                                || tag.value == "quarry"))
                    {
                        is_area = true;
                        let tag_key = format!("{}={}", tag.key, tag.value);
                        if let Some(&color) = area_styles.get(&tag_key) {
                            area_color = color;
                        }
                    }
                }

                // If it's an area, fill it
                if is_area {
                    for segment in &way.way_nodes {
                        if segment.len() < 3 {
                            continue; // Need at least 3 points for a polygon
                        }

                        // Convert lat/lon to screen coordinates
                        let mut polygon_points = Vec::with_capacity(segment.len());
                        for point in segment {
                            polygon_points.push(to_screen(point.latitude, point.longitude));
                        }

                        // Fill the polygon
                        Self::fill_polygon(
                            &polygon_points,
                            area_color,
                            &mut self.buffer,
                            width,
                            height,
                        );

                        // Draw the outline
                        for i in 0..segment.len() {
                            let j = (i + 1) % segment.len();
                            let (x0, y0) = to_screen(segment[i].latitude, segment[i].longitude);
                            let (x1, y1) = to_screen(segment[j].latitude, segment[j].longitude);

                            // Draw a slightly darker outline
                            let outline_color = Self::darken_color(area_color, 0.8);
                            draw_thick_line(
                                x0,
                                y0,
                                x1,
                                y1,
                                outline_color,
                                1,
                                &mut self.buffer,
                                width,
                            );
                        }
                    }
                }
            }
        }
        // After the area rendering code, add this to render ways
        for bundle in &map_data.poi_way_bundles {
            for way in &bundle.ways {
                // Skip if already drawn as area
                let mut is_area = false;
                for tag in &way.tags {
                    let tag_key = format!("{}={}", tag.key, tag.value);
                    if area_styles.contains_key(&tag_key)
                        || (tag.key == "area" && tag.value == "yes")
                    {
                        is_area = true;
                        break;
                    }
                }

                if is_area {
                    continue;
                }

                // Determine style based on tags
                let mut color = 0x00808080; // Default gray
                let mut line_width = 1; // Default width
                let mut is_hiking_path = false;

                for tag in &way.tags {
                    let tag_key = format!("{}={}", tag.key, tag.value);

                    // Check for hiking paths
                    if tag.key == "highway"
                        && (tag.value == "path" || tag.value == "footway" || tag.value == "track")
                    {
                        is_hiking_path = true;
                        color = 0x00AA4400; // Brown
                        line_width = if tag.value == "track" { 2 } else { 1 };
                    }

                    // Check for waterways
                    if tag.key == "waterway" && (tag.value == "river" || tag.value == "stream") {
                        color = 0x0033AAFF; // Blue
                        line_width = if tag.value == "river" { 3 } else { 2 };
                    }

                    // Get standard way style
                    if let Some(&(way_color, way_width)) = way_styles.get(&tag_key) {
                        color = way_color;
                        line_width = way_width;
                    }
                }

                // Draw the way
                for segment in &way.way_nodes {
                    if segment.len() < 2 {
                        continue;
                    }

                    // Draw each segment
                    for i in 0..segment.len() - 1 {
                        let (x0, y0) = to_screen(segment[i].latitude, segment[i].longitude);
                        let (x1, y1) = to_screen(segment[i + 1].latitude, segment[i + 1].longitude);

                        // For hiking paths, use dashed pattern
                        if is_hiking_path {
                            // Draw dashed line code here
                        } else {
                            // Regular line for other ways
                            draw_thick_line(
                                x0,
                                y0,
                                x1,
                                y1,
                                color,
                                line_width,
                                &mut self.buffer,
                                width,
                            );
                        }
                    }
                }
            }
        }

        for bundle in &map_data.poi_way_bundles {
            for poi in &bundle.pois {
                let (x, y) = to_screen(poi.position.latitude, poi.position.longitude);
                let mut poi_color = 0x00FF0000; // Default red
                let mut poi_radius = 3; // Default radius
                let mut poi_name = String::new();

                // Determine POI style based on tags
                for tag in &poi.tags {
                    if tag.key == "name" {
                        poi_name = tag.value.clone();
                    }

                    // Set color based on POI type
                    match tag.key.as_str() {
                        "amenity" => {
                            match tag.value.as_str() {
                                "restaurant" | "cafe" | "fast_food" => poi_color = 0x00FF8000, // Orange
                                "bank" | "atm" => poi_color = 0x0000AAFF, // Blue
                                "hospital" | "pharmacy" | "doctors" => poi_color = 0x00FF0000, // Red
                                "school" | "university" | "library" => poi_color = 0x00AA00FF, // Purple
                                _ => poi_color = 0x00FF6060, // Light red
                            }
                        }
                        "natural" => {
                            match tag.value.as_str() {
                                "peak" => {
                                    poi_color = 0x00663300; // Brown for mountain peaks
                                    poi_radius = 4; // Make peaks more visible
                                    println!("Found mountain peak: {}", poi_name);
                                }
                                "spring" | "water_source" => {
                                    poi_color = 0x0000AAFF; // Blue for water sources
                                    poi_radius = 3;
                                }
                                _ => {}
                            }
                        }
                        "shop" => poi_color = 0x0000CC00, // Green
                        "tourism" => {
                            match tag.value.as_str() {
                                "viewpoint" => {
                                    poi_color = 0x00FF3300; // Red for viewpoints
                                    poi_radius = 4;
                                }
                                "camp_site" | "campsite" => {
                                    poi_color = 0x0066AA00; // Green for campsites
                                    poi_radius = 4;
                                }
                                _ => poi_color = 0x00FF00FF, // Magenta for other tourism
                            }
                        }
                        "amenity" => {
                            match tag.value.as_str() {
                                "shelter" => {
                                    poi_color = 0x00AA6600; // Dark orange for shelters
                                    poi_radius = 4;
                                }
                                _ => {}
                            }
                        }
                        "historic" => {
                            match tag.value.as_str() {
                                "memorial" | "monument" => {
                                    poi_color = 0x00AA00AA; // Purple for memorials
                                    poi_radius = 4;
                                }
                                _ => {}
                            }
                        }
                        "emergency" => {
                            match tag.value.as_str() {
                                "phone" => {
                                    poi_color = 0x00FF00FF; // Magenta for emergency phones
                                    poi_radius = 3;
                                }
                                _ => {}
                            }
                        }
                        "leisure" => {
                            match tag.value.as_str() {
                                "park" => {
                                    poi_color = 0x0000AA00; // Dark green for parks
                                    poi_radius = 4;
                                }
                                _ => {}
                            }
                        }
                        "craft" => {
                            match tag.value.as_str() {
                                "brewery" | "distillery" => {
                                    poi_color = 0x00FFAA00; // Yellow for breweries
                                    poi_radius = 4;
                                }
                                _ => {}
                            }
                        }
                        "office" => {
                            match tag.value.as_str() {
                                "government" => {
                                    poi_color = 0x00FF00FF; // Magenta for government offices
                                    poi_radius = 4;
                                }
                                _ => {}
                            }
                        }
                        "power" => {
                            match tag.value.as_str() {
                                "station" => {
                                    poi_color = 0x00FF00FF; // Magenta for power stations
                                    poi_radius = 4;
                                }
                                _ => {}
                            }
                        }
                        "public_transport" => {
                            match tag.value.as_str() {
                                "station" => {
                                    poi_color = 0x0000FFFF; // Cyan for public transport stations
                                    poi_radius = 4;
                                }
                                _ => {}
                            }
                        }

                        "railway" | "highway" if tag.value == "bus_station" => {
                            poi_color = 0x0000FFFF
                        } // Cyan
                        _ => {}
                    }
                }

                // Draw a filled circle with border for each POI
                for dy in -poi_radius..=poi_radius {
                    for dx in -poi_radius..=poi_radius {
                        let distance_squared = dx * dx + dy * dy;
                        if distance_squared <= poi_radius * poi_radius {
                            // Fill
                            set_pixel(x + dx, y + dy, poi_color, &mut self.buffer, width);
                        } else if distance_squared <= (poi_radius + 1) * (poi_radius + 1) {
                            // Border (slightly larger)
                            set_pixel(x + dx, y + dy, 0x00000000, &mut self.buffer, width);
                        }
                    }
                }
            }
        }

        // Calculate and display performance metrics
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);

        if elapsed.as_millis() > 1000 {
            let fps = self.frame_count as f64 / elapsed.as_secs_f64();
            println!("FPS: {:.1}", fps);
            self.last_frame_time = now;
            self.frame_count = 0;
        }

        // Display render time for this frame
        let frame_time = start_time.elapsed();
        if frame_time.as_millis() > 100 {
            println!("Frame render time: {:?}", frame_time);
        }

        // Update the window with our buffer
        self.window
            .update_with_buffer(&self.buffer, width, height)
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

fn darken_color(color: u32, factor: f64) -> u32 {
    let r = ((color >> 16) & 0xFF) as f64 * factor;
    let g = ((color >> 8) & 0xFF) as f64 * factor;
    let b = (color & 0xFF) as f64 * factor;

    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}
fn main() -> Result<(), String> {
    let map_path = Path::new("/Users/chetan/Developer/hardware/gps/reader/north-eastern-zone.map");
    // You can also load the path from args:
    // let args: Vec<String> = std::env::args().collect();
    // let map_path = if args.len() > 1 { Path::new(&args[1]) } else { Path::new("path/to/default.map") };

    let mut renderer = MapRenderer::new(map_path)?;

    // Main rendering loop
    while renderer.window.is_open() && !renderer.window.is_key_down(Key::Escape) {
        // Handle input
        renderer.handle_input();

        // Render frame
        if let Err(e) = renderer.render() {
            println!("Rendering error: {}", e);
            break;
        }
    }

    Ok(())
}
