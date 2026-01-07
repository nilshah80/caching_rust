//! Geo Schemas
//!
//! Request and response types for geospatial API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::repositories::{
    GeoAddOptions, GeoMember, GeoPosition, GeoSearchCenter, GeoSearchOptions, GeoSearchResult,
    GeoSearchShape, GeoSortOrder, GeoUnit,
};

// ========== Common Types ==========

/// Distance unit for geospatial operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum GeoUnitSchema {
    /// Meters (default)
    #[default]
    #[serde(rename = "m")]
    Meters,
    /// Kilometers
    #[serde(rename = "km")]
    Kilometers,
    /// Miles
    #[serde(rename = "mi")]
    Miles,
    /// Feet
    #[serde(rename = "ft")]
    Feet,
}

impl From<GeoUnitSchema> for GeoUnit {
    fn from(unit: GeoUnitSchema) -> Self {
        match unit {
            GeoUnitSchema::Meters => GeoUnit::Meters,
            GeoUnitSchema::Kilometers => GeoUnit::Kilometers,
            GeoUnitSchema::Miles => GeoUnit::Miles,
            GeoUnitSchema::Feet => GeoUnit::Feet,
        }
    }
}

impl From<GeoUnit> for GeoUnitSchema {
    fn from(unit: GeoUnit) -> Self {
        match unit {
            GeoUnit::Meters => GeoUnitSchema::Meters,
            GeoUnit::Kilometers => GeoUnitSchema::Kilometers,
            GeoUnit::Miles => GeoUnitSchema::Miles,
            GeoUnit::Feet => GeoUnitSchema::Feet,
        }
    }
}

/// Sort order for search results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum GeoSortOrderSchema {
    /// Sort by distance ascending (nearest first)
    Asc,
    /// Sort by distance descending (farthest first)
    Desc,
}

impl From<GeoSortOrderSchema> for GeoSortOrder {
    fn from(order: GeoSortOrderSchema) -> Self {
        match order {
            GeoSortOrderSchema::Asc => GeoSortOrder::Asc,
            GeoSortOrderSchema::Desc => GeoSortOrder::Desc,
        }
    }
}

/// Geographic position (longitude, latitude)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoPositionSchema {
    /// Longitude (-180 to 180)
    pub longitude: f64,
    /// Latitude (-85.05112878 to 85.05112878)
    pub latitude: f64,
}

impl From<GeoPositionSchema> for GeoPosition {
    fn from(pos: GeoPositionSchema) -> Self {
        GeoPosition::new(pos.longitude, pos.latitude)
    }
}

impl From<GeoPosition> for GeoPositionSchema {
    fn from(pos: GeoPosition) -> Self {
        Self {
            longitude: pos.longitude,
            latitude: pos.latitude,
        }
    }
}

/// A member with geographic position
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoMemberSchema {
    /// Member name
    pub member: String,
    /// Longitude
    pub longitude: f64,
    /// Latitude
    pub latitude: f64,
}

impl From<GeoMemberSchema> for GeoMember {
    fn from(m: GeoMemberSchema) -> Self {
        GeoMember::new(&m.member, m.longitude, m.latitude)
    }
}

// ========== Request Types ==========

/// Request to add geospatial items
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct GeoAddRequest {
    /// Members to add with their coordinates
    #[validate(length(min = 1, message = "At least one member is required"))]
    pub members: Vec<GeoMemberSchema>,
    /// Only add new elements, don't update existing ones
    #[serde(default)]
    pub nx: bool,
    /// Only update existing elements, don't add new ones
    #[serde(default)]
    pub xx: bool,
    /// Return number of elements changed (added or updated)
    #[serde(default)]
    pub ch: bool,
}

impl GeoAddRequest {
    pub fn to_options(&self) -> GeoAddOptions {
        GeoAddOptions {
            nx: self.nx,
            xx: self.xx,
            ch: self.ch,
        }
    }
}

/// Request to get positions of members
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct GeoPosRequest {
    /// Member names to get positions for
    #[validate(length(min = 1, message = "At least one member is required"))]
    pub members: Vec<String>,
}

/// Query parameters for GEODIST
#[derive(Debug, Serialize, Deserialize, ToSchema, Default)]
pub struct GeoDistQuery {
    /// Distance unit (default: meters)
    #[serde(default)]
    pub unit: GeoUnitSchema,
}

/// Request to get geohashes
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct GeoHashRequest {
    /// Member names to get geohashes for
    #[validate(length(min = 1, message = "At least one member is required"))]
    pub members: Vec<String>,
}

/// Search center specification
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum GeoSearchCenterSchema {
    /// Search from a member's position
    #[serde(rename = "FROMMEMBER")]
    FromMember {
        /// Member name to search from
        member: String,
    },
    /// Search from longitude/latitude coordinates
    #[serde(rename = "FROMLONLAT")]
    FromLonLat {
        /// Longitude
        longitude: f64,
        /// Latitude
        latitude: f64,
    },
}

