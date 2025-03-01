use tracing::{info, warn};

use crate::errors::MapFileException;
use crate::map_data::{MapReadResult, PoiWayBundle};

use crate::header::{MapFileHeader, MapFileInfo};
use crate::index_cache::IndexCache;
use crate::map_data::{PointOfInterest, Way};
use crate::mercator::MercatorProjection;
use crate::query_parameters::QueryParameters;
use crate::reader::ReadBuffer;
use crate::tile::Tile;
use crate::types::{BoundingBox, LatLong, LatLongUtils, Tag};
use crate::SubFileParameter;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

pub const INDEX_CACHE_SIZE: usize = 64;
pub const DEFAULT_START_ZOOM_LEVEL: u8 = 12;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Selector {
    All,
    Pois,
    Named,
}

// POI constants
const POI_FEATURE_ELEVATION: u8 = 0x20;
const POI_FEATURE_HOUSE_NUMBER: u8 = 0x40;
const POI_FEATURE_NAME: u8 = 0x80;
const POI_LAYER_BITMASK: u8 = 0xf0;
const POI_LAYER_SHIFT: u8 = 4;
const POI_NUMBER_OF_TAGS_BITMASK: u8 = 0x0f;

// Signature lengths
const SIGNATURE_LENGTH_BLOCK: usize = 32;
const SIGNATURE_LENGTH_POI: usize = 32;
const SIGNATURE_LENGTH_WAY: usize = 32;

// Tag keys
const TAG_KEY_ELE: &str = "ele";
const TAG_KEY_HOUSE_NUMBER: &str = "addr:housenumber";
const TAG_KEY_NAME: &str = "name";
const TAG_KEY_REF: &str = "ref";

// Way constants
const WAY_FEATURE_DATA_BLOCKS_BYTE: u8 = 0x08;
const WAY_FEATURE_DOUBLE_DELTA_ENCODING: u8 = 0x04;
const WAY_FEATURE_HOUSE_NUMBER: u8 = 0x40;
const WAY_FEATURE_LABEL_POSITION: u8 = 0x10;
const WAY_FEATURE_NAME: u8 = 0x80;
const WAY_FEATURE_REF: u8 = 0x20;
const WAY_LAYER_BITMASK: u8 = 0xf0;
const WAY_LAYER_SHIFT: u8 = 4;
const WAY_NUMBER_OF_TAGS_BITMASK: u8 = 0x0f;

// Existing constants
const BITMASK_INDEX_OFFSET: i64 = 0x7FFFFFFFF;
const BITMASK_INDEX_WATER: i64 = 0x8000000000;

const INVALID_FIRST_WAY_OFFSET: &str = "invalid first way offset: ";

// Global settings with unsafe access
static mut WAY_FILTER_ENABLED: bool = true;
static mut WAY_FILTER_DISTANCE: i32 = 20;
pub struct MapFile {
    file: File,
    pub header: MapFileHeader,
    database_index_cache: Option<IndexCache<File>>,
    file_size: i64,
    timestamp: i64,
    zoom_level_min: u8,
    zoom_level_max: u8,
}

impl MapFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, MapFileException> {
        let file = File::open(&path)?;
        let file_size = file.metadata()?.len() as i64;
        let timestamp = std::fs::metadata(&path)?
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let mut read_buffer = ReadBuffer::new(file.try_clone()?);

        let mut header = MapFileHeader::new();
        header.read_header(&mut read_buffer, file_size)?;

        let database_index_cache = Some(IndexCache::new(file.try_clone()?, INDEX_CACHE_SIZE));

