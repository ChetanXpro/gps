use crate::errors::MapFileException;
use crate::header::MapFileInfoBuilder;
use crate::reader::ReadBuffer;
use crate::types::{BoundingBox, Tag};
use std::io::{Read, Seek};

const BINARY_OSM_MAGIC_BYTE: &str = "mapsforge binary OSM";
const HEADER_SIZE_MAX: i32 = 1000000;
const HEADER_SIZE_MIN: i32 = 70;
const MERCATOR: &str = "Mercator";
const SUPPORTED_FILE_VERSION_MIN: i32 = 3;
const SUPPORTED_FILE_VERSION_MAX: i32 = 5;

pub struct RequiredFields;

impl RequiredFields {
    pub fn read_magic_byte<R: Read + Seek>(
        read_buffer: &mut ReadBuffer<R>,
    ) -> Result<(), MapFileException> {
        const BINARY_OSM_MAGIC_BYTE: &str = "mapsforge binary OSM";
        let magic_byte_length = BINARY_OSM_MAGIC_BYTE.len();

        // Read the magic byte directly with known length, not as a length-prefixed string
        if !read_buffer.read_from_file(magic_byte_length + 4)? {
            return Err(MapFileException::new("reading magic byte has failed"));
        }

        let magic_byte = read_buffer.read_utf8_encoded_string_with_length(magic_byte_length)?;

        if magic_byte != BINARY_OSM_MAGIC_BYTE {
            return Err(MapFileException::new(format!(
                "invalid magic byte: {} (expected: {})",
                magic_byte, BINARY_OSM_MAGIC_BYTE
            )));
        }

        Ok(())
    }

    pub fn read_remaining_header<R: Read + Seek>(
        read_buffer: &mut ReadBuffer<R>,
    ) -> Result<(), MapFileException> {
        let remaining_header_size = read_buffer.read_int()?;
        if remaining_header_size < HEADER_SIZE_MIN || remaining_header_size > HEADER_SIZE_MAX {
            return Err(MapFileException::new(format!(
                "invalid remaining header size: {}",
                remaining_header_size
            )));
        }

        if !read_buffer.read_from_file(remaining_header_size as usize)? {
            return Err(MapFileException::new(format!(
                "reading header data has failed: {}",
                remaining_header_size
            )));
        }

        Ok(())
    }

    pub fn read_file_version<R: Read + Seek>(
        read_buffer: &mut ReadBuffer<R>,
        map_file_info_builder: &mut MapFileInfoBuilder,
    ) -> Result<(), MapFileException> {
        let file_version = read_buffer.read_int()?;
        if file_version < SUPPORTED_FILE_VERSION_MIN || file_version > SUPPORTED_FILE_VERSION_MAX {
            return Err(MapFileException::new(format!(
                "unsupported file version: {}",
                file_version
            )));
        }
        map_file_info_builder.file_version = file_version;
        Ok(())
    }

    pub fn read_file_size<R: Read + Seek>(
        read_buffer: &mut ReadBuffer<R>,
        file_size: i64,
        map_file_info_builder: &mut MapFileInfoBuilder,
    ) -> Result<(), MapFileException> {
        let header_file_size = read_buffer.read_long()?;
        if header_file_size != file_size {
            return Err(MapFileException::new(format!(
                "invalid file size: {}",
                header_file_size
            )));
        }
        map_file_info_builder.file_size = file_size;
        Ok(())
    }

    pub fn read_bounding_box<R: Read + Seek>(
        read_buffer: &mut ReadBuffer<R>,
        map_file_info_builder: &mut MapFileInfoBuilder,
    ) -> Result<(), MapFileException> {
        let min_latitude = read_buffer.read_int()? as f64 / 1_000_000.0;
        let min_longitude = read_buffer.read_int()? as f64 / 1_000_000.0;
        let max_latitude = read_buffer.read_int()? as f64 / 1_000_000.0;
        let max_longitude = read_buffer.read_int()? as f64 / 1_000_000.0;

        map_file_info_builder.bounding_box = Some(BoundingBox::new(
            min_latitude,
            min_longitude,
            max_latitude,
            max_longitude,
        )?);
        Ok(())
    }

    pub fn read_tile_pixel_size<R: Read + Seek>(
        read_buffer: &mut ReadBuffer<R>,
        map_file_info_builder: &mut MapFileInfoBuilder,
    ) -> Result<(), MapFileException> {
        let tile_pixel_size = read_buffer.read_short()? as i32;
        // If you want to validate against a specific tile size:
        // if tile_pixel_size != TILE_SIZE {
        //     return Err(MapFileException::new(format!(
        //         "unsupported tile pixel size: {}", tile_pixel_size
        //     )));
        // }
        map_file_info_builder.tile_pixel_size = tile_pixel_size;
        Ok(())
    }

    pub fn read_map_date<R: Read + Seek>(
        read_buffer: &mut ReadBuffer<R>,
        map_file_info_builder: &mut MapFileInfoBuilder,
    ) -> Result<(), MapFileException> {
        let map_date = read_buffer.read_long()?;
        if map_date < 1200000000000 {
            return Err(MapFileException::new(format!(
                "invalid map date: {}",
                map_date
            )));
        }
        map_file_info_builder.map_date = map_date;
        Ok(())
    }
    pub fn read_poi_tags<R: Read + Seek>(
        read_buffer: &mut ReadBuffer<R>,
        map_file_info_builder: &mut MapFileInfoBuilder,
    ) -> Result<(), MapFileException> {
        let number_of_poi_tags = read_buffer.read_short()? as i32;
        if number_of_poi_tags < 0 {
            return Err(MapFileException::new(format!(
                "invalid number of POI tags: {}",
                number_of_poi_tags
            )));
        }

        let mut poi_tags = Vec::with_capacity(number_of_poi_tags as usize);
        for current_tag_id in 0..number_of_poi_tags {
            let tag = read_buffer.read_utf8_encoded_string()?;
            if tag.is_empty() {
                return Err(MapFileException::new(format!(
                    "POI tag must not be null: {}",
                    current_tag_id
                )));
            }
            poi_tags.push(Tag::from_string(tag));
        }
        map_file_info_builder.poi_tags = poi_tags;
        Ok(())
    }

    pub fn read_projection_name<R: Read + Seek>(
        read_buffer: &mut ReadBuffer<R>,
        map_file_info_builder: &mut MapFileInfoBuilder,
    ) -> Result<(), MapFileException> {
        let projection_name = read_buffer.read_utf8_encoded_string()?;
        if projection_name != MERCATOR {
            return Err(MapFileException::new(format!(
                "unsupported projection: {}",
                projection_name
            )));
        }
        map_file_info_builder.projection_name = projection_name;
        Ok(())
    }

    pub fn read_way_tags<R: Read + Seek>(
        read_buffer: &mut ReadBuffer<R>,
        map_file_info_builder: &mut MapFileInfoBuilder,
    ) -> Result<(), MapFileException> {
        let number_of_way_tags = read_buffer.read_short()? as i32;
        if number_of_way_tags < 0 {
            return Err(MapFileException::new(format!(
                "invalid number of way tags: {}",
                number_of_way_tags
            )));
        }

        let mut way_tags = Vec::with_capacity(number_of_way_tags as usize);
        for current_tag_id in 0..number_of_way_tags {
            let tag = read_buffer.read_utf8_encoded_string()?;
            if tag.is_empty() {
                return Err(MapFileException::new(format!(
                    "way tag must not be null: {}",
                    current_tag_id
                )));
            }
            way_tags.push(Tag::from_string(tag));
        }
        map_file_info_builder.way_tags = way_tags;
        Ok(())
    }
}
