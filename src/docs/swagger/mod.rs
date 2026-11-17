use crate::models::SyncSummary;
use crate::models::user::event_results::get::UserEventResultsGetResponse;
use crate::models::user::event_results::{
    EventResultResponse, EventResultResponseData, ManualEventResultRequest,
};
use crate::models::user::events_info::UserEventsInfoResponse;
use crate::models::user::get_events_info::UserEventsInfoGetResponse;
use crate::models::{
    CrawlPage, CrawlResponse, EventDetailResponse, EventErrorResponse, EventListResponse,
    EventResponse, EventResultParams, ForgotPasswordRequest, ForgotPasswordResponse, IfinishEvent,
    IfinishEventsResponse, LoginRequest, LoginResponse, MapResponse, ReaderResponse,
    RegisterRequest, RegisterResponse, ResolveResponse, SearchResponse, SearchResult,
    UserEventsInfoRequest,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/* ============================================================
WEB READER
============================================================ */

/// Read a web page
///
/// Opens `url` in a headless Chrome browser, waits for JavaScript to finish
/// rendering and returns the page as clean text. Use it for pages that load
/// their content dynamically (single-page apps), where a plain HTTP fetch
/// returns an empty shell.
///
/// **How it works**
///
/// - Waits until the rendered page stops changing (up to 20 seconds) and
///   scrolls to the bottom so lazy-loaded content and images are rendered.
/// - `content` is the visible text of the page, one block per line. Scripts,
///   styles and hidden templates are removed.
/// - `links` contains only links on the **same host** as `url` (`www.` is
///   ignored). They are absolute URLs, without `#fragment`, and de-duplicated.
/// - `images` contains image URLs found in `src`, `data-src`,
///   `data-lazy-src`, `data-original` and the first `srcset` entry, converted
///   to absolute URLs. Images can be hosted on any domain (for example a CDN).
///   Inline `data:` images are skipped.
///
/// Typically takes 5-25 seconds. The URL must include `https://` or `http://`.
///
/// **Example**
///
/// ```bash
/// curl -G "http://localhost:8080/read" --data-urlencode "url=https://ifinish.in/eventresult"
/// ```
#[utoipa::path(
    get,
    path = "/read",
    params(
        ("url" = String, Query, description = "Full URL of the page to read, including https://. Example: https://ifinish.in/eventresult")
    ),
    responses(
        (status = 200, description = "Page rendered and extracted. Contains the title, visible text, same-host links and image URLs.", body = ReaderResponse),
        (status = 400, description = "Invalid URL: it could not be parsed, has no host, or the browser could not navigate to it"),
        (status = 500, description = "Server error: the browser could not be started (CHROME_PATH is not set), the page failed to render, or the worker failed")
    ),
    tag = "Web Reader"
)]
fn _read_docs() {}

/// Map the links of a page
///
/// Renders `url` exactly like `/read` but returns **only the links** found on
/// the page. Use it to discover which pages exist on a site before reading or
/// crawling them.
///
/// - Only links on the same host as `url` are returned (`www.` is ignored).
/// - Links are absolute URLs, without `#fragment`, and de-duplicated.
/// - Links that are not real `<a href>` anchors (for example cards that open
///   a page with JavaScript on click) are **not** included. Use `/resolve` for
///   those.
///
/// Typically takes 5-25 seconds.
///
/// **Example**
///
/// ```bash
/// curl -G "http://localhost:8080/map" --data-urlencode "url=https://ifinish.in/"
/// ```
#[utoipa::path(
    get,
    path = "/map",
    params(
        ("url" = String, Query, description = "Full URL of the page to map, including https://")
    ),
    responses(
        (status = 200, description = "Page rendered. Contains the list of same-host links.", body = MapResponse),
        (status = 400, description = "Invalid URL: it could not be parsed, has no host, or the browser could not navigate to it"),
        (status = 500, description = "Server error: the browser could not be started, the page failed to render, or the worker failed")
    ),
    tag = "Web Reader"
)]
fn _map_docs() {}

