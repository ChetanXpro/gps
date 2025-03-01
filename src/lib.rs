mod deserializer;
mod errors;
mod header;
mod index_cache;
mod map_data;
pub mod map_file;
mod mercator;
mod optional_field;
mod query_calculations;
mod query_parameters;
mod reader;
mod required_field;
mod tile;
mod types;

// Create a single, consistent public API
pub use deserializer::Deserializer;
pub use errors::MapFileException;
pub use header::{MapFileHeader, MapFileInfo, SubFileParameter};
pub use map_file::MapFile;
pub use map_file::Selector;
pub use mercator::MercatorProjection;
pub use query_parameters::QueryParameters;
pub use tile::Tile;
pub use types::{BoundingBox, LatLong, Tag};

// Re-export these types ONLY from map_data, not from multiple places
pub use map_data::{MapReadResult, PoiWayBundle, PointOfInterest, Way};
