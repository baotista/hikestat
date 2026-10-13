//! Hikes aggregated over the window of a dashboard view: pure, no I/O, no clock.
//!
//! A hike belongs to its **local date**: its start instant expressed in its start time zone (UTC
//! when the zone is missing or unknown). A hike crossing midnight belongs to its start day. Weeks
//! are ISO weeks, Monday first.

use crate::domain::storage::Hike;
use jiff::civil::Date;
use jiff::tz::{self, TimeZone};
use jiff::{Timestamp, ToSpan};
use serde::{Deserialize, Serialize};
use specta::Type;

/// How the dashboard cuts time. Every view but All shows the window containing the anchor date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum View {
    /// An ISO week, one bucket per day, Monday to Sunday.
    Week,
    /// A calendar month, one bucket per day.
    Month,
    /// A calendar year, one bucket per month.
    Year,
    /// One bucket per year, from the year of the first hike to the current year (just the current
    /// year when there is no hike). Ignores the anchor and has no previous or next window.
    All,
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
    /// Anchors of the previous and next windows (their first date); absent (`null`) in the All view.
    pub previous: Option<Date>,
    pub next: Option<Date>,
}

/// Aggregates `hikes` over the `view` window containing `anchor`, keeping those matching `filter`.
/// `today` is only used by the All view, which runs up to the current year. Hikes without a start
/// instant cannot be placed and are left out. Fails only when the window falls outside the dates
/// jiff can represent.
pub fn aggregate(
    hikes: &[Hike],
    view: View,
    anchor: Date,
    filter: &KindFilter,
    today: Date,
) -> Result<Dashboard, jiff::Error> {
    let Window {
        starts,
        last,
        previous,
        next,
    } = window(hikes, view, anchor, today)?;
    let first = starts[0];
    let mut buckets: Vec<Bucket> = starts
        .iter()
        .map(|&start| Bucket {
            start,
            metrics: Metrics::default(),
            hike_ids: Vec::new(),
        })
        .collect();
    let mut totals = Metrics::default();
    let mut without_elevation = 0;

    for h in hikes.iter().filter(|h| matches(filter, h)) {
        let Some(day) = local_date(h) else { continue };
        if day < first || day > last {
            continue;
        }
        // The last bucket starting on or before the day; there is one since `day >= first`.
        let bucket = &mut buckets[starts.partition_point(|&s| s <= day) - 1];
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
        previous,
        next,
    })
}

struct Window {
    /// First date of each bucket, in order; never empty.
    starts: Vec<Date>,
    last: Date,
    previous: Option<Date>,
    next: Option<Date>,
}

fn window(hikes: &[Hike], view: View, anchor: Date, today: Date) -> Result<Window, jiff::Error> {
    let days = |first: Date, n: i64| -> Result<Vec<Date>, jiff::Error> {
        (0..n).map(|i| first.checked_add(i.days())).collect()
    };
    Ok(match view {
        View::Week => {
            let first =
                anchor.checked_sub(i64::from(anchor.weekday().to_monday_zero_offset()).days())?;
            Window {
                starts: days(first, 7)?,
                last: first.checked_add(6.days())?,
                previous: Some(first.checked_sub(1.week())?),
                next: Some(first.checked_add(1.week())?),
            }
        }
        View::Month => {
            let first = anchor.first_of_month();
            Window {
                starts: days(first, i64::from(anchor.days_in_month()))?,
                last: anchor.last_of_month(),
                previous: Some(first.checked_sub(1.month())?),
                next: Some(first.checked_add(1.month())?),
            }
        }
        View::Year => {
            let first = anchor.first_of_year();
            Window {
                starts: (0..12)
                    .map(|i| first.checked_add(i.months()))
                    .collect::<Result<_, _>>()?,
                last: anchor.last_of_year(),
                previous: Some(first.checked_sub(1.year())?),
                next: Some(first.checked_add(1.year())?),
            }
        }
        View::All => {
            // The span covers every placeable hike whatever the filter, so that it stays put when
            // the filter changes, and reaches past the current year if some hikes do.
            let years = hikes.iter().filter_map(local_date).map(|d| d.year());
            let (from, to) = years.fold((today.year(), today.year()), |(lo, hi), y| {
                (lo.min(y), hi.max(y))
            });
            Window {
                starts: (from..=to)
                    .map(|y| Date::new(y, 1, 1))
                    .collect::<Result<_, _>>()?,
                last: Date::new(to, 12, 31)?,
                previous: None,
                next: None,
            }
        }
    })
}