/// Find the page URL of an event card
///
/// Many listing pages (for example the MySamay results home) show events as
/// cards that open their page with JavaScript, so the real address is not in
/// any `<a href>` and `/read` or `/map` cannot see it. This endpoint opens the
/// listing page, finds the card that contains `name`, clicks it and returns
/// the URL the site navigates to.
///
/// **How it works**
///
/// - The listing page `url` is opened and rendered (up to 20 seconds).
/// - The card is found by comparing text with **case and punctuation
///   ignored**, so `hyderabad hitec marathon 25` matches
///   `Hyderabad Hitec Marathon 25`. The smallest element containing the name
///   wins, and the nearest clickable parent (link, button, `onclick` or
///   pointer cursor) is clicked.
/// - It then waits up to 12 seconds for the site to open a different page that
///   has at least two path segments. New browser tabs are also checked.
///
/// The request itself is always `200` when the browser ran successfully.
/// **`url` is `null` when no card matched or nothing opened.**
///
/// **Example**
///
/// ```bash
/// curl -G "http://localhost:8080/resolve" \
///   --data-urlencode "url=https://www.mysamay.in/event/results/home" \
///   --data-urlencode "name=Hyderabad Hitec Marathon 25"
/// ```
#[utoipa::path(
    get,
    path = "/resolve",
    params(
        ("url" = String, Query, description = "Listing page that shows the event cards. Example: https://www.mysamay.in/event/results/home"),
        ("name" = String, Query, description = "Full or partial event name as shown on the card. Case and punctuation are ignored.")
    ),
    responses(
        (status = 200, description = "Request completed. `url` is the event page URL, or null if no matching card was found or nothing opened.", body = ResolveResponse),
        (status = 400, description = "Invalid listing URL, or the browser could not navigate to it"),
        (status = 500, description = "Server error: the browser could not be started, the page failed to render, or the worker failed")
    ),
    tag = "Web Reader"
)]
fn _resolve_docs() {}

/// Crawl a website
///
/// Starts at `url` and follows same-host links breadth first, reading every
/// page like `/read`. Use it when you need the text of several connected
/// pages in one call.
///
/// **How it works**
///
/// - The start page is depth `0`. Its links are depth `1`, and so on.
/// - Only links on the same host are followed (`www.` is ignored). Each URL is
///   visited once.
/// - Pages that fail to load are skipped and the crawl continues.
/// - The crawl stops when `max_pages` pages were read or no pages within
///   `max_depth` are left.
///
/// **Limits**
///
/// - `max_pages`: default `5`, maximum `20`.
/// - `max_depth`: default `1`, maximum `5`.
///
/// **This endpoint is slow.** Every page takes about 8-10 seconds, so 5 pages
/// need about a minute and 20 pages can take over 3 minutes. Start with small
/// values.
///
/// **Example**
///
/// ```bash
/// curl -G "http://localhost:8080/crawl" \
///   --data-urlencode "url=https://ifinish.in/" \
///   --data-urlencode "max_pages=3" \
///   --data-urlencode "max_depth=1"
/// ```
#[utoipa::path(
    get,
    path = "/crawl",
    params(
        ("url" = String, Query, description = "Starting URL, including https://"),
        ("max_pages" = Option<usize>, Query, description = "Maximum number of pages to read. Default 5, maximum 20."),
        ("max_depth" = Option<usize>, Query, description = "How many link levels to follow from the start page. Default 1, maximum 5.")
    ),
    responses(
        (status = 200, description = "Crawl finished. Contains every page that was read successfully. The list can be empty if no page loaded.", body = CrawlResponse),
        (status = 400, description = "Invalid starting URL or the URL has no host"),
        (status = 500, description = "Server error: the browser could not be started (CHROME_PATH is not set)")
    ),
    tag = "Web Reader"
)]
fn _crawl_docs() {}

/// Search text across a website
///
/// Crawls the site like `/crawl` and returns a short text snippet for every
/// page that contains your search text.
///
/// **How it works**
///
/// - Starts at `url` (depth `0`) and follows same-host links breadth first.
/// - The search is **case-insensitive** and looks for `q` as plain text in the
///   visible content of each page.
/// - For each matching page, `matched_content` is the text around the **first**
///   match (about 100 characters before and after, wrapped in `...`).
///   Pages without a match are not listed.
/// - `pages_searched` is the number of pages that were read successfully, not
///   the number of matches.
///
/// **Limits**
///
/// - `max_pages`: default `5`, maximum `20`.
/// - `max_depth`: default `1`, maximum `5`.
///
/// Like `/crawl`, each page takes about 8-10 seconds, so keep the limits small.
///
/// **Example**
///
/// ```bash
/// curl -G "http://localhost:8080/search" \
///   --data-urlencode "url=https://ifinish.in/" \
///   --data-urlencode "q=marathon" \
///   --data-urlencode "max_pages=3"
/// ```
#[utoipa::path(
    get,
    path = "/search",
    params(
        ("url" = String, Query, description = "Starting URL, including https://"),
        ("q" = String, Query, description = "Text to search for. Case-insensitive. Must not be empty."),
        ("max_pages" = Option<usize>, Query, description = "Maximum number of pages to search. Default 5, maximum 20."),
        ("max_depth" = Option<usize>, Query, description = "How many link levels to follow from the start page. Default 1, maximum 5.")
    ),
    responses(
        (status = 200, description = "Search finished. `results` lists only the pages that contain the text and can be empty.", body = SearchResponse),
        (status = 400, description = "The search text `q` is empty, the starting URL is invalid, or the URL has no host"),
        (status = 500, description = "Server error: the browser could not be started (CHROME_PATH is not set)")
    ),
    tag = "Web Reader"
)]
fn _search_docs() {}

