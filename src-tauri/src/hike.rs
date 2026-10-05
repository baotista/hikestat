//! A hike as the list and the panel show it: the library's row, typed for TypeScript.

use hikestat::storage::{Db, Hike};
use serde::{Deserialize, Serialize};
use specta::Type;

/// A hike's id as a command argument (specta refuses a bare `i64`: it may not fit a TypeScript `number`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(transparent)]
pub struct HikeId(#[specta(type = specta_typescript::Number)] pub i64);

/// A hike without its points. Units and nullability follow the database.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HikeDetails {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    /// The raw GPX name, never modified.
    pub name: Option<String>,
    /// The title chosen by the user, shown instead of `name` when set.
    pub title: Option<String>,
    /// Free text written by the user.
    pub note: Option<String>,
    pub kind: Option<String>,
    /// Seconds since the Unix epoch (UTC).
    #[specta(type = Option<specta_typescript::Number>)]
    pub started_at: Option<i64>,
    /// IANA time zone name of the start, e.g. `Europe/Paris`.
    pub start_tz: Option<String>,
    pub distance_m: f64,
    pub ascent_m: Option<f64>,
    pub descent_m: Option<f64>,
    pub min_ele_m: Option<f64>,
    pub max_ele_m: Option<f64>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub duration_total_s: Option<i64>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub duration_moving_s: Option<i64>,
    pub avg_speed_mps: Option<f64>,
}

impl From<Hike> for HikeDetails {
    fn from(h: Hike) -> Self {
        Self {
            id: h.id,
            name: h.name,
            title: h.title,
            note: h.note,
            kind: h.kind,
            started_at: h.started_at,
            start_tz: h.start_tz,
            distance_m: h.distance_m,
            ascent_m: h.ascent_m,
            descent_m: h.descent_m,
            min_ele_m: h.min_ele_m,
            max_ele_m: h.max_ele_m,
            duration_total_s: h.duration_total_s,
            duration_moving_s: h.duration_moving_s,
            avg_speed_mps: h.avg_speed_mps,
        }
    }
}

/// The hike with this `id`, or an error naming it if there is none.
pub fn hike(db: &Db, id: i64) -> Result<HikeDetails, String> {
    db.hike(id)
        .map_err(|e| e.to_string())?
        .map(HikeDetails::from)
        .ok_or_else(|| format!("no hike with id {id}"))
}

/// The hikes with these `ids`, in the same order. Ids with no hike (deleted since the dashboard
/// was fetched) are left out.
pub fn hikes(db: &Db, ids: &[i64]) -> Result<Vec<HikeDetails>, String> {
    let mut found = Vec::with_capacity(ids.len());
    for &id in ids {
        if let Some(h) = db.hike(id).map_err(|e| e.to_string())? {
            found.push(h.into());
        }
    }
    Ok(found)
}

// Edits of the user's own fields. An empty or whitespace-only value clears the field.

/// Sets (or clears, if blank) the title of hike `id` and returns the updated hike.
pub fn update_title(db: &Db, id: i64, value: &str) -> Result<HikeDetails, String> {
    db.set_title(id, Some(value)).map_err(|e| e.to_string())?;
    hike(db, id)
}

/// Sets (or clears, if blank) the note of hike `id` and returns the updated hike.
pub fn update_note(db: &Db, id: i64, value: &str) -> Result<HikeDetails, String> {
    db.set_note(id, Some(value)).map_err(|e| e.to_string())?;
    hike(db, id)
}

// --- Kind edit -------------------------------------------------------------------------------

/// Sets the kind of the hike `id` (trimmed; empty or blank clears it) and returns the updated hike.
pub fn set_kind(db: &Db, id: i64, value: &str) -> Result<HikeDetails, String> {
    db.set_kind(id, value).map_err(|e| e.to_string())?;
    hike(db, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_with(ids: &[i64]) -> Db {
        let db = Db::open_in_memory().unwrap();
        for id in ids {
            db.connection()
                .execute(
                    "INSERT INTO hikes (id, name, distance_m) VALUES (?1, ?2, 1000)",
                    (*id, format!("h{id}")),
                )
                .unwrap();
        }
        db
    }

    #[test]
    fn an_existing_hike_is_returned() {
        let db = db_with(&[3]);
        let h = hike(&db, 3).unwrap();
        assert_eq!((h.id, h.name.as_deref(), h.ascent_m), (3, Some("h3"), None));
    }

    #[test]
    fn an_unknown_hike_is_an_error() {
        let db = db_with(&[3]);
        assert_eq!(hike(&db, 4), Err("no hike with id 4".to_string()));
    }

    #[test]
    fn hikes_keep_the_order_of_the_ids_and_skip_unknown_ones() {
        let db = db_with(&[1, 2, 3]);
        let ids: Vec<i64> = hikes(&db, &[3, 9, 1])
            .unwrap()
            .iter()
            .map(|h| h.id)
            .collect();
        assert_eq!(ids, vec![3, 1]);
        assert_eq!(hikes(&db, &[]).unwrap(), vec![]);
    }

    #[test]
    fn a_title_is_set_trimmed_then_cleared_and_the_name_is_kept() {
        let db = db_with(&[3]);
        let h = update_title(&db, 3, "  Mont Aiguille ").unwrap();
        assert_eq!(
            (h.title.as_deref(), h.name.as_deref()),
            (Some("Mont Aiguille"), Some("h3"))
        );
        assert_eq!(hike(&db, 3).unwrap(), h, "the edit is stored");

        let h = update_title(&db, 3, "   ").unwrap();
        assert_eq!((h.title, h.name.as_deref()), (None, Some("h3")));
    }

    #[test]
    fn a_note_is_set_with_its_line_breaks_then_cleared() {
        let db = db_with(&[3]);
        let h = update_note(&db, 3, "Snow.\n\nGreat view.\n").unwrap();
        assert_eq!(h.note.as_deref(), Some("Snow.\n\nGreat view."));
        assert_eq!(update_note(&db, 3, "").unwrap().note, None);
    }

    #[test]
    fn editing_an_unknown_hike_is_an_error() {
        let db = db_with(&[3]);
        assert_eq!(
            update_title(&db, 4, "x"),
            Err("no hike with id 4".to_string())
        );
        assert_eq!(
            update_note(&db, 4, "x"),
            Err("no hike with id 4".to_string())
        );
    }

    // --- Kind edit ---------------------------------------------------------------------------

    #[test]
    fn a_kind_is_set_trimmed_and_returned() {
        let db = db_with(&[3]);
        let h = set_kind(&db, 3, "  Walking ").unwrap();
        assert_eq!((h.id, h.kind.as_deref()), (3, Some("Walking")));
        assert_eq!(hike(&db, 3).unwrap(), h);
    }

    #[test]
    fn an_empty_kind_clears_it() {
        let db = db_with(&[3]);
        set_kind(&db, 3, "cycling").unwrap();
        assert_eq!(set_kind(&db, 3, "").unwrap().kind, None);
        set_kind(&db, 3, "cycling").unwrap();
        assert_eq!(set_kind(&db, 3, "   ").unwrap().kind, None);
    }

    #[test]
    fn setting_the_kind_of_an_unknown_hike_is_an_error() {
        let db = db_with(&[3]);
        assert_eq!(
            set_kind(&db, 4, "walking"),
            Err("no hike with id 4".to_string())
        );
    }
}
