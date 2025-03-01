use std::io::{Read, Seek};

use tracing::{debug, error, info};

use crate::{
    errors::MapFileException,
    optional_field::OptionalFields,
    reader::ReadBuffer,
    required_field::RequiredFields,
    types::{BoundingBox, LatLong, Tag},
    MercatorProjection,
};

pub const BYTES_PER_INDEX_ENTRY: u8 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubFileParameter {
    pub base_zoom_level: u8,
    pub blocks_height: i64,
    pub blocks_width: i64,
    pub boundary_tile_bottom: i64,
    pub boundary_tile_left: i64,
    pub boundary_tile_right: i64,
    pub boundary_tile_top: i64,
    pub index_end_address: i64,
    pub index_start_address: i64,
    pub number_of_blocks: i64,
    pub start_address: i64,
    pub sub_file_size: i64,
    pub zoom_level_max: u8,
    pub zoom_level_min: u8,
}

impl SubFileParameter {
    pub const BYTES_PER_INDEX_ENTRY: u8 = 5;

    pub fn hash_code(&self) -> i32 {
        let mut result = 7i32;

        // Add logging for hash calculation
        debug!("Calculating hash code:");
        debug!("  start_address: {}", self.start_address);
        debug!("  sub_file_size: {}", self.sub_file_size);
        debug!("  base_zoom_level: {}", self.base_zoom_level);

        result = result
            .wrapping_mul(31)
            .wrapping_add((self.start_address ^ (self.start_address >> 32)) as i32);
        debug!("  After start_address: {}", result);

        result = result
            .wrapping_mul(31)
            .wrapping_add((self.sub_file_size ^ (self.sub_file_size >> 32)) as i32);
        debug!("  After sub_file_size: {}", result);

        result = result
            .wrapping_mul(31)
            .wrapping_add(self.base_zoom_level as i32);
        debug!("  Final hash: {}", result);

        result
    }
}
#[derive(Default)]
pub struct SubFileParameterBuilder {
    pub base_zoom_level: u8,
    pub bounding_box: Option<BoundingBox>,
    pub index_start_address: i64,
    pub start_address: i64,
    pub sub_file_size: i64,
    pub zoom_level_max: u8,
    pub zoom_level_min: u8,
}

impl SubFileParameterBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn build(self) -> Result<SubFileParameter, MapFileException> {
        info!("Building SubFileParameter with Java-compatible calculations");

        // Get bounding box
        let bounding_box = match self.bounding_box {
            Some(ref bb) => bb.clone(),
            None => return Err(MapFileException::new("bounding box is required")),
        };

        // Calculate boundary tiles
        let boundary_tile_bottom =
            MercatorProjection::latitude_to_tile_y(bounding_box.min_latitude, self.base_zoom_level);
        let boundary_tile_left = MercatorProjection::longitude_to_tile_x(
            bounding_box.min_longitude,
            self.base_zoom_level,
        );
        let boundary_tile_top =
            MercatorProjection::latitude_to_tile_y(bounding_box.max_latitude, self.base_zoom_level);
        let boundary_tile_right = MercatorProjection::longitude_to_tile_x(
            bounding_box.max_longitude,
            self.base_zoom_level,
        );

        // Calculate blocks like in Java (using wrapping operations for consistency)
        let blocks_width = boundary_tile_right
            .wrapping_sub(boundary_tile_left)
            .wrapping_add(1);
        let blocks_height = boundary_tile_bottom
            .wrapping_sub(boundary_tile_top)
            .wrapping_add(1);

        // Calculate number of blocks
        let number_of_blocks = blocks_width.wrapping_mul(blocks_height);

        // Calculate index end address directly like in Java code
        let index_end_address = self
            .index_start_address
            .wrapping_add(number_of_blocks.wrapping_mul(BYTES_PER_INDEX_ENTRY as i64));

        Ok(SubFileParameter {
            base_zoom_level: self.base_zoom_level,
            blocks_height,
            blocks_width,
            boundary_tile_bottom,
            boundary_tile_left,
            boundary_tile_right,
            boundary_tile_top,
            index_end_address,
            index_start_address: self.index_start_address,
            number_of_blocks,
            start_address: self.start_address,
            sub_file_size: self.sub_file_size,
            zoom_level_max: self.zoom_level_max,
            zoom_level_min: self.zoom_level_min,
        })
    }
}

#[derive(Debug)]
pub struct MapFileInfo {
    pub bounding_box: BoundingBox,
    pub comment: Option<String>,
    pub created_by: Option<String>,
    pub debug_file: bool,
    pub file_size: i64,
    pub file_version: i32,
    pub languages_preference: Option<String>,
    pub map_date: i64,
    pub number_of_sub_files: u8,
    pub poi_tags: Vec<Tag>,
    pub projection_name: String,
    pub start_position: Option<LatLong>,
    pub start_zoom_level: Option<u8>,
    pub tile_pixel_size: i32,
    pub way_tags: Vec<Tag>,
    pub zoom_level_min: u8,
    pub zoom_level_max: u8,
}