/* ============================================================
EVENT RESULTS
============================================================ */

/// Get a participant's result for an event
///
/// Opens the result site in a headless browser, finds the participant by BIB
/// number and returns the rendered result page (name, times, ranks, etc.).
///
/// **Supported sources**
///
/// - `ifinish` - requires `year`, `event_name` and `bib_number`. The event
///   name must match exactly as shown on iFinish (case is ignored), for
///   example `Ananthagiri Hills Trail Run 2026`. Works for male and female
///   participants in any category. Use `/ifinish-events` to get valid names.
/// - `mysamay` - requires `event_name` and `bib_number`. A full or partial
///   event name is accepted (case and punctuation are ignored), for example
///   `Wipro Bengaluru Marathon 2026`. `year` is not used.
///
/// **How it works**
///
/// - **iFinish:** the year and event are selected, then your BIB is chosen
///   from the site's BIB suggestions and Search is clicked. This is why any
///   BIB works, not only the ones shown in the top results. BIBs can contain
///   letters and dashes (for example `CMS-1008`). The participant's detail page
///   is returned.
/// - **MySamay:** the event card matching `event_name` is opened through its
///   RESULTS button, the BIB is entered in Find Participant, and the filtered
///   leaderboard row (BIB, name, category, official time, overall / gender /
///   age ranks) is returned. Events without a RESULTS button are skipped.
///
/// **Response**
///
/// `content` holds the text of the result page. For iFinish `url` is always
/// `https://ifinish.in/eventresult`. For MySamay `url` is the event results page
/// (`https://www.mysamay.in/race/results/<event-id>`).
///
/// **Timing:** a request takes 10-60 seconds because the page is rendered in a
/// real browser, and each request starts its own browser, so avoid sending many
/// at the same time. An unknown BIB returns `404` after roughly 5-20 seconds.
///
/// **Examples**
///
/// ```bash
/// # iFinish
/// curl -G "http://localhost:8080/event-result" \
///   --data-urlencode "source=ifinish" \
///   --data-urlencode "year=2026" \
///   --data-urlencode "event_name=BHARAT RACE CHANDIGARH 2026" \
///   --data-urlencode "bib_number=CMS-1008"
///
/// # MySamay
/// curl -G "http://localhost:8080/event-result" \
///   --data-urlencode "source=mysamay" \
///   --data-urlencode "event_name=Hyderabad Hitec Marathon 25" \
///   --data-urlencode "bib_number=14028"
/// ```
#[utoipa::path(
    get,
    path = "/event-result",
    params(EventResultParams),
    responses(
        (
            status = 200,
            description = "Result found. `content` contains the participant's result text.",
            body = ReaderResponse
        ),
        (
            status = 400,
            description = "Invalid request: unsupported source, or a required parameter is missing (ifinish: year, event_name, bib_number; mysamay: event_name, bib_number), or the year or event name does not exist on iFinish"
        ),
        (
            status = 404,
            description = "The BIB number was not found for the given event, or (MySamay) the event has no results"
        ),
        (
            status = 500,
            description = "Server error: the browser could not be started (CHROME_PATH is not set) or the source site did not load as expected"
        )
    ),
    tag = "Web Reader"
)]
fn _event_result_docs() {}

