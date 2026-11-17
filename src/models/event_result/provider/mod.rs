use serde::Deserialize;
use utoipa::IntoParams;

// --------------------------------------------------
// EVENT RESULT - PROVIDER QUERY
// --------------------------------------------------

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EventResultParams {
    /// Provider source: `ifinish` or `mysamay`.
    pub source: String,

    /// Event year, for example `2026`. Required for iFinish (must be a year
    /// listed on iFinish), not used for MySamay.
    pub year: Option<String>,

    /// Event name. Required for both sources.
    /// iFinish: name exactly as shown on iFinish, case ignored
    /// (e.g. `Swachhathon 2026`; get valid names from `/ifinish-events`).
    /// MySamay: full or partial name (e.g. `Wipro Bengaluru Marathon 2026`).
    pub event_name: Option<String>,

    /// Participant BIB number, for example `21243`, `117` or `CMS-1008`.
    pub bib_number: String,
}
