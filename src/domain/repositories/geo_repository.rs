//! Geo Repository Trait
//!
//! Abstract interface for Redis geospatial operations.

use async_trait::async_trait;

use crate::domain::errors::CacheError;

/// Geographic position (longitude, latitude)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoPosition {
    /// Longitude (-180 to 180)
    pub longitude: f64,
    /// Latitude (-85.05112878 to 85.05112878)
    pub latitude: f64,
}

impl GeoPosition {
    /// Create a new GeoPosition
    pub fn new(longitude: f64, latitude: f64) -> Self {
        Self {
            longitude,
            latitude,
        }
    }

    /// Validate the position is within valid bounds
    pub fn is_valid(&self) -> bool {
        self.longitude >= -180.0
            && self.longitude <= 180.0
            && self.latitude >= -85.05112878
            && self.latitude <= 85.05112878
    }
}

/// A member with its geographic position
#[derive(Debug, Clone)]
pub struct GeoMember {
    /// The member name
    pub member: String,
    /// The geographic position
    pub position: GeoPosition,
}

impl GeoMember {
    /// Create a new GeoMember
    pub fn new(member: impl Into<String>, longitude: f64, latitude: f64) -> Self {
        Self {
            member: member.into(),
            position: GeoPosition::new(longitude, latitude),
        }
    }
}

/// Distance unit for geo operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeoUnit {
    /// Meters (default)
    #[default]
    Meters,
    /// Kilometers
    Kilometers,
    /// Miles
    Miles,
    /// Feet
    Feet,
}

impl GeoUnit {
    /// Get the Redis command argument for this unit
    pub fn as_str(&self) -> &'static str {
        match self {
            GeoUnit::Meters => "m",
            GeoUnit::Kilometers => "km",
            GeoUnit::Miles => "mi",
            GeoUnit::Feet => "ft",
        }
    }
}

/// Search center for GEOSEARCH command
#[derive(Debug, Clone)]
pub enum GeoSearchCenter {
    /// Search from a specific position (FROMMEMBER)
    FromMember(String),
    /// Search from coordinates (FROMLONLAT)
    FromLonLat(GeoPosition),
}

/// Search shape for GEOSEARCH command
#[derive(Debug, Clone)]
pub enum GeoSearchShape {
    /// Circular search area (BYRADIUS)
    ByRadius { radius: f64, unit: GeoUnit },
    /// Rectangular search area (BYBOX)
    ByBox {
        width: f64,
        height: f64,
        unit: GeoUnit,
    },
}

/// Sort order for geo search results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeoSortOrder {
    /// Ascending (nearest first)
    #[default]
    Asc,
    /// Descending (farthest first)
    Desc,
}

/// Options for GEOSEARCH command
#[derive(Debug, Clone, Default)]
pub struct GeoSearchOptions {
    /// Include coordinates in results
    pub with_coord: bool,
    /// Include distance in results
    pub with_dist: bool,
    /// Include geohash in results
    pub with_hash: bool,
    /// Maximum number of results
    pub count: Option<usize>,
    /// Whether ANY flag is set (return any N results, not necessarily nearest)
    pub count_any: bool,
    /// Sort order
    pub sort: Option<GeoSortOrder>,
}

/// A search result item from GEOSEARCH
#[derive(Debug, Clone)]
pub struct GeoSearchResult {
    /// The member name
    pub member: String,
    /// Distance from search center (if requested)
    pub distance: Option<f64>,
    /// Position (if requested)
    pub position: Option<GeoPosition>,
    /// Geohash as integer (if requested)
    pub geohash: Option<i64>,
}

/// Result of GEOADD operation
#[derive(Debug, Clone)]
pub struct GeoAddResult {
    /// Number of elements added
    pub added: i64,
    /// Number of elements changed (if CH option was used)
    pub changed: Option<i64>,
}

/// Options for GEOADD command
#[derive(Debug, Clone, Default)]
pub struct GeoAddOptions {
    /// Only update existing elements (XX)
    pub xx: bool,
    /// Only add new elements (NX)
    pub nx: bool,
    /// Return changed count instead of added count (CH)
    pub ch: bool,
}

