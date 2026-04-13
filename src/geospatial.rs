//! Geospatial operations
//!
//! Provides geospatial calculations including distance, bearing, and area calculations.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Geographic coordinate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

/// Geospatial calculator
pub struct GeospatialCalculator;

impl GeospatialCalculator {
    pub fn new() -> Self {
        Self
    }

    /// Calculate distance between two coordinates using Haversine formula
    /// Returns distance in kilometers
    pub fn distance(&self, from: &Coordinate, to: &Coordinate) -> f64 {
        const EARTH_RADIUS_KM: f64 = 6371.0;

        let lat1_rad = from.latitude.to_radians();
        let lat2_rad = to.latitude.to_radians();
        let delta_lat = (to.latitude - from.latitude).to_radians();
        let delta_lon = (to.longitude - from.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_KM * c
    }

    /// Calculate bearing (direction) from one point to another
    /// Returns bearing in degrees (0-360)
    pub fn bearing(&self, from: &Coordinate, to: &Coordinate) -> f64 {
        let lat1_rad = from.latitude.to_radians();
        let lat2_rad = to.latitude.to_radians();
        let delta_lon = (to.longitude - from.longitude).to_radians();

        let y = delta_lon.sin() * lat2_rad.cos();
        let x = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * delta_lon.cos();

        let bearing_rad = y.atan2(x);
        let bearing_deg = bearing_rad.to_degrees();

        (bearing_deg + 360.0) % 360.0
    }

    /// Parse coordinate from string (format: "lat,lon" or "lat, lon")
    pub fn parse_coordinate(&self, coord_str: &str) -> Result<Coordinate> {
        let parts: Vec<&str> = coord_str.split(',').map(|s| s.trim()).collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid coordinate format. Expected 'lat,lon', got: {}",
                coord_str
            );
        }

        let lat = parts[0]
            .parse::<f64>()
            .map_err(|e| anyhow::anyhow!("Invalid latitude: {}", e))?;
        let lon = parts[1]
            .parse::<f64>()
            .map_err(|e| anyhow::anyhow!("Invalid longitude: {}", e))?;

        if lat < -90.0 || lat > 90.0 {
            anyhow::bail!("Latitude must be between -90 and 90, got: {}", lat);
        }
        if lon < -180.0 || lon > 180.0 {
            anyhow::bail!("Longitude must be between -180 and 180, got: {}", lon);
        }

        Ok(Coordinate {
            latitude: lat,
            longitude: lon,
        })
    }

    /// Calculate distance between two coordinate strings
    pub fn distance_from_strings(&self, from_str: &str, to_str: &str) -> Result<f64> {
        let from = self.parse_coordinate(from_str)?;
        let to = self.parse_coordinate(to_str)?;
        Ok(self.distance(&from, &to))
    }
}