        Ok(Self {
            file,
            header,
            database_index_cache,
            file_size,
            timestamp,
            zoom_level_min: 0,
            zoom_level_max: u8::MAX,
        })
    }

    pub fn get_map_file_info(&self) -> Option<&MapFileInfo> {
        self.header.get_map_file_info()
    }

    pub fn get_data_timestamp(&self, _tile: &Tile) -> i64 {
        self.timestamp
    }

    pub fn get_map_languages(&self) -> Option<Vec<String>> {
        self.get_map_file_info().and_then(|info| {
            info.languages_preference
                .as_ref()
                .map(|langs| langs.split(',').map(|s| s.to_string()).collect())
        })
    }

    pub fn restrict_to_zoom_range(&mut self, min_zoom: u8, max_zoom: u8) {
        self.zoom_level_max = max_zoom;
        self.zoom_level_min = min_zoom;
    }

    pub fn start_position(&self) -> LatLong {
        if let Some(info) = self.get_map_file_info() {
            if let Some(pos) = &info.start_position {
                return pos.clone();
            }
            // Get center point of bounding box
            return info.bounding_box.get_center_point();
        }
        // This should never happen as MapFileInfo is required
        panic!("Missing MapFileInfo");
    }

    pub fn start_zoom_level(&self) -> u8 {
        if let Some(info) = self.get_map_file_info() {
            info.start_zoom_level.unwrap_or(DEFAULT_START_ZOOM_LEVEL)
        } else {
            DEFAULT_START_ZOOM_LEVEL
        }
    }

    fn close_file_channel(&mut self) {
        if let Some(cache) = &mut self.database_index_cache {
            cache.destroy();
        }
        // File will be closed automatically when dropped
    }

    fn decode_way_nodes_double_delta(
        &self,
        way_segment: &mut [LatLong],
        tile_latitude: f64,
        tile_longitude: f64,
        read_buffer: &mut ReadBuffer<impl Read + Seek>,
    ) -> Result<(), MapFileException> {
        // Get the first way node latitude offset (VBE-S)
        let way_node_latitude =
            tile_latitude + LatLongUtils::microdegrees_to_degrees(read_buffer.read_signed_int()?);

        // Get the first way node longitude offset (VBE-S)
        let way_node_longitude =
            tile_longitude + LatLongUtils::microdegrees_to_degrees(read_buffer.read_signed_int()?);

        // Store the first way node
        way_segment[0] = LatLong::new(way_node_latitude, way_node_longitude);

        let mut previous_single_delta_latitude = 0.0;
        let mut previous_single_delta_longitude = 0.0;
        let mut way_node_lat = way_node_latitude;
        let mut way_node_lon = way_node_longitude;

        for way_segment_pos in 1..way_segment.len() {
            // Get the way node latitude double-delta offset (VBE-S)
            let double_delta_latitude =
                LatLongUtils::microdegrees_to_degrees(read_buffer.read_signed_int()?);
            let double_delta_longitude =
                LatLongUtils::microdegrees_to_degrees(read_buffer.read_signed_int()?);

            let single_delta_latitude = double_delta_latitude + previous_single_delta_latitude;
            let single_delta_longitude = double_delta_longitude + previous_single_delta_longitude;

            way_node_lat += single_delta_latitude;
            way_node_lon += single_delta_longitude;

            // Handle international date line cases
            if way_node_lon < LatLongUtils::LONGITUDE_MIN
                && (LatLongUtils::LONGITUDE_MIN - way_node_lon).abs() < 0.001
            {
                way_node_lon = LatLongUtils::LONGITUDE_MIN;
            } else if way_node_lon > LatLongUtils::LONGITUDE_MAX
                && (way_node_lon - LatLongUtils::LONGITUDE_MAX).abs() < 0.001
            {
                way_node_lon = LatLongUtils::LONGITUDE_MAX;
            }

            way_segment[way_segment_pos] = LatLong::new(way_node_lat, way_node_lon);

            previous_single_delta_latitude = single_delta_latitude;
            previous_single_delta_longitude = single_delta_longitude;
        }

        Ok(())
    }

    fn decode_way_nodes_single_delta(
        &self,
        way_segment: &mut [LatLong],
        tile_latitude: f64,
        tile_longitude: f64,
        read_buffer: &mut ReadBuffer<impl Read + Seek>,
    ) -> Result<(), MapFileException> {
        // Get the first way node latitude offset (VBE-S)
        let mut way_node_latitude =
            tile_latitude + LatLongUtils::microdegrees_to_degrees(read_buffer.read_signed_int()?);
        let mut way_node_longitude =
            tile_longitude + LatLongUtils::microdegrees_to_degrees(read_buffer.read_signed_int()?);

        // Store the first way node
        way_segment[0] = LatLong::new(way_node_latitude, way_node_longitude);

        for way_segment_pos in 1..way_segment.len() {
            // Get the way node offsets (VBE-S)
            way_node_latitude +=
                LatLongUtils::microdegrees_to_degrees(read_buffer.read_signed_int()?);
            way_node_longitude +=
                LatLongUtils::microdegrees_to_degrees(read_buffer.read_signed_int()?);

            // Handle international date line cases
            if way_node_longitude < LatLongUtils::LONGITUDE_MIN
                && (LatLongUtils::LONGITUDE_MIN - way_node_longitude).abs() < 0.001
            {
                way_node_longitude = LatLongUtils::LONGITUDE_MIN;
            } else if way_node_longitude > LatLongUtils::LONGITUDE_MAX
                && (way_node_longitude - LatLongUtils::LONGITUDE_MAX).abs() < 0.001
            {
                way_node_longitude = LatLongUtils::LONGITUDE_MAX;
            }

            way_segment[way_segment_pos] = LatLong::new(way_node_latitude, way_node_longitude);
        }

        Ok(())
    }

    fn process_way_data_block(
        &self,
        tile_latitude: f64,
        tile_longitude: f64,
        double_delta_encoding: bool,
        read_buffer: &mut ReadBuffer<impl Read + Seek>,
    ) -> Result<Vec<Vec<LatLong>>, MapFileException> {
        // Get and check the number of way coordinate blocks (VBE-U)
        let number_of_way_coordinate_blocks = read_buffer.read_unsigned_int()? as usize;
        if number_of_way_coordinate_blocks < 1
            || number_of_way_coordinate_blocks > i16::MAX as usize
        {
            return Err(MapFileException::new(format!(
                "invalid number of way coordinate blocks: {}",
                number_of_way_coordinate_blocks
            )));
        }

        let mut way_coordinates = Vec::with_capacity(number_of_way_coordinate_blocks);

        // Read the way coordinate blocks
        for _ in 0..number_of_way_coordinate_blocks {
            let number_of_way_nodes = read_buffer.read_unsigned_int()? as usize;
            if number_of_way_nodes < 2 || number_of_way_nodes > i16::MAX as usize {
                return Err(MapFileException::new(format!(
                    "invalid number of way nodes: {}",
                    number_of_way_nodes
                )));
            }

            let mut way_segment = vec![LatLong::new(0.0, 0.0); number_of_way_nodes];

            if double_delta_encoding {
                self.decode_way_nodes_double_delta(
                    &mut way_segment,
                    tile_latitude,
                    tile_longitude,
                    read_buffer,
                )?;
            } else {
                self.decode_way_nodes_single_delta(
                    &mut way_segment,
                    tile_latitude,
                    tile_longitude,
                    read_buffer,
                )?;
            }

            way_coordinates.push(way_segment);
        }

        Ok(way_coordinates)
    }

    fn process_pois(
        &self,
        tile_latitude: f64,
        tile_longitude: f64,
        number_of_pois: usize,
        bounding_box: &BoundingBox,
        filter_required: bool,
        read_buffer: &mut ReadBuffer<impl Read + Seek>,
    ) -> Result<Vec<PointOfInterest>, MapFileException> {
        let mut pois = Vec::new();
        let poi_tags = self
            .get_map_file_info()
            .ok_or_else(|| MapFileException::new("Missing map file info"))?
            .poi_tags
            .clone();

        for _ in 0..number_of_pois {
            if self.header.get_map_file_info().unwrap().debug_file {
                // Check POI signature in debug mode
                let signature_poi =
                    read_buffer.read_utf8_encoded_string_with_length(SIGNATURE_LENGTH_POI)?;
                if !signature_poi.starts_with("***POIStart") {
                    return Err(MapFileException::new(format!(
                        "invalid POI signature: {}",
                        signature_poi
                    )));
                }
            }

            // Get POI position
            let latitude = tile_latitude
                + LatLongUtils::microdegrees_to_degrees(read_buffer.read_signed_int()?);
            let longitude = tile_longitude
                + LatLongUtils::microdegrees_to_degrees(read_buffer.read_signed_int()?);

            // Read special byte
            let special_byte = read_buffer.read_byte()?;
            let layer = ((special_byte & POI_LAYER_BITMASK) >> POI_LAYER_SHIFT) as i8;
            let number_of_tags = special_byte & POI_NUMBER_OF_TAGS_BITMASK;

            // Get tags
            let mut tags = read_buffer.read_tags(&poi_tags, number_of_tags)?;

            // Read feature byte
            let feature_byte = read_buffer.read_byte()?;
            let feature_name = (feature_byte & POI_FEATURE_NAME) != 0;
            let feature_house_number = (feature_byte & POI_FEATURE_HOUSE_NUMBER) != 0;
            let feature_elevation = (feature_byte & POI_FEATURE_ELEVATION) != 0;

            // Add optional features
            if feature_name {
                tags.push(Tag::new(
                    TAG_KEY_NAME.to_string(),
                    read_buffer.read_utf8_encoded_string()?,
                ));
            }

            if feature_house_number {
                tags.push(Tag::new(
                    TAG_KEY_HOUSE_NUMBER.to_string(),
                    read_buffer.read_utf8_encoded_string()?,
                ));
            }

            if feature_elevation {
                tags.push(Tag::new(
                    TAG_KEY_ELE.to_string(),
                    read_buffer.read_signed_int()?.to_string(),
                ));
            }

            let position = LatLong::new(latitude, longitude);
            if !filter_required || bounding_box.contains(latitude, longitude) {
                pois.push(PointOfInterest::new(layer, tags, position));
            }
        }

        Ok(pois)
    }

    fn process_block_signature(
        &self,
        read_buffer: &mut ReadBuffer<impl Read + Seek>,
    ) -> Result<bool, MapFileException> {
        if self.header.get_map_file_info().unwrap().debug_file {
            let signature_block =
                read_buffer.read_utf8_encoded_string_with_length(SIGNATURE_LENGTH_BLOCK)?;
            if !signature_block.starts_with("###TileStart") {
                return Err(MapFileException::new(format!(
                    "invalid block signature: {}",
                    signature_block
                )));
            }
        }
        Ok(true)
    }

    fn read_zoom_table(
        &self,
        sub_file_parameter: &SubFileParameter,
        read_buffer: &mut ReadBuffer<impl Read + Seek>,
    ) -> Result<Vec<[i32; 2]>, MapFileException> {
        let rows =
            (sub_file_parameter.zoom_level_max - sub_file_parameter.zoom_level_min + 1) as usize;
        let mut zoom_table = vec![[0, 0]; rows];

        let mut cumulated_number_of_pois = 0;
        let mut cumulated_number_of_ways = 0;

        for row in 0..rows {
            cumulated_number_of_pois += read_buffer.read_unsigned_int()? as i32;
            cumulated_number_of_ways += read_buffer.read_unsigned_int()? as i32;

            zoom_table[row][0] = cumulated_number_of_pois;
            zoom_table[row][1] = cumulated_number_of_ways;
        }

        Ok(zoom_table)
    }

    fn read_optional_label_position(
        &self,
        read_buffer: &mut ReadBuffer<impl Read + Seek>,
    ) -> Result<[i32; 2], MapFileException> {
        let mut label_position = [0, 0];

        // Get label position offsets (VBE-S)
        label_position[1] = read_buffer.read_signed_int()?;
        label_position[0] = read_buffer.read_signed_int()?;

        Ok(label_position)
    }

    fn read_optional_way_data_blocks_byte(
        &self,
        feature_way_data_blocks_byte: bool,
        read_buffer: &mut ReadBuffer<impl Read + Seek>,
    ) -> Result<i32, MapFileException> {
        if feature_way_data_blocks_byte {
            read_buffer.read_unsigned_int().map(|v| v as i32)
        } else {
            Ok(1) // Only one way data block exists
        }
    }

    fn process_ways(
        &self,
        query_parameters: &QueryParameters,
        number_of_ways: usize,
        bounding_box: &BoundingBox,
        filter_required: bool,
        tile_latitude: f64,
        tile_longitude: f64,
        selector: Selector,
        read_buffer: &mut ReadBuffer<impl Read + Seek>,
    ) -> Result<Vec<Way>, MapFileException> {
        let mut ways = Vec::new();
        let way_tags = self
            .get_map_file_info()
            .ok_or_else(|| MapFileException::new("Missing map file info"))?
            .way_tags
            .clone();

        // Calculate extended bounding box for way filtering
        let way_filter_bbox = if unsafe { WAY_FILTER_ENABLED } {
            bounding_box.extend_meters(unsafe { WAY_FILTER_DISTANCE })
        } else {
            bounding_box.clone()
        };

        for _ in 0..number_of_ways {
            if self.header.get_map_file_info().unwrap().debug_file {
                // Check way signature in debug mode
                let signature_way =
                    read_buffer.read_utf8_encoded_string_with_length(SIGNATURE_LENGTH_WAY)?;
                if !signature_way.starts_with("---WayStart") {
                    return Err(MapFileException::new(format!(
                        "invalid way signature: {}",
                        signature_way
                    )));
                }
            }

            // Get way data size
            let way_data_size = read_buffer.read_unsigned_int()? as i32;
            if way_data_size < 0 {
                return Err(MapFileException::new(format!(
                    "invalid way data size: {}",
                    way_data_size
                )));
            }

            if query_parameters.use_tile_bitmask {
                // Check if way is inside requested tile
                let tile_bitmask = read_buffer.read_short()? as i32;
                if (query_parameters.query_tile_bitmask & tile_bitmask) == 0 {
                    // Skip the rest of the way
                    read_buffer.skip_bytes((way_data_size - 2) as usize);
                    continue;
                }
            } else {
                // Skip tile bitmask
                read_buffer.skip_bytes(2);
            }

            // Read special byte
            let special_byte = read_buffer.read_byte()?;
            let layer = ((special_byte & WAY_LAYER_BITMASK) >> WAY_LAYER_SHIFT) as i8;
            let number_of_tags = special_byte & WAY_NUMBER_OF_TAGS_BITMASK;

            // Get tags
            let mut tags = read_buffer.read_tags(&way_tags, number_of_tags)?;

            // Read feature byte
            let feature_byte = read_buffer.read_byte()?;
            let feature_name = (feature_byte & WAY_FEATURE_NAME) != 0;
            let feature_house_number = (feature_byte & WAY_FEATURE_HOUSE_NUMBER) != 0;
            let feature_ref = (feature_byte & WAY_FEATURE_REF) != 0;
            let feature_label_position = (feature_byte & WAY_FEATURE_LABEL_POSITION) != 0;
            let feature_data_blocks_byte = (feature_byte & WAY_FEATURE_DATA_BLOCKS_BYTE) != 0;
            let feature_double_delta_encoding =
                (feature_byte & WAY_FEATURE_DOUBLE_DELTA_ENCODING) != 0;

            // Add optional features
            if feature_name {
                tags.push(Tag::new(
                    TAG_KEY_NAME.to_string(),
                    read_buffer.read_utf8_encoded_string()?,
                ));
            }

            if feature_house_number {
                tags.push(Tag::new(
                    TAG_KEY_HOUSE_NUMBER.to_string(),
                    read_buffer.read_utf8_encoded_string()?,
                ));
            }

            if feature_ref {
                tags.push(Tag::new(
                    TAG_KEY_REF.to_string(),
                    read_buffer.read_utf8_encoded_string()?,
                ));
            }

            // Read label position if present
            let label_position = if feature_label_position {
                Some(self.read_optional_label_position(read_buffer)?)
            } else {
                None
            };

            // Read number of way data blocks
            let way_data_blocks =
                self.read_optional_way_data_blocks_byte(feature_data_blocks_byte, read_buffer)?;
            if way_data_blocks < 1 {
                return Err(MapFileException::new(format!(
                    "invalid number of way data blocks: {}",
                    way_data_blocks
                )));
            }

            // Process each way data block
            for _ in 0..way_data_blocks {
                let way_nodes = self.process_way_data_block(
                    tile_latitude,
                    tile_longitude,
                    feature_double_delta_encoding,
                    read_buffer,
                )?;

                // Skip if way is outside filter area
                if filter_required
                    && unsafe { WAY_FILTER_ENABLED }
                    && !Self::way_intersects_bbox(&way_nodes, &way_filter_bbox)
                {
                    continue;
                }

                // Add way if it meets selector criteria
                if matches!(selector, Selector::All)
                    || feature_name
                    || feature_house_number
                    || feature_ref
                    || Self::has_label_tag(&tags)
                {
                    let label_pos = label_position.map(|pos| {
                        LatLong::new(
                            way_nodes[0][0].latitude
                                + LatLongUtils::microdegrees_to_degrees(pos[1]),
                            way_nodes[0][0].longitude
                                + LatLongUtils::microdegrees_to_degrees(pos[0]),
                        )
                    });

                    ways.push(Way::new(layer, tags.clone(), way_nodes, label_pos));
                }
            }
        }

        Ok(ways)
    }

    fn has_label_tag(tags: &[Tag]) -> bool {
        // Implementation depends on your tag filtering logic
        // For now, return true if any tag might need a label
        tags.iter()
            .any(|tag| tag.key == TAG_KEY_NAME || tag.key == TAG_KEY_REF)
    }

    fn way_intersects_bbox(way_nodes: &[Vec<LatLong>], bbox: &BoundingBox) -> bool {
        // Simple implementation - check if any node is within the bbox
        way_nodes.iter().any(|segment| {
            segment
                .iter()
                .any(|node| bbox.contains(node.latitude, node.longitude))
        })
    }
}