/// Result of GEOSEARCHSTORE operation
#[derive(Debug, Clone)]
pub struct GeoSearchStoreResult {
    /// Number of elements stored
    pub stored: i64,
}

/// Repository trait for Redis geospatial operations
#[async_trait]
pub trait GeoRepository: Send + Sync {
    // ========== Basic operations ==========

    /// GEOADD - Add one or more geospatial items
    /// Returns the number of elements added (or changed if CH option is set)
    async fn geo_add(
        &self,
        key: &str,
        members: &[GeoMember],
        options: GeoAddOptions,
    ) -> Result<GeoAddResult, CacheError>;

    /// GEOPOS - Get positions of members
    /// Returns positions in same order as input, None for non-existent members
    async fn geo_pos(
        &self,
        key: &str,
        members: &[String],
    ) -> Result<Vec<Option<GeoPosition>>, CacheError>;

    /// GEODIST - Get distance between two members
    /// Returns distance in specified unit, None if either member doesn't exist
    async fn geo_dist(
        &self,
        key: &str,
        member1: &str,
        member2: &str,
        unit: GeoUnit,
    ) -> Result<Option<f64>, CacheError>;

    /// GEOHASH - Get geohash strings for members
    /// Returns geohash strings in same order as input, None for non-existent members
    async fn geo_hash(
        &self,
        key: &str,
        members: &[String],
    ) -> Result<Vec<Option<String>>, CacheError>;

    // ========== Search operations ==========

    /// GEOSEARCH - Search for members within a given area
    async fn geo_search(
        &self,
        key: &str,
        center: GeoSearchCenter,
        shape: GeoSearchShape,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError>;

    /// GEOSEARCHSTORE - Store results of GEOSEARCH in a new key
    async fn geo_search_store(
        &self,
        dest_key: &str,
        source_key: &str,
        center: GeoSearchCenter,
        shape: GeoSearchShape,
        options: GeoSearchOptions,
        store_dist: bool,
    ) -> Result<GeoSearchStoreResult, CacheError>;

    // ========== Legacy operations (deprecated but still supported) ==========

    /// GEORADIUS - Search by radius from coordinates (deprecated, use GEOSEARCH)
    async fn geo_radius(
        &self,
        key: &str,
        position: GeoPosition,
        radius: f64,
        unit: GeoUnit,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError>;

    /// GEORADIUSBYMEMBER - Search by radius from a member (deprecated, use GEOSEARCH)
    async fn geo_radius_by_member(
        &self,
        key: &str,
        member: &str,
        radius: f64,
        unit: GeoUnit,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_position_valid() {
        let pos = GeoPosition::new(0.0, 0.0);
        assert!(pos.is_valid());

        let pos = GeoPosition::new(-180.0, -85.0);
        assert!(pos.is_valid());

        let pos = GeoPosition::new(180.0, 85.0);
        assert!(pos.is_valid());
    }

    #[test]
    fn test_geo_position_invalid() {
        let pos = GeoPosition::new(-181.0, 0.0);
        assert!(!pos.is_valid());

        let pos = GeoPosition::new(0.0, 90.0);
        assert!(!pos.is_valid());
    }

    #[test]
    fn test_geo_unit_as_str() {
        assert_eq!(GeoUnit::Meters.as_str(), "m");
        assert_eq!(GeoUnit::Kilometers.as_str(), "km");
        assert_eq!(GeoUnit::Miles.as_str(), "mi");
        assert_eq!(GeoUnit::Feet.as_str(), "ft");
    }

    #[test]
    fn test_geo_member() {
        let member = GeoMember::new("location1", 13.361389, 52.519444);
        assert_eq!(member.member, "location1");
        assert_eq!(member.position.longitude, 13.361389);
        assert_eq!(member.position.latitude, 52.519444);
    }

    #[test]
    fn test_geo_add_options_default() {
        let options = GeoAddOptions::default();
        assert!(!options.xx);
        assert!(!options.nx);
        assert!(!options.ch);
    }

    #[test]
    fn test_geo_search_options_default() {
        let options = GeoSearchOptions::default();
        assert!(!options.with_coord);
        assert!(!options.with_dist);
        assert!(!options.with_hash);
        assert!(options.count.is_none());
        assert!(!options.count_any);
        assert!(options.sort.is_none());
    }
}
