use crate::{types::Tag, MapFileException};
use std::io::{self, Read, Seek, SeekFrom};

const CHARSET_UTF8: &str = "UTF-8";
const MAXIMUM_BUFFER_SIZE: usize = 1024 * 1024 * 10; // Similar to Java's Parameters.MAXIMUM_BUFFER_SIZE

pub struct ReadBuffer<R: Read + Seek> {
    buffer_data: Vec<u8>,
    buffer_position: usize,
    input_channel: R,
    tag_ids: Vec<i32>,
}

impl<R: Read + Seek> ReadBuffer<R> {
    pub fn new(input_channel: R) -> Self {
        Self {
            buffer_data: Vec::new(),
            buffer_position: 0,
            input_channel,
            tag_ids: Vec::new(),
        }
    }

    pub fn read_byte(&mut self) -> Result<u8, MapFileException> {
        if self.buffer_position >= self.buffer_data.len() {
            return Err(MapFileException::new("Buffer overflow when reading byte"));
        }
        let byte = self.buffer_data[self.buffer_position];
        self.buffer_position += 1;
        Ok(byte)
    }

    pub fn read_float(&mut self) -> Result<f32, MapFileException> {
        Ok(f32::from_bits(self.read_int()? as u32))
    }

    pub fn read_from_file(&mut self, length: usize) -> Result<bool, MapFileException> {
        // ensure the read buffer is large enough
        if length > MAXIMUM_BUFFER_SIZE {
            return Ok(false);
        }

        self.buffer_data.resize(length, 0);
        self.buffer_position = 0;

        match self
            .input_channel
            .read_exact(&mut self.buffer_data[..length])
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn read_from_file_at_offset(
        &mut self,
        offset: u64,
        length: usize,
    ) -> Result<bool, MapFileException> {
        if length > MAXIMUM_BUFFER_SIZE {
            return Ok(false);
        }

        self.buffer_data.resize(length, 0);
        self.buffer_position = 0;

        self.input_channel.seek(SeekFrom::Start(offset))?;
        match self
            .input_channel
            .read_exact(&mut self.buffer_data[..length])
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn read_int(&mut self) -> Result<i32, MapFileException> {
        if self.buffer_position + 4 > self.buffer_data.len() {
            return Err(MapFileException::new("Buffer overflow when reading int"));
        }
        self.buffer_position += 4;
        Ok(i32::from_be_bytes(
            self.buffer_data[self.buffer_position - 4..self.buffer_position]
                .try_into()
                .unwrap(),
        ))
    }

    pub fn read_long(&mut self) -> Result<i64, MapFileException> {
        if self.buffer_position + 8 > self.buffer_data.len() {
            return Err(MapFileException::new("Buffer overflow when reading long"));
        }
        self.buffer_position += 8;
        Ok(i64::from_be_bytes(
            self.buffer_data[self.buffer_position - 8..self.buffer_position]
                .try_into()
                .unwrap(),
        ))
    }

    pub fn read_short(&mut self) -> Result<i16, MapFileException> {
        if self.buffer_position + 2 > self.buffer_data.len() {
            return Err(MapFileException::new("Buffer overflow when reading short"));
        }
        self.buffer_position += 2;
        Ok(i16::from_be_bytes(
            self.buffer_data[self.buffer_position - 2..self.buffer_position]
                .try_into()
                .unwrap(),
        ))
    }

    pub fn read_signed_int(&mut self) -> Result<i32, MapFileException> {
        let mut variable_byte_decode = 0;
        let mut variable_byte_shift = 0;

        while (self.buffer_data[self.buffer_position] & 0x80) != 0 {
            if self.buffer_position >= self.buffer_data.len() {
                return Err(MapFileException::new(
                    "Buffer overflow when reading signed int",
                ));
            }
            variable_byte_decode |=
                ((self.buffer_data[self.buffer_position] & 0x7f) as i32) << variable_byte_shift;
            self.buffer_position += 1;
            variable_byte_shift += 7;
        }

        if self.buffer_position >= self.buffer_data.len() {
            return Err(MapFileException::new(
                "Buffer overflow when reading signed int",
            ));
        }

        let result = if (self.buffer_data[self.buffer_position] & 0x40) != 0 {
            -(variable_byte_decode
                | ((self.buffer_data[self.buffer_position] & 0x3f) as i32) << variable_byte_shift)
        } else {
            variable_byte_decode
                | ((self.buffer_data[self.buffer_position] & 0x3f) as i32) << variable_byte_shift
        };
        self.buffer_position += 1;
        Ok(result)
    }

    pub fn read_tags(
        &mut self,
        tags_array: &[Tag],
        number_of_tags: u8,
    ) -> Result<Vec<Tag>, MapFileException> {
        self.tag_ids.clear();
        let max_tag = tags_array.len();

        for _ in 0..number_of_tags {
            let tag_id = self.read_unsigned_int()? as usize;
            if tag_id >= max_tag {
                return Err(MapFileException::new(format!("invalid tag ID: {}", tag_id)));
            }
            self.tag_ids.push(tag_id as i32);
        }

        let mut result = Vec::new();
        for &tag_id in &self.tag_ids {
            let tag = &tags_array[tag_id as usize];
            result.push(tag.clone());
        }

        Ok(result)
    }

    pub fn read_unsigned_int(&mut self) -> Result<u32, MapFileException> {
        let mut variable_byte_decode = 0;
        let mut variable_byte_shift = 0;

        while (self.buffer_data[self.buffer_position] & 0x80) != 0 {
            if self.buffer_position >= self.buffer_data.len() {
                return Err(MapFileException::new(
                    "Buffer overflow when reading unsigned int",
                ));
            }
            variable_byte_decode |=
                ((self.buffer_data[self.buffer_position] & 0x7f) as u32) << variable_byte_shift;
            self.buffer_position += 1;
            variable_byte_shift += 7;
        }

        if self.buffer_position >= self.buffer_data.len() {
            return Err(MapFileException::new(
                "Buffer overflow when reading unsigned int",
            ));
        }

        let result = variable_byte_decode
            | ((self.buffer_data[self.buffer_position] as u32) << variable_byte_shift);
        self.buffer_position += 1;
        Ok(result)
    }

    pub fn read_utf8_encoded_string(&mut self) -> Result<String, MapFileException> {
        let length = self.read_unsigned_int()? as usize;
        self.read_utf8_encoded_string_with_length(length)
    }

    pub fn read_utf8_encoded_string_with_length(
        &mut self,
        string_length: usize,
    ) -> Result<String, MapFileException> {
        if string_length > 0 && self.buffer_position + string_length <= self.buffer_data.len() {
            self.buffer_position += string_length;
            String::from_utf8(
                self.buffer_data[self.buffer_position - string_length..self.buffer_position]
                    .to_vec(),
            )
            .map_err(|e| e.into())
        } else {
            Err(MapFileException::new(format!(
                "invalid string length: {}",
                string_length
            )))
        }
    }

    pub fn get_buffer_position(&self) -> usize {
        self.buffer_position
    }

    pub fn get_buffer_size(&self) -> usize {
        self.buffer_data.len()
    }

    pub fn set_buffer_position(&mut self, position: usize) {
        self.buffer_position = position;
    }

    pub fn skip_bytes(&mut self, bytes: usize) {
        self.buffer_position += bytes;
    }
}