pub(crate) fn matches(filter: &KindFilter, hike: &Hike) -> bool {
    match filter {
        KindFilter::All => true,
        KindFilter::NotSet => hike.kind.is_none(),
        KindFilter::Kind(kind) => hike.kind.as_deref() == Some(kind.as_str()),
    }
}

/// The start instant in the start time zone, UTC if the zone is missing or unknown.
pub(crate) fn local_date(hike: &Hike) -> Option<Date> {
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

    /// "Today" for the views that depend on it (All).
    const TODAY: Date = date(2026, 10, 5);

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
            title: None,
            note: None,
        }
    }

    fn kind(mut h: Hike, kind: Option<&str>) -> Hike {
        h.kind = kind.map(str::to_string);
        h
    }

    fn week(hikes: &[Hike], anchor: Date) -> Dashboard {
        aggregate(hikes, View::Week, anchor, &KindFilter::All, TODAY).unwrap()
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
        assert_eq!(d.next, Some(date(2026, 10, 26)));
        assert_eq!(week(&hikes, d.next.unwrap()).buckets[0].hike_ids, vec![3]);
    }

    #[test]
    fn iso_week_53_spans_the_new_year() {
        // 2026-W53: Monday 28 December 2026 to Sunday 3 January 2027.
        let hikes = [hike(1, "2027-01-02T10:00:00Z", Some("Europe/Paris"), 7.0)];
        let d = week(&hikes, date(2027, 1, 2));
        assert_eq!((d.first, d.last), (date(2026, 12, 28), date(2027, 1, 3)));
        assert_eq!(d.first.iso_week_date().week(), 53);
        assert_eq!(ids(&d)[5], vec![1]);
        assert_eq!(
            (d.previous, d.next),
            (Some(date(2026, 12, 21)), Some(date(2027, 1, 4)))
        );
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
        assert_eq!(
            (d.previous, d.next),
            (Some(date(2026, 9, 21)), Some(date(2026, 10, 5)))
        );
        let back = week(&[], d.previous.unwrap());
        assert_eq!(back.next, Some(d.first));
    }

    #[test]
    fn previous_and_next_cross_year_boundaries() {
        let d = week(&[], date(2025, 1, 1));
        assert_eq!(
            (d.previous, d.next),
            (Some(date(2024, 12, 23)), Some(date(2025, 1, 6)))
        );
        let back = week(&[], d.previous.unwrap());
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
    fn a_hike_without_ascent_counts_as_zero_in_its_bucket_and_is_reported() {
        let mut a = hike(1, "2026-09-28T08:00:00Z", None, 5.0);
        a.ascent_m = None;
        let mut b = hike(2, "2026-10-02T08:00:00Z", None, 5.0);
        b.ascent_m = None;
        let hikes = [a, b, hike(3, "2026-10-02T09:00:00Z", None, 1.0)];
        let d = week(&hikes, date(2026, 10, 1));
        assert_eq!(d.buckets[0].metrics.ascent_m, 0.0);
        assert_eq!(d.buckets[0].metrics.hike_count, 1);
        assert_eq!(d.buckets[4].metrics.ascent_m, 100.0);
        assert_eq!(d.totals.ascent_m, 100.0);
        assert_eq!(d.without_elevation, 2);
    }

    #[test]
    fn a_zero_ascent_is_not_missing_elevation() {
        let mut flat = hike(1, "2026-09-30T08:00:00Z", None, 5.0);
        flat.ascent_m = Some(0.0);
        assert_eq!(week(&[flat], date(2026, 10, 1)).without_elevation, 0);
    }

    #[test]
    fn a_missing_moving_time_is_not_missing_elevation() {
        let mut h = hike(1, "2026-09-30T08:00:00Z", None, 5.0);
        h.duration_moving_s = None;
        let d = week(&[h], date(2026, 10, 1));
        assert_eq!((d.totals.moving_s, d.totals.hike_count), (0, 1));
        assert_eq!(d.without_elevation, 0);
    }

    #[test]
    fn moving_time_and_hike_count_totals_sum_every_bucket_in_every_view() {
        let mut long = hike(1, "2026-03-02T08:00:00Z", None, 20.0);
        long.duration_moving_s = Some(6 * 3600);
        let mut short = hike(2, "2026-03-02T15:00:00Z", None, 2.0);
        short.duration_moving_s = Some(1800);
        let mut later = hike(3, "2026-03-05T08:00:00Z", None, 8.0);
        later.duration_moving_s = Some(7200);
        let hikes = [long, short, later];
        for view in [View::Week, View::Month, View::Year, View::All] {
            let d = aggregate(&hikes, view, date(2026, 3, 4), &KindFilter::All, TODAY).unwrap();
            assert_eq!(d.totals.moving_s, 6 * 3600 + 1800 + 7200, "{view:?}");
            assert_eq!(d.totals.hike_count, 3, "{view:?}");
            let moving: i64 = d.buckets.iter().map(|b| b.metrics.moving_s).sum();
            let count: u32 = d.buckets.iter().map(|b| b.metrics.hike_count).sum();
            assert_eq!((moving, count), (d.totals.moving_s, d.totals.hike_count));
        }
        let w = week(&hikes, date(2026, 3, 4));
        assert_eq!(
            (
                w.buckets[0].metrics.moving_s,
                w.buckets[0].metrics.hike_count
            ),
            (6 * 3600 + 1800, 2)
        );
        assert_eq!(
            (
                w.buckets[3].metrics.moving_s,
                w.buckets[3].metrics.hike_count
            ),
            (7200, 1)
        );
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
        let d = aggregate(&hikes, View::Week, date(2026, 10, 1), &filter, TODAY).unwrap();
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
            TODAY,
        )
        .unwrap();
        assert_eq!(d.without_elevation, 0);
    }

    #[test]
    fn a_window_beyond_the_supported_dates_is_an_error() {
        assert!(aggregate(&[], View::Week, Date::MAX, &KindFilter::All, TODAY).is_err());
    }

    fn month(hikes: &[Hike], anchor: Date) -> Dashboard {
        aggregate(hikes, View::Month, anchor, &KindFilter::All, TODAY).unwrap()
    }

    fn year(hikes: &[Hike], anchor: Date) -> Dashboard {
        aggregate(hikes, View::Year, anchor, &KindFilter::All, TODAY).unwrap()
    }

    fn all(hikes: &[Hike], today: Date) -> Dashboard {
        // The anchor plays no part in the All view.
        aggregate(hikes, View::All, date(2000, 6, 15), &KindFilter::All, today).unwrap()
    }

    fn n_days_from(first: Date, n: i64) -> Vec<Date> {
        (0..n)
            .map(|i| first.checked_add(i.days()).unwrap())
            .collect()
    }

    #[test]
    fn a_month_has_one_bucket_per_day() {
        let d = month(&[], date(2026, 9, 17));
        assert_eq!(d.view, View::Month);
        assert_eq!((d.first, d.last), (date(2026, 9, 1), date(2026, 9, 30)));
        assert_eq!(days(&d), n_days_from(date(2026, 9, 1), 30));
    }

    #[test]
    fn months_have_28_to_31_days_and_february_has_29_in_a_leap_year() {
        let len = |y, m| month(&[], date(y, m, 1)).buckets.len();
        assert_eq!(len(2026, 2), 28);
        assert_eq!(len(2028, 2), 29);
        assert_eq!(len(2026, 4), 30);
        assert_eq!(len(2026, 1), 31);
        let feb = month(&[], date(2028, 2, 10));
        assert_eq!((feb.first, feb.last), (date(2028, 2, 1), date(2028, 2, 29)));
    }

    #[test]
    fn any_day_of_the_month_anchors_the_same_window() {
        assert_eq!(month(&[], date(2026, 9, 1)), month(&[], date(2026, 9, 30)));
    }

    #[test]
    fn a_month_puts_hikes_on_their_local_day() {
        let hikes = [
            hike(1, "2026-09-01T08:00:00Z", Some("Europe/Paris"), 5.0),
            hike(2, "2026-09-30T08:00:00Z", Some("Europe/Paris"), 3.0),
            hike(3, "2026-09-30T21:00:00Z", Some("Asia/Tokyo"), 8.0), // 06:00 on 1 Oct in Tokyo
            hike(4, "2026-08-31T23:30:00Z", Some("Europe/Paris"), 1.0), // 01:30 on 1 Sep in Paris
        ];
        let d = month(&hikes, date(2026, 9, 1));
        assert_eq!(d.buckets[0].hike_ids, vec![1, 4]);
        assert_eq!(d.buckets[29].hike_ids, vec![2]);
        assert_eq!(d.totals.hike_count, 3);
        assert_eq!(d.totals.distance_m, 9000.0);
        assert_eq!(
            month(&hikes, date(2026, 10, 1)).buckets[0].hike_ids,
            vec![3]
        );
    }

    #[test]
    fn month_previous_and_next_cross_year_boundaries() {
        let jan = month(&[], date(2027, 1, 20));
        assert_eq!(
            (jan.previous, jan.next),
            (Some(date(2026, 12, 1)), Some(date(2027, 2, 1)))
        );
        let dec = month(&[], date(2026, 12, 31));
        assert_eq!(
            (dec.previous, dec.next),
            (Some(date(2026, 11, 1)), Some(date(2027, 1, 1)))
        );
    }

    #[test]
    fn month_navigation_from_a_31st_does_not_skip_short_months() {
        let d = month(&[], date(2026, 1, 31));
        assert_eq!(d.next, Some(date(2026, 2, 1)));
        assert_eq!(month(&[], d.next.unwrap()).buckets.len(), 28);
    }

    #[test]
    fn a_year_has_twelve_monthly_buckets() {
        let d = year(&[], date(2026, 7, 14));
        assert_eq!(d.view, View::Year);
        assert_eq!((d.first, d.last), (date(2026, 1, 1), date(2026, 12, 31)));
        let starts: Vec<Date> = (1..=12).map(|m| date(2026, m, 1)).collect();
        assert_eq!(days(&d), starts);
        for b in &d.buckets {
            assert_eq!(b.metrics, Metrics::default());
        }
    }

    #[test]
    fn a_year_puts_a_hike_in_its_local_month() {
        let hikes = [
            hike(1, "2026-09-30T21:00:00Z", Some("Asia/Tokyo"), 8.0), // 06:00 on 1 Oct in Tokyo
            hike(2, "2026-09-30T21:00:00Z", None, 2.0),               // 30 Sep in UTC
            hike(3, "2026-01-01T10:00:00Z", None, 1.0),
            hike(4, "2026-12-31T23:30:00Z", Some("Europe/Paris"), 1.0), // 00:30 on 1 Jan 2027 in Paris
        ];
        let d = year(&hikes, date(2026, 5, 5));
        assert_eq!(
            ids(&d),
            vec![
                vec![3],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![2],
                vec![1],
                vec![],
                vec![]
            ]
        );
        assert_eq!(d.buckets[9].metrics.distance_m, 8000.0);
        assert_eq!(d.totals.hike_count, 3);
        assert_eq!(year(&hikes, date(2027, 1, 1)).buckets[0].hike_ids, vec![4]);
    }

    #[test]
    fn year_previous_and_next_are_the_first_days_of_the_adjacent_years() {
        let d = year(&[], date(2026, 7, 14));
        assert_eq!(
            (d.previous, d.next),
            (Some(date(2025, 1, 1)), Some(date(2027, 1, 1)))
        );
        // From a leap day too.
        let leap = year(&[], date(2028, 2, 29));
        assert_eq!(
            (leap.previous, leap.next),
            (Some(date(2027, 1, 1)), Some(date(2029, 1, 1)))
        );
    }

    #[test]
    fn all_spans_from_the_first_hike_year_to_the_current_year() {
        let hikes = [
            hike(1, "2021-06-01T08:00:00Z", None, 5.0),
            hike(2, "2019-03-01T08:00:00Z", None, 2.0),
            hike(3, "2026-09-30T08:00:00Z", None, 1.0),
        ];
        let d = all(&hikes, TODAY);
        assert_eq!(d.view, View::All);
        assert_eq!((d.first, d.last), (date(2019, 1, 1), date(2026, 12, 31)));
        let starts: Vec<Date> = (2019..=2026).map(|y| date(y, 1, 1)).collect();
        assert_eq!(days(&d), starts);
        assert_eq!(
            ids(&d),
            vec![
                vec![2],
                vec![],
                vec![1],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![3]
            ]
        );
        assert_eq!(d.buckets[1].metrics, Metrics::default());
        assert_eq!(d.totals.hike_count, 3);
    }

    #[test]
    fn all_reaches_the_current_year_even_without_recent_hikes() {
        let hikes = [hike(1, "2024-06-01T08:00:00Z", None, 5.0)];
        let d = all(&hikes, date(2026, 1, 1));
        assert_eq!((d.first, d.last), (date(2024, 1, 1), date(2026, 12, 31)));
        assert_eq!(ids(&d), vec![vec![1], vec![], vec![]]);
    }

    #[test]
    fn all_uses_the_local_year_of_the_first_hike() {
        // 1 January 2020 at 08:00 in Tokyo is still 2019 in UTC.
        let hikes = [hike(1, "2019-12-31T23:00:00Z", Some("Asia/Tokyo"), 5.0)];
        let d = all(&hikes, TODAY);
        assert_eq!(d.first, date(2020, 1, 1));
        assert_eq!(d.buckets[0].hike_ids, vec![1]);
    }

    #[test]
    fn all_keeps_hikes_dated_after_the_current_year() {
        // A clock set in the past must not hide hikes.
        let hikes = [hike(1, "2027-03-01T08:00:00Z", None, 5.0)];
        let d = all(&hikes, TODAY);
        assert_eq!((d.first, d.last), (date(2026, 1, 1), date(2027, 12, 31)));
        assert_eq!(ids(&d), vec![vec![], vec![1]]);
    }

    #[test]
    fn all_on_an_empty_database_is_the_current_year() {
        let d = all(&[], TODAY);
        assert_eq!((d.first, d.last), (date(2026, 1, 1), date(2026, 12, 31)));
        assert_eq!(days(&d), vec![date(2026, 1, 1)]);
        assert_eq!(d.totals, Metrics::default());
    }

    #[test]
    fn all_has_no_previous_or_next() {
        let hikes = [hike(1, "2021-06-01T08:00:00Z", None, 5.0)];
        let d = all(&hikes, TODAY);
        assert_eq!((d.previous, d.next), (None, None));
    }

    #[test]
    fn the_all_span_does_not_depend_on_the_kind_filter() {
        let hikes = [
            kind(hike(1, "2019-06-01T08:00:00Z", None, 5.0), Some("cycling")),
            kind(hike(2, "2024-06-01T08:00:00Z", None, 5.0), Some("walking")),
        ];
        let filter = KindFilter::Kind("walking".into());
        let d = aggregate(&hikes, View::All, TODAY, &filter, TODAY).unwrap();
        assert_eq!(d.first, date(2019, 1, 1));
        assert_eq!(d.buckets[0].hike_ids, Vec::<i64>::new());
        assert_eq!(d.buckets[5].hike_ids, vec![2]);
        assert_eq!(d.totals.hike_count, 1);
    }

    #[test]
    fn the_kind_filter_applies_in_every_view() {
        let hikes = [
            kind(hike(1, "2026-09-28T08:00:00Z", None, 1.0), Some("walking")),
            kind(hike(2, "2026-09-28T09:00:00Z", None, 1.0), Some("cycling")),
            kind(hike(3, "2026-09-28T10:00:00Z", None, 1.0), None),
        ];
        for view in [View::Week, View::Month, View::Year, View::All] {
            let count = |filter: KindFilter| {
                let d = aggregate(&hikes, view, date(2026, 9, 28), &filter, TODAY).unwrap();
                let ids: Vec<i64> = d.buckets.iter().flat_map(|b| b.hike_ids.clone()).collect();
                (d.totals.hike_count, ids)
            };
            assert_eq!(count(KindFilter::All), (3, vec![1, 2, 3]), "{view:?}");
            assert_eq!(count(KindFilter::NotSet), (1, vec![3]), "{view:?}");
            assert_eq!(
                count(KindFilter::Kind("cycling".into())),
                (1, vec![2]),
                "{view:?}"
            );
        }
    }

    /// Hikes of several kinds spread over days, months and years, some without elevation.
    fn mixed_kinds() -> Vec<Hike> {
        let h = |id, start, k, km, ascent, moving| {
            let mut h = kind(hike(id, start, None, km), k);
            h.ascent_m = ascent;
            h.duration_moving_s = Some(moving);
            h
        };
        vec![
            h(
                1,
                "2026-09-28T08:00:00Z",
                Some("walking"),
                10.0,
                Some(300.0),
                7200,
            ),
            h(
                2,
                "2026-09-28T09:00:00Z",
                Some("cycling"),
                60.0,
                Some(500.0),
                9000,
            ),
            h(3, "2026-09-30T08:00:00Z", None, 4.0, None, 3600),
            h(4, "2026-09-30T10:00:00Z", Some("walking"), 6.0, None, 5400),
            h(
                5,
                "2026-03-10T08:00:00Z",
                Some("walking"),
                8.0,
                Some(200.0),
                6000,
            ),
            h(6, "2024-05-01T08:00:00Z", None, 3.0, Some(50.0), 1800),
        ]
    }

    /// The non-empty buckets: their start, hike ids and distance.
    fn filled(d: &Dashboard) -> Vec<(Date, Vec<i64>, f64)> {
        d.buckets
            .iter()
            .filter(|b| !b.hike_ids.is_empty())
            .map(|b| (b.start, b.hike_ids.clone(), b.metrics.distance_m))
            .collect()
    }

    fn metrics(km: f64, ascent_m: f64, moving_s: i64, hike_count: u32) -> Metrics {
        Metrics {
            distance_m: km * 1000.0,
            ascent_m,
            moving_s,
            hike_count,
        }
    }

    #[test]
    fn the_kind_filter_shapes_buckets_totals_and_missing_elevation_in_every_view() {
        let hikes = mixed_kinds();
        let walking = KindFilter::Kind("walking".into());
        let anchor = date(2026, 9, 28);
        type Expected = (Vec<(Date, Vec<i64>, f64)>, Metrics, u32);
        let cases: Vec<(View, KindFilter, Expected)> = vec![
            (
                View::Week,
                KindFilter::All,
                (
                    vec![
                        (date(2026, 9, 28), vec![1, 2], 70_000.0),
                        (date(2026, 9, 30), vec![3, 4], 10_000.0),
                    ],
                    metrics(80.0, 800.0, 25_200, 4),
                    2,
                ),
            ),
            (
                View::Week,
                KindFilter::NotSet,
                (
                    vec![(date(2026, 9, 30), vec![3], 4_000.0)],
                    metrics(4.0, 0.0, 3600, 1),
                    1,
                ),
            ),
            (
                View::Week,
                walking.clone(),
                (
                    vec![
                        (date(2026, 9, 28), vec![1], 10_000.0),
                        (date(2026, 9, 30), vec![4], 6_000.0),
                    ],
                    metrics(16.0, 300.0, 12_600, 2),
                    1,
                ),
            ),
            (
                View::Month,
                KindFilter::All,
                (
                    vec![
                        (date(2026, 9, 28), vec![1, 2], 70_000.0),
                        (date(2026, 9, 30), vec![3, 4], 10_000.0),
                    ],
                    metrics(80.0, 800.0, 25_200, 4),
                    2,
                ),
            ),
            (
                View::Month,
                KindFilter::NotSet,
                (
                    vec![(date(2026, 9, 30), vec![3], 4_000.0)],
                    metrics(4.0, 0.0, 3600, 1),
                    1,
                ),
            ),
            (
                View::Month,
                walking.clone(),
                (
                    vec![
                        (date(2026, 9, 28), vec![1], 10_000.0),
                        (date(2026, 9, 30), vec![4], 6_000.0),
                    ],
                    metrics(16.0, 300.0, 12_600, 2),
                    1,
                ),
            ),
            (
                View::Year,
                KindFilter::All,
                (
                    vec![
                        (date(2026, 3, 1), vec![5], 8_000.0),
                        (date(2026, 9, 1), vec![1, 2, 3, 4], 80_000.0),
                    ],
                    metrics(88.0, 1000.0, 31_200, 5),
                    2,
                ),
            ),
            (
                View::Year,
                KindFilter::NotSet,
                (
                    vec![(date(2026, 9, 1), vec![3], 4_000.0)],
                    metrics(4.0, 0.0, 3600, 1),
                    1,
                ),
            ),
            (
                View::Year,
                walking.clone(),
                (
                    vec![
                        (date(2026, 3, 1), vec![5], 8_000.0),
                        (date(2026, 9, 1), vec![1, 4], 16_000.0),
                    ],
                    metrics(24.0, 500.0, 18_600, 3),
                    1,
                ),
            ),
            (
                View::All,
                KindFilter::All,
                (
                    vec![
                        (date(2024, 1, 1), vec![6], 3_000.0),
                        (date(2026, 1, 1), vec![1, 2, 3, 4, 5], 88_000.0),
                    ],
                    metrics(91.0, 1050.0, 33_000, 6),
                    2,
                ),
            ),
            (
                View::All,
                KindFilter::NotSet,
                (
                    vec![
                        (date(2024, 1, 1), vec![6], 3_000.0),
                        (date(2026, 1, 1), vec![3], 4_000.0),
                    ],
                    metrics(7.0, 50.0, 5400, 2),
                    1,
                ),
            ),
            (
                View::All,
                walking,
                (
                    vec![(date(2026, 1, 1), vec![1, 4, 5], 24_000.0)],
                    metrics(24.0, 500.0, 18_600, 3),
                    1,
                ),
            ),
        ];
        for (view, filter, (buckets, totals, without_elevation)) in cases {
            let d = aggregate(&hikes, view, anchor, &filter, TODAY).unwrap();
            let label = format!("{view:?} {filter:?}");
            assert_eq!(filled(&d), buckets, "{label}");
            assert_eq!(d.totals, totals, "{label}");
            assert_eq!(d.without_elevation, without_elevation, "{label}");
            if view == View::All {
                assert_eq!((d.first, d.last), (date(2024, 1, 1), date(2026, 12, 31)));
            }
        }
    }

    #[test]
    fn the_filter_does_not_change_the_window_or_its_buckets() {
        let hikes = mixed_kinds();
        for view in [View::Week, View::Month, View::Year, View::All] {
            let shape = |filter: KindFilter| {
                let d = aggregate(&hikes, view, date(2026, 9, 28), &filter, TODAY).unwrap();
                (d.first, d.last, days(&d), d.previous, d.next)
            };
            let all = shape(KindFilter::All);
            assert_eq!(shape(KindFilter::NotSet), all, "{view:?}");
            assert_eq!(shape(KindFilter::Kind("skiing".into())), all, "{view:?}");
        }
    }

    #[test]
    fn a_kind_matching_no_hike_gives_empty_buckets() {
        let d = aggregate(
            &mixed_kinds(),
            View::Year,
            date(2026, 9, 28),
            &KindFilter::Kind("skiing".into()),
            TODAY,
        )
        .unwrap();
        assert_eq!(d.buckets.len(), 12);
        assert_eq!(filled(&d), vec![]);
        assert_eq!(
            (d.totals.clone(), d.without_elevation),
            (Metrics::default(), 0)
        );
    }

    #[test]
    fn switching_views_shows_the_window_containing_the_anchor() {
        let anchor = date(2026, 3, 18); // a Wednesday in March 2026
        let w = week(&[], anchor);
        let m = month(&[], anchor);
        let y = year(&[], anchor);
        assert_eq!((w.first, w.last), (date(2026, 3, 16), date(2026, 3, 22)));
        assert_eq!((m.first, m.last), (date(2026, 3, 1), date(2026, 3, 31)));
        assert_eq!((y.first, y.last), (date(2026, 1, 1), date(2026, 12, 31)));
    }

    #[test]
    fn month_and_year_windows_beyond_the_supported_dates_are_errors() {
        for view in [View::Month, View::Year] {
            assert!(aggregate(&[], view, Date::MAX, &KindFilter::All, TODAY).is_err());
        }
    }
}
