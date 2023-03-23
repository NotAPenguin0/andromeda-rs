use std::ops::RangeInclusive;
use std::str::FromStr;

pub fn format_km(value: f64, _digits: RangeInclusive<usize>) -> String {
    format!("{:.0} km", value)
}

pub fn parse_km(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    if trimmed.ends_with("km") {
        f64::from_str(trimmed.get(0..trimmed.len() - 3).unwrap()).ok()
    } else {
        f64::from_str(trimmed).ok()
    }
}

pub fn format_meters(value: f64, _digits: RangeInclusive<usize>) -> String {
    format!("{:.0} m", value)
}

pub fn parse_meters(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    if trimmed.ends_with("m") {
        f64::from_str(trimmed.get(0..trimmed.len() - 2).unwrap()).ok()
    } else {
        f64::from_str(trimmed).ok()
    }
}
