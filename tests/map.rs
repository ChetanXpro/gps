#[cfg(test)]
mod tests {
    use env_logger;
    use reader::{Deserializer, LatLong, MapFile, MercatorProjection, QueryParameters, Tile};
    use tracing::{error, info};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    use super::*;

    use std::path::PathBuf;

    #[test]
    fn test_deserializer() {
        // Test getInt
        let buffer = vec![0, 0, 0, 0];
        assert_eq!(Deserializer::get_int(&buffer, 0), 0);

        let buffer = vec![0, 0, 0, 1];
        assert_eq!(Deserializer::get_int(&buffer, 0), 1);

        let buffer = vec![0, 0, 0, 127];
        assert_eq!(Deserializer::get_int(&buffer, 0), 127);

        let buffer = vec![0, 0, 0, 128];
        assert_eq!(Deserializer::get_int(&buffer, 0), 128);

        // Test getShort
        let buffer = vec![0, 0];
        assert_eq!(Deserializer::get_short(&buffer, 0), 0);

        let buffer = vec![0, 1];
        assert_eq!(Deserializer::get_short(&buffer, 0), 1);

        let buffer = vec![0, 127];
        assert_eq!(Deserializer::get_short(&buffer, 0), 127);
    }
    fn run_encoding_test(map_file: &mut MapFile) {
        init();
        const ZOOM_LEVEL: u8 = 8;

        let tile_x = MercatorProjection::longitude_to_tile_x(0.0, ZOOM_LEVEL);
        let tile_y = MercatorProjection::latitude_to_tile_y(0.0, ZOOM_LEVEL);

        info!("Test coordinates: lon=0.0, lat=0.0");
        info!("Calculated tile coordinates: x={}, y={}", tile_x, tile_y);

        let tile = Tile::new(tile_x, tile_y, ZOOM_LEVEL, 256);

        // Log SubFileParameter details
        if let Some(info) = map_file.get_map_file_info() {
            info!("Map file info: {:?}", info);
        }

        // Test named items
        info!("Reading named items...");
        let map_read_result = map_file.read_named_items(&tile).unwrap();
        info!(
            "Named items result: {} bundles",
            map_read_result.poi_way_bundles.len()
        );

        // Test POI data
        info!("Reading POI data...");
        let map_read_result = map_file.read_poi_data(&tile).unwrap();
        info!(
            "POI data result: {} bundles",
            map_read_result.poi_way_bundles.len()
        );

        // Test map data
        info!("Reading map data...");
        let map_read_result = map_file.read_map_data(&tile).unwrap();
        info!(
            "Map data result: {} bundles",
            map_read_result.poi_way_bundles.len()
        );

        assert_eq!(map_read_result.poi_way_bundles.len(), 1);

        let way = &map_read_result.poi_way_bundles[0].ways[0];
        let expected_coords = vec![vec![
            LatLong::new(0.0, 0.0),
            LatLong::new(0.0, 0.1),
            LatLong::new(-0.1, 0.1),
            LatLong::new(-0.1, 0.0),
            LatLong::new(0.0, 0.0),
        ]];
        info!("Comparing coordinates:");
        info!("Expected: {:?}", expected_coords);
        info!("Actual: {:?}", way.way_nodes);
        assert_eq!(way.way_nodes, expected_coords);
    }
    #[test]
    fn test_double_delta_encoding() {
        let mut map_file =
            MapFile::new("/Users/chetan/Developer/hardware/gps/mapsforge/mapsforge-map-reader/src/test/resources/double_delta_encoding/output.map").unwrap();
        run_encoding_test(&mut map_file);
    }

    #[test]
    fn test_single_delta_encoding() {
        init();
        info!("Starting single delta encoding test");
        let mut map_file = MapFile::new(
            "/Users/chetan/Developer/hardware/gps/mapsforge/mapsforge-map-reader/src/test/resources/single_delta_encoding/output.map"
        ).unwrap_or_else(|e| {
            error!("Failed to open map file: {}", e);
            panic!("Failed to open map file: {}", e);
        });
        run_encoding_test(&mut map_file);
    }

    #[test]
    fn test_empty_map() {
        init();
        info!("Starting empty map test");
        let mut map_file = MapFile::new(
            "/Users/chetan/Developer/hardware/gps/mapsforge/mapsforge-map-reader/src/test/resources/empty/output.map"
        ).unwrap_or_else(|e| {
            error!("Failed to open map file: {}", e);
            panic!("Failed to open map file: {}", e);
        });

        for zoom_level in 0..=25 {
            info!("Testing zoom level {}", zoom_level);
            let tile_x = MercatorProjection::longitude_to_tile_x(1.0, zoom_level);
            let tile_y = MercatorProjection::latitude_to_tile_y(1.0, zoom_level);
            info!("Tile coordinates: x={}, y={}", tile_x, tile_y);

            let tile = Tile::new(tile_x, tile_y, zoom_level, 256);
            let map_read_result = map_file.read_map_data(&tile).unwrap_or_else(|e| {
                error!("Failed to read map data: {}", e);
                panic!("Failed to read map data: {}", e);
            });
            assert!(map_read_result.poi_way_bundles.is_empty());
        }
    }
    #[test]
    fn test_query_calculations() {
        init();
        let mut map_file =
            MapFile::new("/Users/chetan/Developer/hardware/gps/mapsforge/mapsforge-map-reader/src/test/resources/single_delta_encoding/output.map").unwrap();

        for zoom_level in 0..=25 {
            let mut single = QueryParameters::new();
            let mut multi = QueryParameters::new();

            let sub_file_parameter = map_file
                .header
                .get_sub_file_parameter(single.query_zoom_level as usize)
                .unwrap();
            let tile = Tile::new(zoom_level as i64, zoom_level as i64, zoom_level, 256);

            single.calculate_base_tiles(&tile, &tile, sub_file_parameter);
            multi.calculate_base_tiles(&tile, &tile, sub_file_parameter);

            assert_eq!(single, multi);
        }
    }

    #[test]
    fn test_map_file_with_data() {
        init();

        info!("Starting map file with data tes==================================================t");
        let mut map_file = MapFile::new("/Users/chetan/Developer/hardware/gps/mapsforge/mapsforge-map-reader/src/test/resources/with_data/output.map").unwrap();

        let map_file_info = map_file.get_map_file_info().unwrap();
        assert!(map_file_info.debug_file);

        let tile_x = MercatorProjection::longitude_to_tile_x(0.04, 10);
        let tile_y = MercatorProjection::latitude_to_tile_y(0.04, 10);
        let tile = Tile::new(tile_x, tile_y, 10, 256);

        let map_read_result = map_file.read_map_data(&tile).unwrap();
        assert_eq!(map_read_result.poi_way_bundles.len(), 1);

        let poi = &map_read_result.poi_way_bundles[0].pois[0];
        assert_eq!(poi.layer, 7);
        assert!(approx_equal(poi.position.latitude, 0.04, 0.0001));

        assert_eq!(poi.position.longitude, 0.08);
        assert_eq!(poi.tags.len(), 4);
        // Check specific tags...

        let way = &map_read_result.poi_way_bundles[0].ways[0];
        assert_eq!(way.layer, 4);
        assert!(way.label_position.is_none());
        // Check way coordinates and tags...
    }

    fn approx_equal(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }
}