impl Drop for MapFile {
    fn drop(&mut self) {
        self.close_file_channel();
    }
}

impl MapFile {
    fn process_block(
        &self,
        query_parameters: &QueryParameters,
        sub_file_parameter: &SubFileParameter,
        bounding_box: &BoundingBox,
        tile_latitude: f64,
        tile_longitude: f64,
        selector: Selector,
        read_buffer: &mut ReadBuffer<impl Read + Seek>,
    ) -> Result<Option<PoiWayBundle>, MapFileException> {
        if !self.process_block_signature(read_buffer)? {
            return Ok(None);
        }

        let zoom_table = self.read_zoom_table(sub_file_parameter, read_buffer)?;
        let zoom_table_row =
            query_parameters.query_zoom_level - sub_file_parameter.zoom_level_min as i32;
        let pois_on_query_zoom_level = zoom_table[zoom_table_row as usize][0] as usize;
        let ways_on_query_zoom_level = zoom_table[zoom_table_row as usize][1] as usize;

        // Get first way offset
        let first_way_offset = read_buffer.read_unsigned_int()? as i32;
        if first_way_offset < 0 {
            return Err(MapFileException::new(format!(
                "{}{}",
                INVALID_FIRST_WAY_OFFSET, first_way_offset
            )));
        }

        let first_way_offset = first_way_offset + read_buffer.get_buffer_position() as i32;
        if first_way_offset > read_buffer.get_buffer_size() as i32 {
            return Err(MapFileException::new(format!(
                "{}{}",
                INVALID_FIRST_WAY_OFFSET, first_way_offset
            )));
        }

        let filter_required =
            query_parameters.query_zoom_level > sub_file_parameter.base_zoom_level as i32;

        // Process POIs
        let pois = self.process_pois(
            tile_latitude,
            tile_longitude,
            pois_on_query_zoom_level,
            bounding_box,
            filter_required,
            read_buffer,
        )?;

        let ways = if matches!(selector, Selector::Pois) {
            Vec::new()
        } else {
            if read_buffer.get_buffer_position() > first_way_offset as usize {
                return Err(MapFileException::new(format!(
                    "invalid buffer position: {}",
                    read_buffer.get_buffer_position()
                )));
            }

            read_buffer.set_buffer_position(first_way_offset as usize);

            self.process_ways(
                query_parameters,
                ways_on_query_zoom_level,
                bounding_box,
                filter_required,
                tile_latitude,
                tile_longitude,
                selector,
                read_buffer,
            )?
        };

        Ok(Some(PoiWayBundle::new(pois, ways)))
    }