/// Get user's event results
///
/// Returns event results for the user.
///
/// By default, the endpoint returns results already stored in
/// `user_data.event_results`.
///
/// When `refresh=true`, the endpoint fetches a fresh result from the
/// requested external provider and returns it immediately.
///
/// Supported providers:
/// - `ifinish`
/// - `mysamay`
///
/// For `ifinish`, `year`, `event_name`, and `bib_number` are required.
///
/// For `mysamay`, `event_name` and `bib_number` are required.
///
/// The fresh provider result is returned immediately. If the event already
/// exists in the database, the result is saved in the background.
///
/// If the requested event is not present in the `events` table, the fresh
/// result is still returned successfully, but background persistence is
/// skipped.
///
/// **Stored results**
///
/// ```bash
/// curl "http://localhost:8080/users/3502a61c-4509-4347-91fa-9ef9003f1c2c/event-results"
/// ```
///
/// **Refresh iFinish result**
///
/// ```bash
/// curl "http://localhost:8080/users/3502a61c-4509-4347-91fa-9ef9003f1c2c/event-results?refresh=true&provider=ifinish&year=2026&event_name=Ananthagiri%20Hills%20Trail%20Run%202026&bib_number=32123"
/// ```
///
/// **Refresh MySamay result**
///
/// ```bash
/// curl "http://localhost:8080/users/3502a61c-4509-4347-91fa-9ef9003f1c2c/event-results?refresh=true&provider=mysamay&event_name=Wipro%20Bengaluru%20Marathon%202026&bib_number=12345"
/// ```
#[utoipa::path(
    get,
    path = "/users/{user_id}/event-results",

    params(
        (
            "user_id" = String,
            Path,
            description = "UUID of the user"
        ),
        (
            "refresh" = Option<bool>,
            Query,
            description = "Fetch a fresh result from an external provider. Defaults to false.",
            example = false
        ),
        (
            "provider" = Option<String>,
            Query,
            description = "External result provider. Required when refresh=true. Supported values: ifinish, mysamay.",
            example = "ifinish"
        ),
        (
            "year" = Option<String>,
            Query,
            description = "Event year. Required when provider=ifinish and refresh=true.",
            example = "2026"
        ),
        (
            "event_name" = Option<String>,
            Query,
            description = "Event name. Required when refresh=true. iFinish requires an exact event name; MySamay accepts a full or partial event name.",
            example = "Ananthagiri Hills Trail Run 2026"
        ),
        (
            "bib_number" = Option<String>,
            Query,
            description = "Participant BIB number. Required when refresh=true.",
            example = "32123"
        )
    ),

    responses(
        (
            status = 200,
            description = "Stored event results fetched successfully.",
            body = UserEventResultsGetResponse
        ),
        (
            status = 200,
            description = "Fresh event result returned from the external provider when refresh=true.",
            body = ReaderResponse
        ),
        (
            status = 400,
            description = "Invalid request. Possible causes: missing provider, missing provider-specific parameters, or unsupported provider."
        ),
        (
            status = 404,
            description = "User not found."
        ),
        (
            status = 500,
            description = "Failed to validate the user, fetch stored event results, or process the provider result."
        )
    ),

    tag = "User"
)]
fn _get_event_results_docs() {}

/// Save or update a manual event result
///
/// Saves a result for a completed event in the user's stored event results.
///
/// If the event already has a result, the existing result is updated instead
/// of creating a duplicate entry.
///
/// The `result` object can contain additional event result information.
/// The `timing` field is required and must use `HH:MM:SS` format.
///
/// **Example**
///
/// The `result` object can contain different event-result details.
/// `timing` is required and must use `HH:MM:SS` format.
/// Other fields such as rank, distance, price, BIB number, pace, etc.
/// can be included when available.
///
/// ```bash
/// curl -X POST "http://localhost:8080/users/3502a61c-4509-4347-91fa-9ef9003f1c2c/event-results" \
///   -H "Content-Type: application/json" \
///   -d '{
///     "eventId": "bb8dcd88-636f-4c28-b341-ba18c890dd90",
///     "result": {
///       "timing": "00:52:34",
///       "rank": 125,
///       "distance": "10 KM",
///       "price": 999,
///       "eventDay": "2026-10-12",
///       "bibNumber": "A12345",
///       "pace": "05:15 min/km"
///     }
///   }'
/// ```
///
/// **Validation**
///
/// - `eventId` must be provided and must identify an existing event.
/// - `result` must be a JSON object.
/// - `result.timing` is required and must be in `HH:MM:SS` format.
/// - `timing` minutes and seconds must be between `00` and `59`.
/// - The event must exist in the system.
/// - The event must already be completed.
/// - Additional result fields such as `rank`, `distance`, `price`,
///   `bibNumber`, `pace`, and other provider-specific details are allowed.
/// - If a result already exists for the same event, it is updated instead
///   of creating a duplicate entry.
#[utoipa::path(
    post,
    path = "/users/{user_id}/event-results",
    params(
        (
            "user_id" = String,
            Path,
            description = "UUID of the user"
        )
    ),
    request_body = ManualEventResultRequest,
    responses(
        (
            status = 201,
            description = "Event result created successfully",
            body = EventResultResponse
        ),
        (
            status = 200,
            description = "Existing event result updated successfully",
            body = EventResultResponse
        ),
        (
            status = 400,
            description = "Bad request. This can happen in the following cases:\n\n\
            1. The eventId is empty or invalid.\n\n\
            2. The result field is missing or is not a JSON object.\n\n\
            3. The result.timing field is missing.\n\n\
            4. The timing value is not in HH:MM:SS format.\n\n\
            5. The timing contains invalid values, such as minutes or seconds greater than 59.\n\n\
            6. The event does not exist in the events table.\n\n\
            7. The event exists but is not completed yet.\n\n\
            8. The event does not have a valid event date."
        ),
        (
            status = 404,
            description = "User not found"
        ),
        (
            status = 500,
            description = "Failed to save event result"
        )
    ),
    tag = "User"
)]
fn _save_event_result_docs() {}

