use crate::MapFileException;

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min_latitude: f64,
    pub min_longitude: f64,
    pub max_latitude: f64,
    pub max_longitude: f64,
}

impl BoundingBox {
    pub fn new(
        min_latitude: f64,
        min_longitude: f64,
        max_latitude: f64,
        max_longitude: f64,
    ) -> Result<Self, MapFileException> {
        if min_latitude > max_latitude || min_longitude > max_longitude {
            return Err(MapFileException::new("Invalid bounding box coordinates"));
        }
        Ok(Self {
            min_latitude,
            min_longitude,
            max_latitude,
            max_longitude,
        })
    }

    pub fn get_center_point(&self) -> LatLong {
        LatLong {
            latitude: (self.min_latitude + self.max_latitude) / 2.0,
            longitude: (self.min_longitude + self.max_longitude) / 2.0,
        }
    }

    pub fn contains(&self, latitude: f64, longitude: f64) -> bool {
        latitude >= self.min_latitude
            && latitude <= self.max_latitude
            && longitude >= self.min_longitude
            && longitude <= self.max_longitude
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        !(other.min_latitude > self.max_latitude
            || other.max_latitude < self.min_latitude
            || other.min_longitude > self.max_longitude
            || other.max_longitude < self.min_longitude)
    }

    pub fn extend_meters(&self, meters: i32) -> BoundingBox {
        // Rough approximation: 1 degree = 111km at equator
        let degree_delta = (meters as f64) / 111_000.0;
        BoundingBox {
            min_latitude: self.min_latitude - degree_delta,
            min_longitude: self.min_longitude - degree_delta,
            max_latitude: self.max_latitude + degree_delta,
            max_longitude: self.max_longitude + degree_delta,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]

pub struct LatLong {
    pub latitude: f64,
    pub longitude: f64,
}

impl LatLong {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

impl Tag {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }

    pub fn from_string(tag: impl Into<String>) -> Self {
        let tag = tag.into();
        // Assuming the tag string contains both key and value
        Self {
            key: tag.clone(),
            value: tag,
        }
    }
}

pub struct LatLongUtils;

impl LatLongUtils {
    pub const LONGITUDE_MAX: f64 = 180.0;
    pub const LONGITUDE_MIN: f64 = -180.0;
    const CONVERSION_FACTOR: f64 = 1_000_000.0;

    pub fn microdegrees_to_degrees(microdegrees: i32) -> f64 {
        // Simple division without any special rounding
        microdegrees as f64 / Self::CONVERSION_FACTOR
    }

    pub fn degrees_to_microdegrees(degrees: f64) -> i32 {
        // Ensure precise conversion
        (degrees * Self::CONVERSION_FACTOR).round() as i32
    }

    // Approximate equality check for floating-point comparisons
    pub fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }
}
