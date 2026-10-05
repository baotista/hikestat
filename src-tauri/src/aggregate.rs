//! Hikes aggregated over the window of a dashboard view: pure, no I/O, no clock.
//!
//! A hike belongs to its **local date**: its start instant expressed in its start time zone (UTC
//! when the zone is missing or unknown). A hike crossing midnight belongs to its start day. Weeks
//! are ISO weeks, Monday first.

use hikestat::storage::Hike;
use jiff::civil::Date;
use jiff::tz::{self, TimeZone};
use jiff::{Timestamp, ToSpan};
use serde::{Deserialize, Serialize};
use specta::Type;

/// How the dashboard cuts time. Only the week exists so far; month, year and all come next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum View {
    /// An ISO week, one bucket per day, Monday to Sunday.
    Week,
}

/// Which hikes are counted, by kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "filter", content = "kind", rename_all = "camelCase")]
pub enum KindFilter {
    All,
    /// Hikes without a kind.
    NotSet,
    /// Hikes of exactly this kind (case-sensitive, as stored).
    Kind(String),
}

/// Summed metrics. A missing (NULL) metric counts as 0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub distance_m: f64,
    pub ascent_m: f64,
    #[specta(type = specta_typescript::Number)]
    pub moving_s: i64,
    pub hike_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    /// First local date of the bucket (the day itself in the week view).
    pub start: Date,
    pub metrics: Metrics,
    /// The bucket's hikes, in the order they were given.
    #[specta(type = Vec<specta_typescript::Number>)]
    pub hike_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub view: View,
    /// First and last local dates of the window, both included.
    pub first: Date,
    pub last: Date,
    /// In chronological order, empty ones included.
    pub buckets: Vec<Bucket>,
    pub totals: Metrics,
    /// Hikes of the window whose ascent is missing.
    pub without_elevation: u32,
    /// Anchors of the previous and next windows (their first date).
    pub previous: Date,
    pub next: Date,
}

