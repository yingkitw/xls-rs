//! Tests for geospatial operations

use xls_rs::{Coordinate, GeospatialCalculator};

#[test]
fn test_distance_same_point() {
    let calc = GeospatialCalculator::new();
    let point = Coordinate {
        latitude: 40.7128,
        longitude: -74.0060,
    };

    let distance = calc.distance(&point, &point);
    assert_eq!(distance, 0.0);
}

#[test]
fn test_distance_nyc_to_la() {
    let calc = GeospatialCalculator::new();
    let nyc = Coordinate {
        latitude: 40.7128,
        longitude: -74.0060,
    };
    let la = Coordinate {
        latitude: 34.0522,
        longitude: -118.2437,
    };

    let distance = calc.distance(&nyc, &la);
    // NYC to LA is approximately 3935 km
    assert!(distance > 3900.0 && distance < 4000.0);
}

#[test]
fn test_distance_london_to_paris() {
    let calc = GeospatialCalculator::new();
    let london = Coordinate {
        latitude: 51.5074,
        longitude: -0.1278,
    };
    let paris = Coordinate {
        latitude: 48.8566,
        longitude: 2.3522,
    };

    let distance = calc.distance(&london, &paris);
    // London to Paris is approximately 344 km
    assert!(distance > 300.0 && distance < 400.0);
}

#[test]
fn test_distance_tokyo_to_sydney() {
    let calc = GeospatialCalculator::new();
    let tokyo = Coordinate {
        latitude: 35.6762,
        longitude: 139.6503,
    };
    let sydney = Coordinate {
        latitude: -33.8688,
        longitude: 151.2093,
    };

    let distance = calc.distance(&tokyo, &sydney);
    // Tokyo to Sydney is approximately 7825 km
    assert!(distance > 7500.0 && distance < 8000.0);
}

#[test]
fn test_bearing_north() {
    let calc = GeospatialCalculator::new();
    let from = Coordinate {
        latitude: 0.0,
        longitude: 0.0,
    };
    let to = Coordinate {
        latitude: 10.0,
        longitude: 0.0,
    };

    let bearing = calc.bearing(&from, &to);
    // Due north should be approximately 0 degrees
    assert!(bearing < 10.0 || bearing > 350.0);
}

#[test]
fn test_bearing_east() {
    let calc = GeospatialCalculator::new();
    let from = Coordinate {
        latitude: 0.0,
        longitude: 0.0,
    };
    let to = Coordinate {
        latitude: 0.0,
        longitude: 10.0,
    };

    let bearing = calc.bearing(&from, &to);
    // Due east should be approximately 90 degrees
    assert!(bearing > 80.0 && bearing < 100.0);
}

#[test]
fn test_bearing_south() {
    let calc = GeospatialCalculator::new();
    let from = Coordinate {
        latitude: 10.0,
        longitude: 0.0,
    };
    let to = Coordinate {
        latitude: 0.0,
        longitude: 0.0,
    };

    let bearing = calc.bearing(&from, &to);
    // Due south should be approximately 180 degrees
    assert!(bearing > 170.0 && bearing < 190.0);
}

#[test]
fn test_bearing_west() {
    let calc = GeospatialCalculator::new();
    let from = Coordinate {
        latitude: 0.0,
        longitude: 10.0,
    };
    let to = Coordinate {
        latitude: 0.0,
        longitude: 0.0,
    };

    let bearing = calc.bearing(&from, &to);
    // Due west should be approximately 270 degrees
    assert!(bearing > 260.0 && bearing < 280.0);
}

#[test]
fn test_parse_coordinate_valid() {
    let calc = GeospatialCalculator::new();

    let coord = calc.parse_coordinate("40.7128,-74.0060").unwrap();
    assert_eq!(coord.latitude, 40.7128);
    assert_eq!(coord.longitude, -74.0060);

    let coord_with_space = calc.parse_coordinate("40.7128, -74.0060").unwrap();
    assert_eq!(coord_with_space.latitude, 40.7128);
    assert_eq!(coord_with_space.longitude, -74.0060);
}

#[test]
fn test_parse_coordinate_invalid_format() {
    let calc = GeospatialCalculator::new();

    let result = calc.parse_coordinate("40.7128");
    assert!(result.is_err());

    let result = calc.parse_coordinate("40.7128,-74.0060,extra");
    assert!(result.is_err());
}

#[test]
fn test_parse_coordinate_invalid_latitude() {
    let calc = GeospatialCalculator::new();

    let result = calc.parse_coordinate("invalid,-74.0060");
    assert!(result.is_err());
}

#[test]
fn test_parse_coordinate_out_of_range() {
    let calc = GeospatialCalculator::new();

    // Latitude out of range
    let result = calc.parse_coordinate("91.0,0.0");
    assert!(result.is_err());

    let result = calc.parse_coordinate("-91.0,0.0");
    assert!(result.is_err());

    // Longitude out of range
    let result = calc.parse_coordinate("0.0,181.0");
    assert!(result.is_err());

    let result = calc.parse_coordinate("0.0,-181.0");
    assert!(result.is_err());
}

#[test]
fn test_distance_from_strings() {
    let calc = GeospatialCalculator::new();

    let distance = calc
        .distance_from_strings("40.7128,-74.0060", "34.0522,-118.2437")
        .unwrap();
    assert!(distance > 3900.0 && distance < 4000.0);
}

#[test]
fn test_distance_from_strings_invalid() {
    let calc = GeospatialCalculator::new();

    let result = calc.distance_from_strings("invalid", "34.0522,-118.2437");
    assert!(result.is_err());
}

#[test]
fn test_equator_circumference() {
    let calc = GeospatialCalculator::new();

    // Points on equator, 1 degree apart = approximately 111 km
    let point1 = Coordinate {
        latitude: 0.0,
        longitude: 0.0,
    };
    let point2 = Coordinate {
        latitude: 0.0,
        longitude: 1.0,
    };

    let distance = calc.distance(&point1, &point2);
    assert!(distance > 110.0 && distance < 112.0);
}

#[test]
fn test_antipodal_points() {
    let calc = GeospatialCalculator::new();

    // Antipodal points (opposite sides of Earth)
    let point1 = Coordinate {
        latitude: 40.7128,
        longitude: -74.0060,
    };
    let point2 = Coordinate {
        latitude: -40.7128,
        longitude: 105.994,
    };

    let distance = calc.distance(&point1, &point2);
    // Should be approximately half Earth's circumference (~20015 km)
    assert!(distance > 19000.0 && distance < 21000.0);
}
