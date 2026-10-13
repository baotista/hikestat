use std::sync::LazyLock;

use tzf_rs::DefaultFinder;

/// Built once: loading the bundled boundaries takes a noticeable moment.
static FINDER: LazyLock<DefaultFinder> = LazyLock::new(DefaultFinder::new);

/// IANA name of the time zone at a coordinate, derived offline from bundled boundaries.
///
/// The boundaries cover the oceans too, so a point at sea gets its nautical zone (`Etc/GMT±N`).
/// `None` only for a coordinate no zone covers, i.e. one outside the valid range.
pub fn zone_at(lat: f64, lon: f64) -> Option<&'static str> {
    Some(FINDER.get_tz_name(lon, lat)).filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paris_is_in_europe_paris() {
        assert_eq!(zone_at(48.8566, 2.3522), Some("Europe/Paris"));
    }

    #[test]
    fn tokyo_is_in_asia_tokyo() {
        assert_eq!(zone_at(35.6762, 139.6503), Some("Asia/Tokyo"));
    }

    #[test]
    fn each_side_of_the_rhine_has_its_own_zone() {
        // Strasbourg and Kehl, about 5 km apart across the French-German border.
        assert_eq!(zone_at(48.5734, 7.7521), Some("Europe/Paris"));
        assert_eq!(zone_at(48.5726, 7.8156), Some("Europe/Berlin"));
    }

    #[test]
    fn the_open_sea_gets_its_nautical_zone() {
        // Mid-Atlantic, 30° W: UTC-2, which IANA spells Etc/GMT+2.
        assert_eq!(zone_at(45.0, -30.0), Some("Etc/GMT+2"));
    }

    #[test]
    fn impossible_coordinates_have_no_zone() {
        assert_eq!(zone_at(100.0, 0.0), None);
        assert_eq!(zone_at(0.0, 200.0), None);
        assert_eq!(zone_at(f64::NAN, 0.0), None);
    }
}
