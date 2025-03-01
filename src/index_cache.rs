use crate::deserializer::Deserializer;

use crate::header::SubFileParameter;
use crate::MapFileException;
use lru::LruCache;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::num::NonZeroUsize;
use tracing::{debug, error, info};

const INDEX_ENTRIES_PER_BLOCK: usize = 128;
const SIZE_OF_INDEX_BLOCK: usize =
    INDEX_ENTRIES_PER_BLOCK * SubFileParameter::BYTES_PER_INDEX_ENTRY as usize;

#[derive(Debug, Eq)]
struct IndexCacheEntryKey {
    sub_file_parameter: SubFileParameter,
    index_block_number: i64,
    hash_code_value: i32,
}

impl IndexCacheEntryKey {
    fn new(sub_file_parameter: SubFileParameter, index_block_number: i64) -> Self {
        let mut key = Self {
            sub_file_parameter,
            index_block_number,
            hash_code_value: 0,
        };
        key.hash_code_value = key.calculate_hash_code();
        key
    }

    fn calculate_hash_code(&self) -> i32 {
        let mut result = 7i32;
        // Use wrapping operations for safe arithmetic
        result = result
            .wrapping_mul(31)
            .wrapping_add(self.sub_file_parameter.hash_code());

        // Safely handle the index block number hash calculation
        let block_hash = (self.index_block_number ^ (self.index_block_number >> 32)) as i32;
        result = result.wrapping_mul(31).wrapping_add(block_hash);

        result
    }
}

impl PartialEq for IndexCacheEntryKey {
    fn eq(&self, other: &Self) -> bool {
        self.sub_file_parameter == other.sub_file_parameter
            && self.index_block_number == other.index_block_number
    }
}

impl std::hash::Hash for IndexCacheEntryKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash_code_value.hash(state);
    }
}

pub struct IndexCache<R: Read + Seek> {
    map: LruCache<IndexCacheEntryKey, Vec<u8>>,
    file_channel: R,
}

impl<R: Read + Seek> IndexCache<R> {
    pub fn new(file_channel: R, capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            map: LruCache::new(capacity),
            file_channel,
        }
    }

    pub fn destroy(&mut self) {
        self.map.clear();
    }

    pub fn get_index_entry(
        &mut self,
        sub_file_parameter: &SubFileParameter,
        block_number: i64,
    ) -> Result<i64, MapFileException> {
        // Check if the block number is out of bounds (similar to Java)
        if block_number >= sub_file_parameter.number_of_blocks {
            return Err(MapFileException::new(format!(
                "invalid block number: {}",
                block_number
            )));
        }

        // Calculate the index block number using normal division
        // Java doesn't check for overflow here
        let index_block_number = block_number / INDEX_ENTRIES_PER_BLOCK as i64;

        let key = IndexCacheEntryKey::new(sub_file_parameter.clone(), index_block_number);

        let index_block = if let Some(block) = self.map.get(&key) {
            block.clone()
        } else {
            // Cache miss, read from file
            // Replicate Java's calculation logic without overflow checks
            let index_block_position = sub_file_parameter.index_start_address
                + index_block_number * SIZE_OF_INDEX_BLOCK as i64;

            let remaining_index_size =
                (sub_file_parameter.index_end_address - index_block_position) as usize;
            let index_block_size = std::cmp::min(SIZE_OF_INDEX_BLOCK, remaining_index_size);

            if index_block_size == 0 {
                return Err(MapFileException::new("invalid index block size"));
            }

            let mut index_block = vec![0u8; index_block_size];

            // Handle any potential file reading errors
            match self
                .file_channel
                .seek(SeekFrom::Start(index_block_position as u64))
            {
                Ok(_) => {}
                Err(e) => return Err(MapFileException::new(format!("IO error: {}", e))),
            }

            match self.file_channel.read_exact(&mut index_block) {
                Ok(_) => {}
                Err(e) => {
                    // If we have a file too small error, just return 0 like Java silently does
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        return Ok(0);
                    }
                    return Err(MapFileException::new(format!("IO error: {}", e)));
                }
            }

            self.map.put(key, index_block.clone());
            index_block
        };

        // Calculate index entry position within block (using wrapping mul for Java compatibility)
        let index_entry_in_block = block_number % INDEX_ENTRIES_PER_BLOCK as i64;
        let address_in_index_block =
            (index_entry_in_block * SubFileParameter::BYTES_PER_INDEX_ENTRY as i64) as usize;

        // Bounds check to prevent out-of-bounds access
        if address_in_index_block + SubFileParameter::BYTES_PER_INDEX_ENTRY as usize
            > index_block.len()
        {
            return Ok(0); // Return 0 as a fallback like Java would implicitly do
        }

        Ok(Deserializer::get_five_bytes_long(
            &index_block,
            address_in_index_block,
        ))
    }
}