/// Aggregates `hikes` over the `view` window containing `anchor`, keeping those matching `filter`.
/// Hikes without a start instant cannot be placed and are left out. Fails only when the window
/// falls outside the dates jiff can represent.
pub fn aggregate(
    hikes: &[Hike],
    view: View,
    anchor: Date,
    filter: &KindFilter,
) -> Result<Dashboard, jiff::Error> {
    let View::Week = view;
    let first = anchor.checked_sub(i64::from(anchor.weekday().to_monday_zero_offset()).days())?;
    let last = first.checked_add(6.days())?;
    let mut buckets = (0..7)
        .map(|i| {
            Ok(Bucket {
                start: first.checked_add(i.days())?,
                metrics: Metrics::default(),
                hike_ids: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>, jiff::Error>>()?;
    let mut totals = Metrics::default();
    let mut without_elevation = 0;

    for h in hikes.iter().filter(|h| matches(filter, h)) {
        let Some(day) = local_date(h) else { continue };
        if day < first || day > last {
            continue;
        }
        let bucket = &mut buckets[(day - first).get_days() as usize];
        add(&mut bucket.metrics, h);
        bucket.hike_ids.push(h.id);
        add(&mut totals, h);
        if h.ascent_m.is_none() {
            without_elevation += 1;
        }
    }

    Ok(Dashboard {
        view,
        first,
        last,
        buckets,
        totals,
        without_elevation,
        previous: first.checked_sub(7.days())?,
        next: first.checked_add(7.days())?,
    })
}

fn matches(filter: &KindFilter, hike: &Hike) -> bool {
    match filter {
        KindFilter::All => true,
        KindFilter::NotSet => hike.kind.is_none(),
        KindFilter::Kind(kind) => hike.kind.as_deref() == Some(kind.as_str()),
    }
}

/// The start instant in the start time zone, UTC if the zone is missing or unknown.
fn local_date(hike: &Hike) -> Option<Date> {
    let instant = Timestamp::from_second(hike.started_at?).ok()?;
    let zone = hike
        .start_tz
        .as_deref()
        .and_then(|name| tz::db().get(name).ok())
        .unwrap_or(TimeZone::UTC);
    Some(instant.to_zoned(zone).date())
}

fn add(metrics: &mut Metrics, hike: &Hike) {
    metrics.distance_m += hike.distance_m;
    metrics.ascent_m += hike.ascent_m.unwrap_or(0.0);
    metrics.moving_s += hike.duration_moving_s.unwrap_or(0);
    metrics.hike_count += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    /// A hike starting at `start` (RFC 3339) in `tz`, of `km` kilometres.
    fn hike(id: i64, start: &str, tz: Option<&str>, km: f64) -> Hike {
        Hike {
            id,
            name: None,
            kind: None,
            started_at: Some(start.parse::<Timestamp>().unwrap().as_second()),
            start_tz: tz.map(str::to_string),
            distance_m: km * 1000.0,
            ascent_m: Some(100.0),
            descent_m: Some(100.0),
            min_ele_m: Some(0.0),
            max_ele_m: Some(100.0),
            duration_total_s: Some(3600),
            duration_moving_s: Some(3000),
            avg_speed_mps: Some(1.0),
            stats_version: Some(2),
        }
    }

    fn kind(mut h: Hike, kind: Option<&str>) -> Hike {
        h.kind = kind.map(str::to_string);
        h
    }

    fn week(hikes: &[Hike], anchor: Date) -> Dashboard {
        aggregate(hikes, View::Week, anchor, &KindFilter::All).unwrap()
    }

    fn days(d: &Dashboard) -> Vec<Date> {
        d.buckets.iter().map(|b| b.start).collect()
    }

    /// The ids in each of the window's buckets.
    fn ids(d: &Dashboard) -> Vec<Vec<i64>> {
        d.buckets.iter().map(|b| b.hike_ids.clone()).collect()
    }

    fn seven_days_from(first: Date) -> Vec<Date> {
        (0..7)
            .map(|i| first.checked_add(i.days()).unwrap())
            .collect()
    }

    #[test]
    fn a_week_is_its_iso_week_monday_to_sunday() {
        let d = week(&[], date(2026, 10, 1)); // a Thursday
        assert_eq!((d.first, d.last), (date(2026, 9, 28), date(2026, 10, 4)));
        assert_eq!(days(&d), seven_days_from(date(2026, 9, 28)));
        assert_eq!(d.view, View::Week);
    }

    #[test]
    fn any_day_of_the_week_anchors_the_same_window() {
        let monday = week(&[], date(2026, 9, 28));
        let sunday = week(&[], date(2026, 10, 4));
        assert_eq!(monday, sunday);
    }

    #[test]
    fn an_empty_week_has_seven_zero_buckets() {
        let d = week(&[], date(2026, 10, 1));
        assert_eq!(d.buckets.len(), 7);
        for b in &d.buckets {
            assert_eq!(b.metrics, Metrics::default());
            assert!(b.hike_ids.is_empty());
        }
        assert_eq!(d.totals, Metrics::default());
        assert_eq!(d.without_elevation, 0);
    }

    #[test]
    fn hikes_outside_the_week_are_left_out() {
        let hikes = [
            hike(1, "2026-09-27T12:00:00Z", None, 5.0), // Sunday before
            hike(2, "2026-10-05T12:00:00Z", None, 5.0), // Monday after
        ];
        let d = week(&hikes, date(2026, 10, 1));
        assert_eq!(d.totals, Metrics::default());
    }

    #[test]
    fn buckets_sum_their_hikes_and_totals_sum_the_buckets() {
        let mut other = hike(3, "2026-10-03T08:00:00Z", Some("Europe/Paris"), 4.0);
        other.ascent_m = Some(250.5);
        other.duration_moving_s = Some(1000);
        let hikes = [
            hike(1, "2026-09-30T08:00:00Z", Some("Europe/Paris"), 10.0),
            hike(2, "2026-09-30T14:00:00Z", Some("Europe/Paris"), 2.5),
            other,
        ];
        let d = week(&hikes, date(2026, 10, 1));

        assert_eq!(
            ids(&d),
            vec![vec![], vec![], vec![1, 2], vec![], vec![], vec![3], vec![]]
        );
        assert_eq!(
            d.buckets[2].metrics,
            Metrics {
                distance_m: 12_500.0,
                ascent_m: 200.0,
                moving_s: 6000,
                hike_count: 2
            }
        );
        assert_eq!(
            d.totals,
            Metrics {
                distance_m: 16_500.0,
                ascent_m: 450.5,
                moving_s: 7000,
                hike_count: 3
            }
        );
        let summed: f64 = d.buckets.iter().map(|b| b.metrics.distance_m).sum();
        assert_eq!(summed, d.totals.distance_m);
    }

    #[test]
    fn a_japanese_morning_hike_lands_on_the_japanese_date() {
        // 06:00 in Tokyo on Thursday 1 October is 21:00 UTC (23:00 in Paris) on Wednesday.
        let hikes = [hike(1, "2026-09-30T21:00:00Z", Some("Asia/Tokyo"), 8.0)];
        let d = week(&hikes, date(2026, 10, 1));
        assert_eq!(d.buckets[3].start, date(2026, 10, 1));
        assert_eq!(d.buckets[3].hike_ids, vec![1]);
        assert_eq!(d.totals.hike_count, 1);
    }

    #[test]
    fn a_missing_start_time_zone_falls_back_to_utc() {
        // 23:30 UTC on Wednesday; it would be Thursday in Paris.
        let hikes = [hike(1, "2026-09-30T23:30:00Z", None, 3.0)];
        let d = week(&hikes, date(2026, 10, 1));
        assert_eq!(ids(&d)[2], vec![1]);
    }

    #[test]
    fn an_unknown_start_time_zone_falls_back_to_utc() {
        let hikes = [hike(
            1,
            "2026-09-30T23:30:00Z",
            Some("Mars/Olympus_Mons"),
            3.0,
        )];
        let d = week(&hikes, date(2026, 10, 1));
        assert_eq!(ids(&d)[2], vec![1]);
    }

    #[test]
    fn a_hike_crossing_midnight_belongs_to_its_start_day() {
        let mut h = hike(1, "2026-10-01T20:00:00Z", Some("Europe/Paris"), 20.0); // 22:00 local
        h.duration_total_s = Some(5 * 3600);
        let d = week(&[h], date(2026, 10, 1));
        assert_eq!(ids(&d)[3], vec![1]);
        assert_eq!(d.totals.hike_count, 1);
    }

    #[test]
    fn a_week_with_the_spring_dst_change_has_seven_days() {
        // Europe moves to summer time on Sunday 29 March 2026.
        let hikes = [
            hike(1, "2026-03-23T06:30:00Z", Some("Europe/Paris"), 1.0), // Mon 07:30 CET
            hike(2, "2026-03-28T23:30:00Z", Some("Europe/Paris"), 1.0), // Sun 00:30 CET
            hike(3, "2026-03-29T21:30:00Z", Some("Europe/Paris"), 1.0), // Sun 23:30 CEST
            hike(4, "2026-03-29T22:30:00Z", Some("Europe/Paris"), 1.0), // Mon 00:30 CEST
        ];
        let d = week(&hikes, date(2026, 3, 29));
        assert_eq!(days(&d), seven_days_from(date(2026, 3, 23)));
        assert_eq!(
            ids(&d),
            vec![vec![1], vec![], vec![], vec![], vec![], vec![], vec![2, 3]]
        );
    }

    #[test]
    fn a_week_with_the_autumn_dst_change_has_seven_days() {
        // Europe goes back to winter time on Sunday 25 October 2026.
        let hikes = [
            hike(1, "2026-10-24T22:30:00Z", Some("Europe/Paris"), 1.0), // Sun 00:30 CEST
            hike(2, "2026-10-25T22:30:00Z", Some("Europe/Paris"), 1.0), // Sun 23:30 CET
            hike(3, "2026-10-25T23:30:00Z", Some("Europe/Paris"), 1.0), // Mon 00:30 CET
        ];
        let d = week(&hikes, date(2026, 10, 21));
        assert_eq!(days(&d), seven_days_from(date(2026, 10, 19)));
        assert_eq!(ids(&d)[6], vec![1, 2]);
        assert_eq!(d.totals.hike_count, 2);
        assert_eq!(d.next, date(2026, 10, 26));
        assert_eq!(week(&hikes, d.next).buckets[0].hike_ids, vec![3]);
    }

    #[test]
    fn iso_week_53_spans_the_new_year() {
        // 2026-W53: Monday 28 December 2026 to Sunday 3 January 2027.
        let hikes = [hike(1, "2027-01-02T10:00:00Z", Some("Europe/Paris"), 7.0)];
        let d = week(&hikes, date(2027, 1, 2));
        assert_eq!((d.first, d.last), (date(2026, 12, 28), date(2027, 1, 3)));
        assert_eq!(d.first.iso_week_date().week(), 53);
        assert_eq!(ids(&d)[5], vec![1]);
        assert_eq!((d.previous, d.next), (date(2026, 12, 21), date(2027, 1, 4)));
    }

    #[test]
    fn week_1_can_start_in_december() {
        // 2025-W01: Monday 30 December 2024 to Sunday 5 January 2025.
        let d = week(&[], date(2025, 1, 1));
        assert_eq!((d.first, d.last), (date(2024, 12, 30), date(2025, 1, 5)));
        assert_eq!(d.first.iso_week_date().week(), 1);
        assert_eq!(d.first.iso_week_date().year(), 2025);
    }

    #[test]
    fn previous_and_next_cross_month_boundaries() {
        let d = week(&[], date(2026, 10, 1));
        assert_eq!((d.previous, d.next), (date(2026, 9, 21), date(2026, 10, 5)));
        let back = week(&[], d.previous);
        assert_eq!(back.next, d.first);
    }

    #[test]
    fn previous_and_next_cross_year_boundaries() {
        let d = week(&[], date(2025, 1, 1));
        assert_eq!((d.previous, d.next), (date(2024, 12, 23), date(2025, 1, 6)));
        let back = week(&[], d.previous);
        assert_eq!(
            (back.first, back.last),
            (date(2024, 12, 23), date(2024, 12, 29))
        );
    }

    #[test]
    fn missing_metrics_count_as_zero_and_missing_ascent_is_reported() {
        let mut h = hike(1, "2026-09-30T08:00:00Z", None, 5.0);
        h.ascent_m = None;
        h.duration_moving_s = None;
        let hikes = [h, hike(2, "2026-09-30T09:00:00Z", None, 1.0)];
        let d = week(&hikes, date(2026, 10, 1));
        assert_eq!(
            d.totals,
            Metrics {
                distance_m: 6000.0,
                ascent_m: 100.0,
                moving_s: 3000,
                hike_count: 2
            }
        );
        assert_eq!(d.without_elevation, 1);
    }

    #[test]
    fn hikes_outside_the_window_are_not_reported_without_elevation() {
        let mut h = hike(1, "2026-09-20T08:00:00Z", None, 5.0);
        h.ascent_m = None;
        assert_eq!(week(&[h], date(2026, 10, 1)).without_elevation, 0);
    }

    #[test]
    fn a_hike_without_a_start_is_left_out() {
        let mut h = hike(1, "2026-09-30T08:00:00Z", None, 5.0);
        h.started_at = None;
        assert_eq!(week(&[h], date(2026, 10, 1)).totals.hike_count, 0);
    }

    fn filtered(filter: KindFilter) -> Vec<Vec<i64>> {
        let hikes = [
            kind(hike(1, "2026-09-28T08:00:00Z", None, 1.0), Some("walking")),
            kind(hike(2, "2026-09-28T09:00:00Z", None, 1.0), Some("cycling")),
            kind(hike(3, "2026-09-28T10:00:00Z", None, 1.0), None),
            kind(hike(4, "2026-09-28T11:00:00Z", None, 1.0), Some("Walking")),
        ];
        let d = aggregate(&hikes, View::Week, date(2026, 10, 1), &filter).unwrap();
        assert_eq!(d.totals.hike_count as usize, d.buckets[0].hike_ids.len());
        ids(&d)
    }

    #[test]
    fn the_all_filter_keeps_every_hike() {
        assert_eq!(filtered(KindFilter::All)[0], vec![1, 2, 3, 4]);
    }

    #[test]
    fn the_not_set_filter_keeps_hikes_without_a_kind() {
        assert_eq!(filtered(KindFilter::NotSet)[0], vec![3]);
    }

    #[test]
    fn a_kind_filter_keeps_that_exact_kind() {
        assert_eq!(filtered(KindFilter::Kind("walking".into()))[0], vec![1]);
        assert_eq!(
            filtered(KindFilter::Kind("skiing".into()))[0],
            Vec::<i64>::new()
        );
    }

    #[test]
    fn the_filter_also_applies_to_the_missing_elevation_count() {
        let mut h = kind(hike(1, "2026-09-30T08:00:00Z", None, 5.0), Some("cycling"));
        h.ascent_m = None;
        let d = aggregate(
            &[h],
            View::Week,
            date(2026, 10, 1),
            &KindFilter::Kind("walking".into()),
        )
        .unwrap();
        assert_eq!(d.without_elevation, 0);
    }

    #[test]
    fn a_window_beyond_the_supported_dates_is_an_error() {
        assert!(aggregate(&[], View::Week, Date::MAX, &KindFilter::All).is_err());
    }
}