impl From<GeoSearchCenterSchema> for GeoSearchCenter {
    fn from(center: GeoSearchCenterSchema) -> Self {
        match center {
            GeoSearchCenterSchema::FromMember { member } => GeoSearchCenter::FromMember(member),
            GeoSearchCenterSchema::FromLonLat {
                longitude,
                latitude,
            } => GeoSearchCenter::FromLonLat(GeoPosition::new(longitude, latitude)),
        }
    }
}

/// Search shape specification
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum GeoSearchShapeSchema {
    /// Search by radius
    #[serde(rename = "BYRADIUS")]
    ByRadius {
        /// Search radius
        radius: f64,
        /// Distance unit
        #[serde(default)]
        unit: GeoUnitSchema,
    },
    /// Search by bounding box
    #[serde(rename = "BYBOX")]
    ByBox {
        /// Box width
        width: f64,
        /// Box height
        height: f64,
        /// Distance unit
        #[serde(default)]
        unit: GeoUnitSchema,
    },
}

impl From<GeoSearchShapeSchema> for GeoSearchShape {
    fn from(shape: GeoSearchShapeSchema) -> Self {
        match shape {
            GeoSearchShapeSchema::ByRadius { radius, unit } => GeoSearchShape::ByRadius {
                radius,
                unit: unit.into(),
            },
            GeoSearchShapeSchema::ByBox {
                width,
                height,
                unit,
            } => GeoSearchShape::ByBox {
                width,
                height,
                unit: unit.into(),
            },
        }
    }
}

/// Options for geo search
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct GeoSearchOptionsSchema {
    /// Include distance in results
    #[serde(default)]
    pub with_dist: bool,
    /// Include coordinates in results
    #[serde(default)]
    pub with_coord: bool,
    /// Include geohash in results
    #[serde(default)]
    pub with_hash: bool,
    /// Sort order (ASC = nearest first, DESC = farthest first)
    pub sort: Option<GeoSortOrderSchema>,
    /// Maximum number of results
    pub count: Option<usize>,
    /// If true with count, may return less accurate results for better performance
    #[serde(default)]
    pub count_any: bool,
}

impl From<GeoSearchOptionsSchema> for GeoSearchOptions {
    fn from(opts: GeoSearchOptionsSchema) -> Self {
        GeoSearchOptions {
            with_dist: opts.with_dist,
            with_coord: opts.with_coord,
            with_hash: opts.with_hash,
            sort: opts.sort.map(|s| s.into()),
            count: opts.count,
            count_any: opts.count_any,
        }
    }
}

/// Request for GEOSEARCH
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct GeoSearchRequest {
    /// Center point for the search
    pub center: GeoSearchCenterSchema,
    /// Search shape (radius or box)
    pub shape: GeoSearchShapeSchema,
    /// Search options
    #[serde(default)]
    pub options: GeoSearchOptionsSchema,
}

/// Request for GEOSEARCHSTORE
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct GeoSearchStoreRequest {
    /// Source key containing geo data
    #[validate(length(min = 1, message = "Source key is required"))]
    pub source_key: String,
    /// Center point for the search
    pub center: GeoSearchCenterSchema,
    /// Search shape (radius or box)
    pub shape: GeoSearchShapeSchema,
    /// Search options
    #[serde(default)]
    pub options: GeoSearchOptionsSchema,
    /// Store distances instead of geohashes
    #[serde(default)]
    pub store_dist: bool,
}

/// Query parameters for GEORADIUS (legacy)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GeoRadiusQuery {
    /// Center longitude
    pub longitude: f64,
    /// Center latitude
    pub latitude: f64,
    /// Search radius
    pub radius: f64,
    /// Distance unit
    #[serde(default)]
    pub unit: GeoUnitSchema,
    /// Include distance in results
    #[serde(default)]
    pub with_dist: bool,
    /// Include coordinates in results
    #[serde(default)]
    pub with_coord: bool,
    /// Include geohash in results
    #[serde(default)]
    pub with_hash: bool,
    /// Sort order
    pub sort: Option<GeoSortOrderSchema>,
    /// Maximum number of results
    pub count: Option<usize>,
}

impl GeoRadiusQuery {
    pub fn to_options(&self) -> GeoSearchOptions {
        GeoSearchOptions {
            with_dist: self.with_dist,
            with_coord: self.with_coord,
            with_hash: self.with_hash,
            sort: self.sort.clone().map(|s| s.into()),
            count: self.count,
            count_any: false,
        }
    }
}