    fn process_blocks(
        &mut self,
        query_parameters: &QueryParameters,
        sub_file_parameter: &SubFileParameter,
        bounding_box: &BoundingBox,
        selector: Selector,
    ) -> Result<MapReadResult, MapFileException> {
        let mut query_is_water = true;
        let mut query_read_water_info = false;
        let mut result = MapReadResult {
            poi_way_bundles: Vec::new(),
            is_water: false,
        };

        info!(
            "Processing blocks from {} to {} (x) and {} to {} (y)",
            query_parameters.from_block_x,
            query_parameters.to_block_x,
            query_parameters.from_block_y,
            query_parameters.to_block_y
        );

        // Process blocks from top to bottom and left to right
        for row in query_parameters.from_block_y..=query_parameters.to_block_y {
            for column in query_parameters.from_block_x..=query_parameters.to_block_x {
                let block_number = row * sub_file_parameter.blocks_width + column;
                info!(
                    "Processing block {}, at row {} column {}",
                    block_number, row, column
                );

                // Get current index entry
                let current_block_index_entry = match self
                    .database_index_cache
                    .as_mut()
                    .ok_or_else(|| MapFileException::new("Missing index cache"))?
                    .get_index_entry(&sub_file_parameter, block_number)
                {
                    Ok(entry) => entry,
                    Err(e) => {
                        warn!("Error getting index entry: {}", e);
                        continue; // Skip this block on error
                    }
                };

                // Check water info
                if query_is_water {
                    query_is_water &= (current_block_index_entry & BITMASK_INDEX_WATER) != 0;
                    query_read_water_info = true;
                }

                // Get and check block pointer
                let current_block_pointer = current_block_index_entry & BITMASK_INDEX_OFFSET;
                info!("Block pointer: {}", current_block_pointer);

                // Skip blocks with invalid pointers, but log it
                if current_block_pointer == 0 {
                    warn!("Skipping block with zero pointer");
                    continue;
                }
                if current_block_pointer > sub_file_parameter.sub_file_size {
                    warn!(
                        "Skipping block with pointer > sub_file_size: {} > {}",
                        current_block_pointer, sub_file_parameter.sub_file_size
                    );
                    continue;
                }

                // Get next block pointer
                let next_block_pointer = if block_number + 1 == sub_file_parameter.number_of_blocks
                {
                    sub_file_parameter.sub_file_size
                } else {
                    match self
                        .database_index_cache
                        .as_mut()
                        .unwrap()
                        .get_index_entry(&sub_file_parameter, block_number + 1)
                    {
                        Ok(next_entry) => {
                            let next_ptr = next_entry & BITMASK_INDEX_OFFSET;
                            if next_ptr > sub_file_parameter.sub_file_size {
                                warn!(
                                    "Next block pointer > sub_file_size: {} > {}",
                                    next_ptr, sub_file_parameter.sub_file_size
                                );
                                continue; // Skip if next pointer is invalid
                            }
                            next_ptr
                        }
                        Err(e) => {
                            warn!("Error getting next index entry: {}", e);
                            continue;
                        }
                    }
                };

                // Calculate block size
                let current_block_size = (next_block_pointer - current_block_pointer) as usize;
                info!("Block size: {}", current_block_size);
                if current_block_size == 0 {
                    warn!("Skipping block with zero size");
                    continue;
                }

                // Read and process block
                let mut read_buffer = match ReadBuffer::new(self.file.try_clone()?) {
                    read_buffer => read_buffer,
                };

                let file_position =
                    (sub_file_parameter.start_address + current_block_pointer) as u64;
                info!("Reading from file position: {}", file_position);
                match read_buffer.read_from_file_at_offset(file_position, current_block_size) {
                    Ok(success) => {
                        if !success {
                            warn!("Failed to read from file");
                            continue;
                        }
                    }
                    Err(e) => {
                        warn!("Error reading from file: {}", e);
                        continue;
                    }
                }

                let tile_latitude = MercatorProjection::tile_y_to_latitude(
                    sub_file_parameter.boundary_tile_top + row,
                    sub_file_parameter.base_zoom_level,
                );
                let tile_longitude = MercatorProjection::tile_x_to_longitude(
                    sub_file_parameter.boundary_tile_left + column,
                    sub_file_parameter.base_zoom_level,
                );

                info!(
                    "Processing block at tile coordinates: lat={}, lon={}",
                    tile_latitude, tile_longitude
                );
                match self.process_block(
                    query_parameters,
                    sub_file_parameter,
                    bounding_box,
                    tile_latitude,
                    tile_longitude,
                    selector,
                    &mut read_buffer,
                ) {
                    Ok(Some(bundle)) => {
                        info!(
                            "Found bundle with {} POIs and {} ways",
                            bundle.pois.len(),
                            bundle.ways.len()
                        );
                        result.poi_way_bundles.push(bundle);
                    }
                    Ok(None) => {
                        info!("No bundle found for this block");
                    }
                    Err(e) => {
                        warn!("Error processing block: {}", e);
                        continue;
                    }
                }
            }
        }

        if query_is_water && query_read_water_info {
            result.is_water = true;
        }

        info!(
            "Processed all blocks, found {} bundles",
            result.poi_way_bundles.len()
        );
        Ok(result)
    }