/// List recent iFinish events with their full names
///
/// The "Recent Events" section on https://ifinish.in/eventresult shows events
/// only as thumbnail images, so their full names cannot be read as text. This
/// endpoint opens the page, clicks each thumbnail and reads the full event
/// name from the results page that opens.
///
/// **How it works**
///
/// - Every thumbnail in "Recent Events" is opened one after another (iFinish
///   currently shows 10), so no limit parameter is needed.
/// - `name` is the event name exactly as iFinish uses it. Pass it unchanged as
///   `event_name` to `/event-result` (with `source=ifinish`).
/// - `url` is the iFinish results page of that event.
/// - Thumbnails that do not open an event page are skipped, and duplicate
///   names are returned once.
/// - Only the recent events shown on the page are returned, not the full
///   archive of past years.
///
/// **This endpoint is slow:** each event takes about 3-5 seconds, so all 10
/// events need roughly 30-50 seconds. Cache the result in your app instead of
/// calling it for every request.
///
/// **Example**
///
/// ```bash
/// curl "http://localhost:8080/ifinish-events"
/// ```
///
/// ```json
/// {
///   "count": 1,
///   "events": [
///     {
///       "name": "Swachhathon 2026",
///       "url": "https://ifinish.in/eventresult/result-Swachhathon%202026"
///     }
///   ]
/// }
/// ```
#[utoipa::path(
    get,
    path = "/ifinish-events",
    responses(
        (status = 200, description = "At least one event name was resolved. `count` is the number of events returned.", body = IfinishEventsResponse),
        (status = 404, description = "No event name could be resolved from the Recent Events thumbnails"),
        (status = 500, description = "Server error: the browser could not be started (CHROME_PATH is not set) or iFinish did not load as expected")
    ),
    tag = "Web Reader"
)]
fn _ifinish_events_docs() {}

/* ============================================================
EVENTS (DATABASE)
============================================================ */

/// List events
///
/// Returns the events stored in the `events` table. The table is filled by
/// `GET /sync/ifinish` and `GET /sync/mysamay`, so this endpoint is fast and
/// does not open a browser or call the source websites.
///
/// **Filters** (all optional, can be combined)
///
/// - `provider`: `ifinish` or `mysamay`.
/// - `status`: `upcoming` (event date today or later) or `past` (before
///   today). It is worked out from `event_date` in **India time**. Events
///   without a date are not returned when `status` is used.
/// - `q`: searches inside the event name. Case-insensitive, partial match.
/// - `limit` / `offset`: paging. `limit` defaults to 200 (maximum 200),
///   `offset` defaults to 0.
/// - `include_raw`: `true` adds the full original data of each event in
///   `raw_data`. It is large, so leave it off for lists.
///
/// **Sorting:** `status=past` returns the newest events first. Everything else
/// returns the soonest events first, and events without a date last.
///
/// **Response:** `total` is the number of events matching the filters, ignoring
/// paging, so a client can work out how many pages exist. `count` is the number
/// of events in this response. An unknown `provider` or `status` returns `400`.
///
/// **Fields**
///
/// - `organizer`, `dates`, `location`, `links`, `images` and `details` are JSON
///   objects, and `prices` is a list of price options. Their keys are the same
///   for both providers (for example `location.city`, `images.banner`,
///   `dates.registration_close`). Some keys can be `null` when a provider does
///   not supply the value.
/// - `event_id` is the provider's own event ID and `event_code` its short code
///   or slug (for example `TIHM26`).
///
/// **Examples**
///
/// ```bash
/// curl "http://localhost:8080/events?provider=mysamay&status=upcoming&limit=5"
/// curl "http://localhost:8080/events?q=hyderabad"
/// ```
#[utoipa::path(
    get,
    path = "/events",
    params(
        ("provider" = Option<String>, Query, description = "Only events of this provider: `ifinish` or `mysamay`."),
        ("status" = Option<String>, Query, description = "`upcoming` (today or later) or `past` (before today), based on event_date in India time."),
        ("q" = Option<String>, Query, description = "Search in the event name. Case-insensitive, partial match."),
        ("limit" = Option<i64>, Query, description = "Page size. Default 200, maximum 200."),
        ("offset" = Option<i64>, Query, description = "Number of events to skip. Default 0."),
        ("include_raw" = Option<bool>, Query, description = "Include the full original data (`raw_data`) of each event. Default false.")
    ),
    responses(
        (status = 200, description = "Events matching the filters. `data` can be empty.", body = EventListResponse),
        (status = 400, description = "Invalid `provider` or `status` value", body = EventErrorResponse),
        (status = 500, description = "Server error: the database query failed", body = EventErrorResponse)
    ),
    tag = "Events"
)]
fn _list_events_docs() {}

