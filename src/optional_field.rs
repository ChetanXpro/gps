use std::io::{Read, Seek};

use crate::{reader::ReadBuffer, LatLong, MapFileException};

pub struct OptionalFields {
    pub comment: Option<String>,
    pub created_by: Option<String>,
    pub is_debug_file: bool,
    pub has_start_position: bool,
    pub has_start_zoom_level: bool,
    pub has_languages_preference: bool,
    pub has_comment: bool,
    pub has_created_by: bool,
    pub languages_preference: Option<String>,
    pub start_position: Option<LatLong>,
    pub start_zoom_level: Option<u8>,
}

impl Default for OptionalFields {
    fn default() -> Self {
        Self {
            comment: None,
            created_by: None,
            is_debug_file: false,
            has_start_position: false,
            has_start_zoom_level: false,
            has_languages_preference: false,
            has_comment: false,
            has_created_by: false,
            languages_preference: None,
            start_position: None,
            start_zoom_level: None,
        }
    }
}
impl OptionalFields {
    pub fn new(flags: u8) -> Self {
        Self {
            is_debug_file: (flags & 0x80) != 0,
            has_start_position: (flags & 0x40) != 0,
            has_start_zoom_level: (flags & 0x20) != 0,
            has_languages_preference: (flags & 0x10) != 0,
            has_comment: (flags & 0x08) != 0,
            has_created_by: (flags & 0x04) != 0,
            comment: None,
            created_by: None,
            languages_preference: None,
            start_position: None,
            start_zoom_level: None,
        }
    }

    pub fn read_optional_fields<R: Read + Seek>(
        &mut self,
        read_buffer: &mut ReadBuffer<R>,
    ) -> Result<(), MapFileException> {
        // Read each optional field in order, only if its flag is set
        if self.has_start_position {
            let lat = read_buffer.read_int()? as f64 / 1_000_000.0;
            let lon = read_buffer.read_int()? as f64 / 1_000_000.0;
            self.start_position = Some(LatLong {
                latitude: lat,
                longitude: lon,
            });
        }

        if self.has_start_zoom_level {
            let zoom_level = read_buffer.read_byte()?;
            if zoom_level > 22 {
                return Err(MapFileException::new(format!(
                    "invalid map start zoom level: {}",
                    zoom_level
                )));
            }
            self.start_zoom_level = Some(zoom_level);
        }

        if self.has_languages_preference {
            self.languages_preference = Some(read_buffer.read_utf8_encoded_string()?);
        }

        if self.has_comment {
            self.comment = Some(read_buffer.read_utf8_encoded_string()?);
        }

        if self.has_created_by {
            self.created_by = Some(read_buffer.read_utf8_encoded_string()?);
        }

        Ok(())
    }
}