#[derive(Default)]
pub struct MapFileInfoBuilder {
    pub bounding_box: Option<BoundingBox>,
    pub file_size: i64,
    pub file_version: i32,
    pub map_date: i64,
    pub number_of_sub_files: u8,
    pub optional_fields: OptionalFields,
    pub poi_tags: Vec<Tag>,
    pub projection_name: String,
    pub tile_pixel_size: i32,
    pub way_tags: Vec<Tag>,
    pub zoom_level_min: u8,
    pub zoom_level_max: u8,
}

impl MapFileInfoBuilder {
    pub fn new() -> Self {
        Self {
            bounding_box: None,
            file_size: 0,
            file_version: 0,
            map_date: 0,
            number_of_sub_files: 0,
            optional_fields: OptionalFields::default(),
            poi_tags: Vec::new(),
            projection_name: String::new(),
            tile_pixel_size: 0,
            way_tags: Vec::new(),
            zoom_level_min: 0,
            zoom_level_max: 0,
        }
    }

    pub fn build(self) -> Result<MapFileInfo, MapFileException> {
        let bounding_box = self
            .bounding_box
            .ok_or_else(|| MapFileException::new("bounding box is required"))?;

        Ok(MapFileInfo {
            bounding_box,
            comment: self.optional_fields.comment,
            created_by: self.optional_fields.created_by,
            debug_file: self.optional_fields.is_debug_file,
            file_size: self.file_size,
            file_version: self.file_version,
            languages_preference: self.optional_fields.languages_preference,
            map_date: self.map_date,
            number_of_sub_files: self.number_of_sub_files,
            poi_tags: self.poi_tags,
            projection_name: self.projection_name,
            start_position: self.optional_fields.start_position,
            start_zoom_level: self.optional_fields.start_zoom_level,
            tile_pixel_size: self.tile_pixel_size,
            way_tags: self.way_tags,
            zoom_level_min: self.zoom_level_min,
            zoom_level_max: self.zoom_level_max,
        })
    }
}

pub struct MapFileHeader {
    map_file_info: Option<MapFileInfo>,
    sub_file_parameters: Option<Vec<SubFileParameter>>,
    zoom_level_maximum: u8,
    zoom_level_minimum: u8,
}

impl MapFileHeader {
    pub const BASE_ZOOM_LEVEL_MAX: i32 = 20;
    const HEADER_SIZE_MIN: i32 = 70;
    const SIGNATURE_LENGTH_INDEX: u8 = 16;
    const SPACE: char = ' ';

    pub fn new() -> Self {
        Self {
            map_file_info: None,
            sub_file_parameters: None,
            zoom_level_maximum: 0,
            zoom_level_minimum: u8::MAX,
        }
    }

    pub fn get_map_file_info(&self) -> Option<&MapFileInfo> {
        self.map_file_info.as_ref()
    }

    pub fn get_query_zoom_level(&self, zoom_level: u8) -> u8 {
        if zoom_level > self.zoom_level_maximum {
            self.zoom_level_maximum
        } else if zoom_level < self.zoom_level_minimum {
            self.zoom_level_minimum
        } else {
            zoom_level
        }
    }

    pub fn get_sub_file_parameter(&self, query_zoom_level: usize) -> Option<&SubFileParameter> {
        self.sub_file_parameters.as_ref().and_then(|params| {
            // Ensure we're within the valid range of parameters
            if query_zoom_level >= params.len() {
                return None;
            }

            // Attempt to get the parameter, working backwards if needed
            for offset in 0..=query_zoom_level {
                let index = query_zoom_level - offset;
                if let Some(param) = params.iter().find(|p| {
                    index >= p.zoom_level_min as usize && index <= p.zoom_level_max as usize
                }) {
                    return Some(param);
                }
            }
            None
        })
    }

    pub fn read_header<R: Read + Seek>(
        &mut self,
        read_buffer: &mut ReadBuffer<R>,
        file_size: i64,
    ) -> Result<(), MapFileException> {
        RequiredFields::read_magic_byte(read_buffer)?;
        RequiredFields::read_remaining_header(read_buffer)?;

        let mut map_file_info_builder = MapFileInfoBuilder::new();

        RequiredFields::read_file_version(read_buffer, &mut map_file_info_builder)?;
        RequiredFields::read_file_size(read_buffer, file_size, &mut map_file_info_builder)?;
        RequiredFields::read_map_date(read_buffer, &mut map_file_info_builder)?;
        RequiredFields::read_bounding_box(read_buffer, &mut map_file_info_builder)?;
        RequiredFields::read_tile_pixel_size(read_buffer, &mut map_file_info_builder)?;
        RequiredFields::read_projection_name(read_buffer, &mut map_file_info_builder)?;

        let mut optional_fields = OptionalFields::new(read_buffer.read_byte()?);
        optional_fields.read_optional_fields(read_buffer)?;
        map_file_info_builder.optional_fields = optional_fields;
        RequiredFields::read_poi_tags(read_buffer, &mut map_file_info_builder)?;
        RequiredFields::read_way_tags(read_buffer, &mut map_file_info_builder)?;

        self.read_sub_file_parameters(read_buffer, file_size, &mut map_file_info_builder)?;

        self.map_file_info = Some(map_file_info_builder.build()?);
        Ok(())
    }