/// Get one event
///
/// Returns a single event from the `events` table, including the full original
/// data from the source website in `raw_data`.
///
/// `id` is the event's own database UUID (the `id` field returned by
/// `GET /events`), not the provider's `event_id`. A malformed or unknown id
/// returns `404`.
///
/// **Example**
///
/// ```bash
/// curl "http://localhost:8080/events/0b6d1f0e-5a43-4c52-9b0a-6f4c2d9f1a11"
/// ```
#[utoipa::path(
    get,
    path = "/events/{id}",
    params(
        ("id" = String, Path, description = "Database UUID of the event (the `id` field from GET /events).")
    ),
    responses(
        (status = 200, description = "The event, including `raw_data`.", body = EventDetailResponse),
        (status = 404, description = "No event exists with this id", body = EventErrorResponse),
        (status = 500, description = "Server error: the database query failed", body = EventErrorResponse)
    ),
    tag = "Events"
)]
fn _get_event_docs() {}

/* ============================================================
EVENT SYNCHRONIZATION
============================================================ */

/// Synchronize iFinish events into the database
///
/// Fetches the upcoming events from iFinish's public API and saves them in the
/// common `events` table. No browser is used, so it normally finishes in a few
/// seconds.
///
/// **How it works**
///
/// - Calls the same event list the iFinish website uses and maps every event
///   to the common structure (name, code, dates, organizer, location, links,
///   images and description).
/// - `event_id` is iFinish's own event UUID and `event_code` its short code
///   (for example `TIHM26`).
/// - Gallery image paths are converted to full image URLs.
/// - New events are inserted. Events that already exist (same provider and
///   `event_id`) are updated, so running the sync again never creates
///   duplicates.
/// - Events dated before 2025 are skipped.
/// - `prices` is **not** filled by this sync yet, because the race categories
///   come from a separate iFinish call. Prices that were saved earlier are
///   never overwritten by an empty list.
/// - If one event cannot be saved, the sync continues with the others and
///   counts it in `failed`.
///
/// **Response:** the number of events `fetched`, `inserted`, `updated`,
/// `skipped` and `failed`, plus `warnings` for non-fatal problems.
///
/// **Example**
///
/// ```bash
/// curl -X POST "http://localhost:8080/sync/ifinish"
/// ```
///
/// ```json
/// {
///   "fetched": 12,
///   "inserted": 12,
///   "updated": 0,
///   "skipped": 0,
///   "failed": 0,
///   "warnings": []
/// }
/// ```
#[utoipa::path(
    get,
    path = "/sync/ifinish",
    responses(
        (status = 200, description = "Synchronization finished. Individual events that could not be saved are counted in `failed`.", body = SyncSummary),
        (status = 502, description = "iFinish could not be reached, returned an error, or returned data without an event list")
    ),
    tag = "Event Synchronization"
)]
fn _sync_ifinish_docs() {}

/// Synchronize MySamay events into the database
///
/// Fetches the events from MySamay's public API and saves them in the common
/// `events` table. No browser is used, so it normally finishes in a few
/// seconds.
///
/// **How it works**
///
/// - Loads the `upcoming` and `past` event lists. If one list cannot be
///   loaded, a message is added to `warnings` and the other list is still
///   saved. An event that appears in both lists is saved once.
/// - `event_id` is MySamay's `_id` (the id in `mysamay.in/event/info/<id>`) and
///   `event_code` is the event slug.
/// - MySamay sends times in UTC. They are converted to **India time** before
///   saving, so an event on 1 November is stored as 1 November.
/// - Every race category becomes one entry in `prices` with the current price,
///   the original (struck-through) price when it is higher, GST percent and
///   amount, final amount, distance, registration window and age groups.
/// - New events are inserted. Existing events (same provider and `event_id`)
///   are updated, so running the sync again never creates duplicates.
/// - Events dated before 2025 are skipped.
/// - If one event cannot be saved, the sync continues with the others and
///   counts it in `failed`.
///
/// **Response:** the number of events `fetched`, `inserted`, `updated`,
/// `skipped` and `failed`, plus `warnings`.
///
/// **Example**
///
/// ```bash
/// curl -X POST "http://localhost:8080/sync/mysamay"
/// ```
///
/// ```json
/// {
///   "fetched": 37,
///   "inserted": 37,
///   "updated": 0,
///   "skipped": 0,
///   "failed": 0,
///   "warnings": []
/// }
/// ```
#[utoipa::path(
    get,
    path = "/sync/mysamay",
    responses(
        (status = 200, description = "Synchronization finished. Check `warnings` for lists that could not be loaded and `failed` for events that could not be saved.", body = SyncSummary),
        (status = 502, description = "None of the MySamay event lists could be loaded")
    ),
    tag = "Event Synchronization"
)]
fn _sync_mysamay_docs() {}

