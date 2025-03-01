use std::env;

use map_rs::MapFile;

fn main() {
    let args: Vec<String> = env::args().collect();

    let map_file =
        match MapFile::new("/Users/chetan/Developer/hardware/gps/parser/north-eastern-zone.map") {
            Ok(file) => file,
            Err(e) => {
                println!("Error opening map file: {}", e);
                return;
            }
        };

    match map_file.get_map_file_info() {
        Some(info) => {
            println!("Map File Info:");
            println!("  Version: {}", info.file_version);
            println!("  Bounds: {:?}", info.bounding_box);
            println!("  File size: {}", info.file_size);
            println!("  Map date: {}", info.map_date);
            println!("  Number of sub-files: {}", info.number_of_sub_files);
            println!("  Projection: {}", info.projection_name);
            println!("  Tile size: {}", info.tile_pixel_size);

            // Optional fields
            if let Some(pos) = &info.start_position {
                println!("  Start position: {:?}", pos);
            }
            if let Some(zoom) = info.start_zoom_level {
                println!("  Start zoom: {}", zoom);
            }
            if let Some(lang) = &info.languages_preference {
                println!("  Languages: {}", lang);
            }
            if let Some(comment) = &info.comment {
                println!("  Comment: {}", comment);
            }
            if let Some(created_by) = &info.created_by {
                println!("  Created by: {}", created_by);
            }

            println!("  Debug file: {}", info.debug_file);
            println!("  POI tags: {}", info.poi_tags.len());
            println!("  Way tags: {}", info.way_tags.len());
        }
        None => println!("No map file info available"),
    }
}
