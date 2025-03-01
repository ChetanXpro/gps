#[cfg(test)]
mod tests {
    use reader::{BoundingBox, LatLong, MapFile};

    use super::*;

    use std::path::PathBuf;

    const BOUNDING_BOX: BoundingBox = BoundingBox {
        min_latitude: 0.1,
        min_longitude: 0.2,
        max_latitude: 0.3,
        max_longitude: 0.4,
    };
    const COMMENT: &str = "testcomment";
    const CREATED_BY: &str = "mapsforge-map-writer-0.3.1-SNAPSHOT";
    const FILE_SIZE: i64 = 709;
    const FILE_VERSION: i32 = 3;
    const LANGUAGES_PREFERENCE: &str = "en";
    const MAP_DATE: i64 = 1335871456973;
    const NUMBER_OF_SUBFILES: u8 = 3;
    const PROJECTION_NAME: &str = "Mercator";
    const START_POSITION: LatLong = LatLong {
        latitude: 0.15,
        longitude: 0.25,
    };
    const START_ZOOM_LEVEL: u8 = 16;
    const TILE_PIXEL_SIZE: i32 = 256;

    #[test]
    fn test_map_file_info() {
        let test_file = PathBuf::from("/Users/chetan/Developer/hardware/gps/mapsforge/mapsforge-map-reader/src/test/resources/file_header/output.map");
        let map_file = MapFile::new(test_file).expect("Failed to open map file");

        let map_file_info = map_file
            .get_map_file_info()
            .expect("Failed to get map file info");

        // assert_eq!(map_file_info.bounding_box, BOUNDING_BOX);
        assert_eq!(map_file_info.file_size, FILE_SIZE);
        assert_eq!(map_file_info.file_version, FILE_VERSION);
        assert_eq!(map_file_info.map_date, MAP_DATE);
        assert_eq!(map_file_info.number_of_sub_files, NUMBER_OF_SUBFILES);
        assert_eq!(map_file_info.projection_name, PROJECTION_NAME);
        assert_eq!(map_file_info.tile_pixel_size, TILE_PIXEL_SIZE);

        assert_eq!(map_file_info.poi_tags.len(), 0);
        assert_eq!(map_file_info.way_tags.len(), 0);

        assert!(!map_file_info.debug_file);
        // assert_eq!(map_file_info.start_position, Some(START_POSITION));
        assert_eq!(map_file_info.start_zoom_level, Some(START_ZOOM_LEVEL));
        assert_eq!(
            map_file_info.languages_preference,
            Some(LANGUAGES_PREFERENCE.to_string())
        );
        assert_eq!(map_file_info.comment, Some(COMMENT.to_string()));
        assert_eq!(map_file_info.created_by, Some(CREATED_BY.to_string()));
    }
}
