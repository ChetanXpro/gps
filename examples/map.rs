use reader::{LatLong, MapFile, MercatorProjection, Tile};
use std::{env, time::Instant};

fn main() {
    let file_path = "/Users/chetan/Developer/hardware/gps/reader/northern-zone.map";
    println!("Opening map file: {}", file_path);
    let start = Instant::now();

    let mut map_file = match MapFile::new(file_path) {
        Ok(file) => {
            println!("✅ Map file opened successfully in {:?}", start.elapsed());
            file
        }
        Err(e) => {
            println!("❌ Error opening map file: {}", e);
            return;
        }
    };

    // Print detailed header info
    if let Some(info) = map_file.get_map_file_info() {
        println!("\n📋 MAP FILE METADATA:");
        println!("---------------------");
        println!("🌍 Bounds: {:?}", info.bounding_box);
        println!(
            "🔍 Zoom levels: {} to {}",
            info.zoom_level_min, info.zoom_level_max
        );
        println!("🗣️ Available languages: {:?}", map_file.get_map_languages());
        println!("📅 Map date: {}", info.map_date);
        println!("🏷️ Number of POI tags: {}", info.poi_tags.len());
        println!("🏷️ Number of way tags: {}", info.way_tags.len());
        println!("🧩 Tile pixel size: {}", info.tile_pixel_size);
        println!("📍 Start position: {:?}", info.start_position);
        println!("🔢 Number of sub-files: {}", info.number_of_sub_files);
        println!("🔎 Debug file: {}", info.debug_file);

        println!("\n📊 POI Tags:");
        for (i, tag) in info.poi_tags.iter().take(20).enumerate() {
            println!("  {}: {} = {}", i, tag.key, tag.value);
        }
        if info.poi_tags.len() > 20 {
            println!("  ... and {} more", info.poi_tags.len() - 20);
        }

        println!("\n📊 Way Tags:");
        for (i, tag) in info.way_tags.iter().take(20).enumerate() {
            println!("  {}: {} = {}", i, tag.key, tag.value);
        }
        if info.way_tags.len() > 20 {
            println!("  ... and {} more", info.way_tags.len() - 20);
        }
    }

    // Print sub-file parameters
    println!("\n📂 SUB-FILE PARAMETERS:");
    println!("----------------------");
    if let Some(info) = map_file.get_map_file_info() {
        for zoom in info.zoom_level_min..=info.zoom_level_max {
            match map_file.header.get_sub_file_parameter(zoom as usize) {
                Some(param) => {
                    println!("\n🔎 Zoom level {}:", zoom);
                    println!("  Base zoom level: {}", param.base_zoom_level);
                    println!(
                        "  Min/Max zoom: {} to {}",
                        param.zoom_level_min, param.zoom_level_max
                    );
                    println!("  Start address: {}", param.start_address);
                    println!("  Sub-file size: {}", param.sub_file_size);
                    println!("  Number of blocks: {}", param.number_of_blocks);
                    println!(
                        "  Block dimensions: {}x{}",
                        param.blocks_width, param.blocks_height
                    );
                    println!(
                        "  Boundary tiles: Left={}, Top={}, Right={}, Bottom={}",
                        param.boundary_tile_left,
                        param.boundary_tile_top,
                        param.boundary_tile_right,
                        param.boundary_tile_bottom
                    );
                }
                None => println!("❌ Zoom level {}: Not available", zoom),
            }
        }
    }

    // Try a few different coordinates and zoom levels
    println!("\n🌍 TESTING DIFFERENT COORDINATES AND ZOOM LEVELS:");
    println!("----------------------------------------------");

    // Define test cases
    let test_cases = [
        // Original file bounds
        // (0.04, 0.04, 10), // Center of the with_data/output.map
        // (0.0, 0.0, 8),    // Corner of map at default test zoom
        // Your provided coordinates
        (26.7428831, 93.9074701, 12), // Guwahati
                                      // Different zoom levels
                                      // (0.04, 0.04, 8),  // Same location, different zoom
                                      // (0.04, 0.04, 14), // Same location, even higher zoom
    ];

    for (i, (lat, lon, zoom)) in test_cases.iter().enumerate() {
        println!(
            "\n🧪 Test case {}: ({}, {}) at zoom {}",
            i + 1,
            lat,
            lon,
            zoom
        );

        // Convert coordinates to tile
        let tile_x = MercatorProjection::longitude_to_tile_x(*lon, *zoom);
        let tile_y = MercatorProjection::latitude_to_tile_y(*lat, *zoom);
        let tile = Tile::new(tile_x, tile_y, *zoom, 256);

        println!("  🧩 Tile: x={}, y={}, zoom={}", tile_x, tile_y, zoom);

        // Read map data with timing
        let start = Instant::now();
        match map_file.read_map_data(&tile) {
            Ok(result) => {
                println!("  ✅ Read map data in {:?}", start.elapsed());
                println!("  📦 Number of bundles: {}", result.poi_way_bundles.len());

                let mut total_pois = 0;
                let mut total_ways = 0;

                // Print detailed information about each bundle
                for (b_idx, bundle) in result.poi_way_bundles.iter().enumerate() {
                    println!("  📦 Bundle {}:", b_idx);

                    // POIs
                    println!("    🔍 POIs: {}", bundle.pois.len());
                    for (p_idx, poi) in bundle.pois.iter().enumerate().take(5) {
                        println!(
                            "      📍 POI {}: layer={}, position=({}, {})",
                            p_idx, poi.layer, poi.position.latitude, poi.position.longitude
                        );
                        println!("        🏷️ Tags: {}", poi.tags.len());
                        for (t_idx, tag) in poi.tags.iter().enumerate().take(3) {
                            println!("          📝 {}: {} = {}", t_idx, tag.key, tag.value);
                        }
                        if poi.tags.len() > 3 {
                            println!("          ... and {} more tags", poi.tags.len() - 3);
                        }
                    }
                    if bundle.pois.len() > 5 {
                        println!("      ... and {} more POIs", bundle.pois.len() - 5);
                    }

                    // Ways
                    println!("    🛣️ Ways: {}", bundle.ways.len());
                    for (w_idx, way) in bundle.ways.iter().enumerate().take(5) {
                        println!(
                            "      🛣️ Way {}: layer={}, label_position={:?}",
                            w_idx, way.layer, way.label_position
                        );
                        println!("        🏷️ Tags: {}", way.tags.len());
                        for (t_idx, tag) in way.tags.iter().enumerate().take(3) {
                            println!("          📝 {}: {} = {}", t_idx, tag.key, tag.value);
                        }
                        if way.tags.len() > 3 {
                            println!("          ... and {} more tags", way.tags.len() - 3);
                        }

                        println!("        🧭 Segments: {}", way.way_nodes.len());
                        for (s_idx, segment) in way.way_nodes.iter().enumerate() {
                            println!("          📍 Segment {}: {} points", s_idx, segment.len());
                            for (n_idx, node) in segment.iter().enumerate().take(5) {
                                println!(
                                    "            📌 Node {}: ({}, {})",
                                    n_idx, node.latitude, node.longitude
                                );
                            }
                            if segment.len() > 5 {
                                println!("            ... and {} more nodes", segment.len() - 5);
                            }
                        }
                    }
                    if bundle.ways.len() > 5 {
                        println!("      ... and {} more ways", bundle.ways.len() - 5);
                    }

                    total_pois += bundle.pois.len();
                    total_ways += bundle.ways.len();
                }

                println!("  📊 Summary for this tile:");
                println!("    Total POIs: {}", total_pois);
                println!("    Total Ways: {}", total_ways);
            }
            Err(e) => println!("  ❌ Error reading map data: {}", e),
        }
    }

    // Final performance test with timing
    println!("\n⏱️ PERFORMANCE TEST:");
    println!("-----------------");
    let start = Instant::now();
    let zoom = 14;
    let tile_x = MercatorProjection::longitude_to_tile_x(0.04, zoom);
    let tile_y = MercatorProjection::latitude_to_tile_y(0.04, zoom);
    let tile = Tile::new(tile_x, tile_y, zoom, 256);

    match map_file.read_map_data(&tile) {
        Ok(result) => {
            let elapsed = start.elapsed();
            println!("✅ Read tile at zoom {} in {:?}", zoom, elapsed);
            println!(
                "📊 Found {} bundles, with a total of {} POIs and {} ways",
                result.poi_way_bundles.len(),
                result
                    .poi_way_bundles
                    .iter()
                    .map(|b| b.pois.len())
                    .sum::<usize>(),
                result
                    .poi_way_bundles
                    .iter()
                    .map(|b| b.ways.len())
                    .sum::<usize>()
            );
        }
        Err(e) => println!("❌ Error in performance test: {}", e),
    }

    println!("\n🏁 Testing completed!");
}
