pub struct Deserializer;

impl Deserializer {
    /// Converts five bytes of a byte array to an unsigned long.
    /// The byte order is big-endian.
    pub fn get_five_bytes_long(buffer: &[u8], offset: usize) -> i64 {
        ((buffer[offset] as i64 & 0xff) << 32)
            | ((buffer[offset + 1] as i64 & 0xff) << 24)
            | ((buffer[offset + 2] as i64 & 0xff) << 16)
            | ((buffer[offset + 3] as i64 & 0xff) << 8)
            | (buffer[offset + 4] as i64 & 0xff)
    }

    /// Converts four bytes of a byte array to a signed int.
    /// The byte order is big-endian.
    pub fn get_int(buffer: &[u8], offset: usize) -> i32 {
        ((buffer[offset] as i32) << 24)
            | ((buffer[offset + 1] as i32 & 0xff) << 16)
            | ((buffer[offset + 2] as i32 & 0xff) << 8)
            | (buffer[offset + 3] as i32 & 0xff)
    }

    /// Converts eight bytes of a byte array to a signed long.
    /// The byte order is big-endian.
    pub fn get_long(buffer: &[u8], offset: usize) -> i64 {
        ((buffer[offset] as i64 & 0xff) << 56)
            | ((buffer[offset + 1] as i64 & 0xff) << 48)
            | ((buffer[offset + 2] as i64 & 0xff) << 40)
            | ((buffer[offset + 3] as i64 & 0xff) << 32)
            | ((buffer[offset + 4] as i64 & 0xff) << 24)
            | ((buffer[offset + 5] as i64 & 0xff) << 16)
            | ((buffer[offset + 6] as i64 & 0xff) << 8)
            | (buffer[offset + 7] as i64 & 0xff)
    }

    /// Converts two bytes of a byte array to a signed int.
    /// The byte order is big-endian.
    pub fn get_short(buffer: &[u8], offset: usize) -> i16 {
        ((buffer[offset] as i16) << 8) | (buffer[offset + 1] as i16 & 0xff)
    }
}