/// Query parameters for GEORADIUSBYMEMBER (legacy)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GeoRadiusByMemberQuery {
    /// Search radius
    pub radius: f64,
    /// Distance unit
    #[serde(default)]
    pub unit: GeoUnitSchema,
    /// Include distance in results
    #[serde(default)]
    pub with_dist: bool,
    /// Include coordinates in results
    #[serde(default)]
    pub with_coord: bool,
    /// Include geohash in results
    #[serde(default)]
    pub with_hash: bool,
    /// Sort order
    pub sort: Option<GeoSortOrderSchema>,
    /// Maximum number of results
    pub count: Option<usize>,
}

impl GeoRadiusByMemberQuery {
    pub fn to_options(&self) -> GeoSearchOptions {
        GeoSearchOptions {
            with_dist: self.with_dist,
            with_coord: self.with_coord,
            with_hash: self.with_hash,
            sort: self.sort.clone().map(|s| s.into()),
            count: self.count,
            count_any: false,
        }
    }
}

// ========== Response Types ==========

/// Response from GEOADD
#[derive(Debug, Serialize, ToSchema)]
pub struct GeoAddResponse {
    /// Number of new elements added
    pub added: i64,
    /// Number of elements changed (if CH option was used)
    pub changed: Option<i64>,
}

/// Response from GEOPOS
#[derive(Debug, Serialize, ToSchema)]
pub struct GeoPosResponse {
    /// Positions for each requested member (None if member doesn't exist)
    pub positions: Vec<Option<GeoPositionSchema>>,
}

/// Response from GEODIST
#[derive(Debug, Serialize, ToSchema)]
pub struct GeoDistResponse {
    /// Distance between the two members (None if either doesn't exist)
    pub distance: Option<f64>,
    /// Unit of the distance
    pub unit: GeoUnitSchema,
}

/// Response from GEOHASH
#[derive(Debug, Serialize, ToSchema)]
pub struct GeoHashResponse {
    /// Geohashes for each requested member (None if member doesn't exist)
    pub hashes: Vec<Option<String>>,
}

/// A single search result item
#[derive(Debug, Serialize, ToSchema)]
pub struct GeoSearchResultItem {
    /// Member name
    pub member: String,
    /// Distance from center (if requested)
    pub distance: Option<f64>,
    /// Position coordinates (if requested)
    pub position: Option<GeoPositionSchema>,
    /// Raw geohash integer (if requested)
    pub geohash: Option<i64>,
}

impl From<GeoSearchResult> for GeoSearchResultItem {
    fn from(r: GeoSearchResult) -> Self {
        Self {
            member: r.member,
            distance: r.distance,
            position: r.position.map(|p| p.into()),
            geohash: r.geohash,
        }
    }
}

/// Response from GEOSEARCH
#[derive(Debug, Serialize, ToSchema)]
pub struct GeoSearchResponse {
    /// Search results
    pub results: Vec<GeoSearchResultItem>,
}