/// Save user event information
///
/// Stores the user's event interests and registrations in `user_data`.
///
/// The request contains the user ID and a list of events with their current
/// status.
///
/// Supported statuses:
/// - `INTERESTED`
/// - `REGISTERED`
///
/// **Request Example**
///
/// ```json
/// {
///   "userId": "3502a61c-4509-4347-91fa-9ef9003f1c2c",
///   "events": [
///     {
///       "eventId": "rate-event-uuid-001",
///       "status": "REGISTERED"
///     },
///     {
///       "eventId": "rate-event-uuid-002",
///       "status": "INTERESTED"
///     }
///   ]
/// }
/// ```
#[utoipa::path(
    post,
    path = "/users/events-info",
    request_body = UserEventsInfoRequest,
    responses(
        (
            status = 200,
            description = "User events information saved successfully",
            body = UserEventsInfoResponse,
            example = json!({
                "message": "User events information saved successfully",
                "data": {
                    "userId": "3502a61c-4509-4347-91fa-9ef9003f1c2c",
                    "events": [
                        {
                            "eventId": "rate-event-uuid-001",
                            "status": "REGISTERED"
                        },
                        {
                            "eventId": "rate-event-uuid-002",
                            "status": "INTERESTED"
                        }
                    ]
                }
            })
        ),
        (
            status = 400,
            description = "Invalid request"
        ),
        (
            status = 500,
            description = "Internal server error"
        )
    ),
    tag = "User"
)]
fn _user_events_info_docs() {}

/* ============================================================
AUTHENTICATION
============================================================ */

/// Register a new user
///
/// Creates a user account. The **mobile number is the unique login ID**, so a
/// number that is already registered is rejected.
///
/// **Validation**
///
/// - `name`, `mobileNumber`, `address` and `password` must not be empty
///   (spaces only count as empty).
/// - `age` must be greater than 0.
/// - Text fields are trimmed before saving.
///
/// The password is never stored as plain text. It is hashed with Argon2 before
/// it is saved.
///
/// **Example**
///
/// ```json
/// {
///   "name": "Priyanshu Kumar",
///   "mobileNumber": "9876543210",
///   "address": "Ranchi, Jharkhand",
///   "password": "MySecret123",
///   "age": 28
/// }
/// ```
#[utoipa::path(
    post,
    path = "/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User created. Returns the new user id, name and mobile number.", body = RegisterResponse),
        (status = 400, description = "A required field is empty, or `age` is not greater than 0"),
        (status = 409, description = "This mobile number is already registered"),
        (status = 500, description = "Server error: the password could not be processed or the database failed")
    ),
    tag = "Authentication"
)]
fn _register_docs() {}

/// Log in
///
/// Checks the mobile number and password and returns the user's profile
/// details along with their associated events.
///
/// - Both `mobileNumber` and `password` are required.
/// - The same `401` message is returned for an unknown mobile number and for a
///   wrong password.
/// - The response contains **no token or session**. It only confirms that the
///   credentials are valid.
/// - Events are retrieved from the user's stored event information and matched
///   with the events table.
/// - If an event's `eventDate` is before the current date, its status is
///   returned as `COMPLETED`.
///
/// **Example**
///
/// ```json
/// {
///   "mobileNumber": "9876543210",
///   "password": "MySecret123"
/// }
/// ```
#[utoipa::path(
    post,
    path = "/login",
    request_body = LoginRequest,
    responses(
        (
            status = 200,
            description = "Credentials are valid. Returns the user profile and associated events.",
            body = LoginResponse
        ),
        (
            status = 400,
            description = "`mobileNumber` or `password` is empty"
        ),
        (
            status = 401,
            description = "Invalid mobile number or password"
        ),
        (
            status = 500,
            description = "Server error: database, password verification, user events, or event lookup failed"
        )
    ),
    tag = "Authentication"
)]
fn _login_docs() {}

/// Reset a user's password
///
/// Sets a new password for the account with the given mobile number.
///
/// - `mobileNumber` and `newPassword` are required.
/// - `newPassword` must be **at least 8 characters** long.
/// - The new password is hashed with Argon2 before it is saved, and the old
///   password stops working immediately.
/// - Returns `404` when no account exists for the mobile number.
///
/// **Example**
///
/// ```json
/// {
///   "mobileNumber": "9876543210",
///   "newPassword": "MyNewSecret456"
/// }
/// ```
#[utoipa::path(
    post,
    path = "/forgot-password",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Password was updated", body = ForgotPasswordResponse),
        (status = 400, description = "`mobileNumber` or `newPassword` is empty, or the new password is shorter than 8 characters"),
        (status = 404, description = "No user is registered with this mobile number"),
        (status = 500, description = "Server error: the password could not be processed or the database failed")
    ),
    tag = "Authentication"
)]
fn _forgot_password_docs() {}