    pub fn read_map_data(&mut self, tile: &Tile) -> Result<MapReadResult, MapFileException> {
        self.read_map_data_impl(tile, tile, Selector::All)
    }

    pub fn read_poi_data(&mut self, tile: &Tile) -> Result<MapReadResult, MapFileException> {
        self.read_map_data_impl(tile, tile, Selector::Pois)
    }

    pub fn read_named_items(&mut self, tile: &Tile) -> Result<MapReadResult, MapFileException> {
        self.read_map_data_impl(tile, tile, Selector::Named)
    }

    fn read_map_data_impl(
        &mut self,
        upper_left: &Tile,
        lower_right: &Tile,
        selector: Selector,
    ) -> Result<MapReadResult, MapFileException> {
        if upper_left.tile_x > lower_right.tile_x || upper_left.tile_y > lower_right.tile_y {
            return Err(MapFileException::new(
                "upperLeft tile must be above and left of lowerRight tile",
            ));
        }

        // Get all the data we need from header first
        let query_zoom_level = self.header.get_query_zoom_level(upper_left.zoom_level) as i32;
        let sub_file_parameter = self
            .header
            .get_sub_file_parameter(query_zoom_level as usize)
            .ok_or_else(|| {
                MapFileException::new(format!("no sub-file for zoom level: {}", query_zoom_level))
            })?
            .clone(); // Clone the SubFileParameter to avoid borrowing self.header

        // Create and populate query parameters
        let mut query_parameters = QueryParameters::new();
        query_parameters.query_zoom_level = query_zoom_level;
        query_parameters.calculate_base_tiles(upper_left, lower_right, &sub_file_parameter);
        query_parameters.calculate_blocks(&sub_file_parameter);

        // Create bounding box
        let bounding_box = Tile::get_bounding_box_range(upper_left, lower_right);

        // Now process blocks
        self.process_blocks(
            &query_parameters,
            &sub_file_parameter,
            &bounding_box,
            selector,
        )
    }
}