/// Response from GEOSEARCHSTORE
#[derive(Debug, Serialize, ToSchema)]
pub struct GeoSearchStoreResponse {
    /// Number of elements stored
    pub stored: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_unit_schema() {
        let m: GeoUnitSchema = serde_json::from_str(r#""m""#).unwrap();
        assert!(matches!(m, GeoUnitSchema::Meters));

        let km: GeoUnitSchema = serde_json::from_str(r#""km""#).unwrap();
        assert!(matches!(km, GeoUnitSchema::Kilometers));

        let mi: GeoUnitSchema = serde_json::from_str(r#""mi""#).unwrap();
        assert!(matches!(mi, GeoUnitSchema::Miles));

        let ft: GeoUnitSchema = serde_json::from_str(r#""ft""#).unwrap();
        assert!(matches!(ft, GeoUnitSchema::Feet));
    }

    #[test]
    fn test_geo_position_schema() {
        let pos: GeoPositionSchema =
            serde_json::from_str(r#"{"longitude": 13.361389, "latitude": 52.519444}"#).unwrap();
        assert_eq!(pos.longitude, 13.361389);
        assert_eq!(pos.latitude, 52.519444);
    }

    #[test]
    fn test_geo_member_schema() {
        let member: GeoMemberSchema = serde_json::from_str(
            r#"{"member": "Berlin", "longitude": 13.361389, "latitude": 52.519444}"#,
        )
        .unwrap();
        assert_eq!(member.member, "Berlin");
        assert_eq!(member.longitude, 13.361389);
        assert_eq!(member.latitude, 52.519444);
    }

    #[test]
    fn test_geo_add_request() {
        let req: GeoAddRequest = serde_json::from_str(
            r#"{
                "members": [
                    {"member": "Berlin", "longitude": 13.361389, "latitude": 52.519444},
                    {"member": "Paris", "longitude": 2.349014, "latitude": 48.864716}
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(req.members.len(), 2);
        assert!(!req.nx);
        assert!(!req.xx);
        assert!(!req.ch);
    }

    #[test]
    fn test_geo_add_request_with_options() {
        let req: GeoAddRequest = serde_json::from_str(
            r#"{
                "members": [{"member": "Berlin", "longitude": 13.361389, "latitude": 52.519444}],
                "nx": true,
                "ch": true
            }"#,
        )
        .unwrap();
        assert!(req.nx);
        assert!(!req.xx);
        assert!(req.ch);
    }

    #[test]
    fn test_geo_search_center_from_member() {
        let center: GeoSearchCenterSchema =
            serde_json::from_str(r#"{"type": "FROMMEMBER", "member": "Berlin"}"#).unwrap();
        assert!(matches!(
            center,
            GeoSearchCenterSchema::FromMember { member } if member == "Berlin"
        ));
    }

    #[test]
    fn test_geo_search_center_from_lonlat() {
        let center: GeoSearchCenterSchema = serde_json::from_str(
            r#"{"type": "FROMLONLAT", "longitude": 13.361389, "latitude": 52.519444}"#,
        )
        .unwrap();
        assert!(matches!(
            center,
            GeoSearchCenterSchema::FromLonLat { longitude, latitude }
                if (longitude - 13.361389).abs() < 0.0001 && (latitude - 52.519444).abs() < 0.0001
        ));
    }

    #[test]
    fn test_geo_search_shape_by_radius() {
        let shape: GeoSearchShapeSchema =
            serde_json::from_str(r#"{"type": "BYRADIUS", "radius": 100.0, "unit": "km"}"#).unwrap();
        assert!(matches!(
            shape,
            GeoSearchShapeSchema::ByRadius { radius, .. } if (radius - 100.0).abs() < 0.0001
        ));
    }

    #[test]
    fn test_geo_search_shape_by_box() {
        let shape: GeoSearchShapeSchema = serde_json::from_str(
            r#"{"type": "BYBOX", "width": 200.0, "height": 100.0, "unit": "mi"}"#,
        )
        .unwrap();
        assert!(matches!(
            shape,
            GeoSearchShapeSchema::ByBox { width, height, .. }
                if (width - 200.0).abs() < 0.0001 && (height - 100.0).abs() < 0.0001
        ));
    }

    #[test]
    fn test_geo_search_request() {
        let req: GeoSearchRequest = serde_json::from_str(
            r#"{
                "center": {"type": "FROMMEMBER", "member": "Berlin"},
                "shape": {"type": "BYRADIUS", "radius": 500.0, "unit": "km"},
                "options": {
                    "with_dist": true,
                    "with_coord": true,
                    "sort": "ASC",
                    "count": 10
                }
            }"#,
        )
        .unwrap();
        assert!(matches!(
            req.center,
            GeoSearchCenterSchema::FromMember { .. }
        ));
        assert!(matches!(req.shape, GeoSearchShapeSchema::ByRadius { .. }));
        assert!(req.options.with_dist);
        assert!(req.options.with_coord);
        assert!(matches!(req.options.sort, Some(GeoSortOrderSchema::Asc)));
        assert_eq!(req.options.count, Some(10));
    }

    #[test]
    fn test_geo_search_store_request() {
        let req: GeoSearchStoreRequest = serde_json::from_str(
            r#"{
                "source_key": "locations",
                "center": {"type": "FROMLONLAT", "longitude": 13.361389, "latitude": 52.519444},
                "shape": {"type": "BYRADIUS", "radius": 100.0, "unit": "km"},
                "store_dist": true
            }"#,
        )
        .unwrap();
        assert_eq!(req.source_key, "locations");
        assert!(req.store_dist);
    }

    #[test]
    fn test_geo_unit_conversion() {
        let domain_unit: GeoUnit = GeoUnitSchema::Meters.into();
        assert!(matches!(domain_unit, GeoUnit::Meters));

        let domain_unit: GeoUnit = GeoUnitSchema::Kilometers.into();
        assert!(matches!(domain_unit, GeoUnit::Kilometers));

        let domain_unit: GeoUnit = GeoUnitSchema::Miles.into();
        assert!(matches!(domain_unit, GeoUnit::Miles));

        let domain_unit: GeoUnit = GeoUnitSchema::Feet.into();
        assert!(matches!(domain_unit, GeoUnit::Feet));
    }

    #[test]
    fn test_geo_sort_order_conversion() {
        let domain_order: GeoSortOrder = GeoSortOrderSchema::Asc.into();
        assert!(matches!(domain_order, GeoSortOrder::Asc));

        let domain_order: GeoSortOrder = GeoSortOrderSchema::Desc.into();
        assert!(matches!(domain_order, GeoSortOrder::Desc));
    }
}
