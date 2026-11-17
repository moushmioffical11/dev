/// Columns shared by the list and the detail query (everything except raw_data).
/// A macro (not a const) so it can be joined into a static SQL string with concat!.
macro_rules! event_columns {
    () => {
        "id, provider, event_id, event_code, event_name, event_type, event_date, \
         organizer, dates, location, links, images, details, prices, \
         created_at, updated_at"
    };
}

/// List query. Fully static (sqlx 0.9 only accepts SQL written in the code).
/// Sorting: past events newest first, everything else soonest first.
pub const LIST_SQL: &str = concat!(
    "SELECT ",
    event_columns!(),
    ", CASE WHEN $6 THEN raw_data END AS raw_data",
    ", COUNT(*) OVER () AS total ",
    "FROM events ",
    "WHERE ($1::text IS NULL OR provider = $1) ",
    "AND ( $2::text IS NULL ",
    "OR ($2 = 'upcoming' AND event_date >= (now() AT TIME ZONE 'Asia/Kolkata')::date) ",
    "OR ($2 = 'past' AND event_date < (now() AT TIME ZONE 'Asia/Kolkata')::date) ) ",
    "AND ($3::text IS NULL OR event_name ILIKE '%' || $3 || '%') ",
    "ORDER BY ",
    "CASE WHEN $2 = 'past' THEN event_date END DESC NULLS LAST, ",
    "CASE WHEN $2 IS DISTINCT FROM 'past' THEN event_date END ASC NULLS LAST, ",
    "event_name ASC ",
    "LIMIT $4 OFFSET $5"
);

/// Detail query: one event with its full raw_data.
pub const DETAIL_SQL: &str = concat!(
    "SELECT ",
    event_columns!(),
    ", raw_data, 1::bigint AS total FROM events WHERE id = $1"
);
