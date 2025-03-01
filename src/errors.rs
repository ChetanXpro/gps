use std::fmt;
use std::io;
use std::time::SystemTimeError;

#[derive(Debug)]
pub struct MapFileException {
    message: String,
}

impl MapFileException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for MapFileException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MapFileException: {}", self.message)
    }
}

impl std::error::Error for MapFileException {}

// Add conversion from io::Error to MapFileException
impl From<io::Error> for MapFileException {
    fn from(err: io::Error) -> Self {
        MapFileException::new(format!("IO error: {}", err))
    }
}

// Add conversion from string UTF-8 errors
impl From<std::string::FromUtf8Error> for MapFileException {
    fn from(err: std::string::FromUtf8Error) -> Self {
        MapFileException::new(format!("UTF-8 error: {}", err))
    }
}

// Add conversion from String to MapFileException
impl From<String> for MapFileException {
    fn from(message: String) -> Self {
        MapFileException::new(message)
    }
}

// Add conversion from &str to MapFileException
impl From<&str> for MapFileException {
    fn from(message: &str) -> Self {
        MapFileException::new(message)
    }
}

impl From<SystemTimeError> for MapFileException {
    fn from(err: SystemTimeError) -> Self {
        MapFileException::new(format!("System time error: {}", err))
    }
}