/// Get user's events with complete event details
///
/// Returns all events associated with the user from `user_data.user_events`,
/// along with their complete details from the `events` table.
///
/// If an event date is before the current date, its status is returned as
/// `COMPLETED`. Otherwise, the user's stored status (`INTERESTED` or
/// `REGISTERED`) is returned.
///
/// **Example**
///
/// ```bash
/// curl "http://localhost:8080/users/3502a61c-4509-4347-91fa-9ef9003f1c2c/events"
/// ```
#[utoipa::path(
    get,
    path = "/users/{user_id}/events",
    summary = "Get user's events",
    tag = "User",

    params(
        (
            "user_id" = uuid::Uuid,
            Path,
            description = "Unique ID of the user"
        )
    ),

    responses(
        (
            status = 200,
            description = "User events fetched successfully",
            body = UserEventsInfoGetResponse,
            example = json!({
                "message": "User events fetched successfully",
                "data": [
                    {
                        "eventId": "12886639-5337-4ef3-85ae-d408aad425e6",
                        "status": "COMPLETED",
                        "event": {
                            "id": "e6fc10cc-b1a7-4716-921d-4cefe6729e19",
                            "provider": "ifinish",
                            "eventId": "12886639-5337-4ef3-85ae-d408aad425e6",
                            "eventCode": "TSP26-27",
                            "eventName": "Train & Shine Program 2026-27",
                            "eventType": "Running",
                            "eventDate": "2026-03-28",
                            "organizer": {
                                "companyName": "HYDERABAD RUNNERS",
                                "name": "HYDERABAD RUNNERS"
                            },
                            "dates": {
                                "eventEndDate": "2027-12-31"
                            },
                            "location": {},
                            "links": {},
                            "images": {},
                            "details": {},
                            "prices": [],
                            "rawData": {},
                            "createdAt": "2026-01-01T10:00:00Z",
                            "updatedAt": "2026-01-01T10:00:00Z"
                        }
                    }
                ]
            })
        ),
        (
            status = 500,
            description = "Failed to fetch or parse user events"
        )
    )
)]
fn _get_user_events_docs() {}

#[derive(OpenApi)]
#[openapi(
    paths(
        _read_docs,
        _map_docs,
        _resolve_docs,
        _crawl_docs,
        _search_docs,
        _event_result_docs,
        _get_event_results_docs,
        _save_event_result_docs,
        _ifinish_events_docs,
        _list_events_docs,
        _get_event_docs,
        _sync_ifinish_docs,
        _sync_mysamay_docs,
        _user_events_info_docs,
        _register_docs,
        _login_docs,
        _forgot_password_docs,
        _get_user_events_docs
    ),
    components(
        schemas(
            ReaderResponse,
            ResolveResponse,
            MapResponse,
            CrawlPage,
            CrawlResponse,
            SearchResult,
            SearchResponse,
            IfinishEvent,
            IfinishEventsResponse,
            EventResponse,
            EventListResponse,
            EventDetailResponse,
            EventErrorResponse,
            SyncSummary,
            RegisterRequest,
            RegisterResponse,
            LoginRequest,
            LoginResponse,
            ForgotPasswordRequest,
            ForgotPasswordResponse,
            ManualEventResultRequest,
            UserEventResultsGetResponse,
            EventResultResponse,
            EventResultResponseData,
            UserEventsInfoRequest,
            UserEventsInfoResponse,
            UserEventsInfoGetResponse
        )
    ),
    tags(
        (name = "Web Reader", description = "Read, map, crawl and search web pages, and fetch event results from iFinish and MySamay. All endpoints render pages in a headless Chrome browser, so responses can take several seconds to a few minutes."),
        (name = "Events", description = "Read the events stored in the database (iFinish and MySamay). Fast: no browser and no call to the source websites."),
        (name = "Event Synchronization", description = "Copy events from the iFinish and MySamay public APIs into the events table. Safe to run repeatedly: existing events are updated, not duplicated."),
        (name = "User", description = "Manage user-specific event information."),
        (name = "Authentication", description = "Register users, log in and reset passwords. Mobile number is the unique user ID and passwords are stored hashed (Argon2).")
    )
)]
pub struct ApiDoc;

pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", ApiDoc::openapi())
}