    fn read_sub_file_parameters<R: Read + Seek>(
        &mut self,
        read_buffer: &mut ReadBuffer<R>,
        file_size: i64,
        map_file_info_builder: &mut MapFileInfoBuilder,
    ) -> Result<(), MapFileException> {
        let number_of_sub_files = read_buffer.read_byte()?;
        if number_of_sub_files < 1 {
            return Err(MapFileException::new(format!(
                "invalid number of sub-files: {}",
                number_of_sub_files
            )));
        }
        map_file_info_builder.number_of_sub_files = number_of_sub_files;

        let mut temp_sub_file_parameters = Vec::with_capacity(number_of_sub_files as usize);

        for current_sub_file in 0..number_of_sub_files {
            let mut builder = SubFileParameterBuilder::new();

            // Read base zoom level
            let base_zoom_level = read_buffer.read_byte()?;
            if base_zoom_level as i32 > Self::BASE_ZOOM_LEVEL_MAX {
                return Err(MapFileException::new(format!(
                    "invalid base zoom level: {}",
                    base_zoom_level
                )));
            }
            builder.base_zoom_level = base_zoom_level;

            // Read min zoom level
            let zoom_level_min = read_buffer.read_byte()?;
            if zoom_level_min > 22 {
                return Err(MapFileException::new(format!(
                    "invalid minimum zoom level: {}",
                    zoom_level_min
                )));
            }
            builder.zoom_level_min = zoom_level_min;

            // Read max zoom level
            let zoom_level_max = read_buffer.read_byte()?;
            if zoom_level_max > 22 {
                return Err(MapFileException::new(format!(
                    "invalid maximum zoom level: {}",
                    zoom_level_max
                )));
            }
            builder.zoom_level_max = zoom_level_max;

            // Check zoom level range
            if zoom_level_min > zoom_level_max {
                return Err(MapFileException::new(format!(
                    "invalid zoom level range: {} {}",
                    zoom_level_min,
                    Self::SPACE
                )));
            }

            // Read start address
            let start_address = read_buffer.read_long()?;
            if start_address < Self::HEADER_SIZE_MIN as i64 || start_address >= file_size {
                return Err(MapFileException::new(format!(
                    "invalid start address: {}",
                    start_address
                )));
            }
            builder.start_address = start_address;

            let index_start_address = if map_file_info_builder.optional_fields.is_debug_file {
                start_address + Self::SIGNATURE_LENGTH_INDEX as i64
            } else {
                start_address
            };
            builder.index_start_address = index_start_address;

            // Read sub-file size
            let sub_file_size = read_buffer.read_long()?;
            if sub_file_size < 1 {
                return Err(MapFileException::new(format!(
                    "invalid sub-file size: {}",
                    sub_file_size
                )));
            }
            builder.sub_file_size = sub_file_size;

            builder.bounding_box = map_file_info_builder.bounding_box.clone();

            let sub_file_parameter = builder.build()?;
            temp_sub_file_parameters.push(sub_file_parameter);

            // Update global zoom levels
            if self.zoom_level_minimum > zoom_level_min {
                self.zoom_level_minimum = zoom_level_min;
                map_file_info_builder.zoom_level_min = zoom_level_min;
            }
            if self.zoom_level_maximum < zoom_level_max {
                self.zoom_level_maximum = zoom_level_max;
                map_file_info_builder.zoom_level_max = zoom_level_max;
            }
        }

        // Create a dense array of parameters covering all zoom levels
        let mut sub_file_parameters = Vec::with_capacity(self.zoom_level_maximum as usize + 1);

        // For each zoom level, find the first matching sub-file parameter
        for zoom_level in 0..=self.zoom_level_maximum as usize {
            if let Some(matching_param) = temp_sub_file_parameters.iter().find(|p| {
                zoom_level >= p.zoom_level_min as usize && zoom_level <= p.zoom_level_max as usize
            }) {
                sub_file_parameters.push(matching_param.clone());
            } else {
                // If no matching parameter is found, use the last valid parameter
                if let Some(last_valid_param) = temp_sub_file_parameters.last() {
                    sub_file_parameters.push(last_valid_param.clone());
                } else {
                    return Err(MapFileException::new("No valid sub-file parameters found"));
                }
            }
        }

        self.sub_file_parameters = Some(sub_file_parameters);
        Ok(())
    }
}
