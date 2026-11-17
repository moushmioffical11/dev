use actix_cors::Cors;
use actix_web::web;

use crate::handlers::auth::forgot_password::forgot_password;
use crate::handlers::auth::login::login;
use crate::handlers::auth::register::register;
use crate::{docs, handlers, providers};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(register)
        .service(login)
        .service(forgot_password)
        .service(handlers::reader::read_page)
        .service(handlers::reader::map_page)
        .service(handlers::resolve::resolve_page)
        .service(handlers::crawler::crawl_page)
        .service(handlers::search::search_page)
        .service(handlers::event_result::event_result)
        .service(handlers::user::event_results::get_event_results)
        .service(handlers::user::event_results::save_event_result)
        .service(providers::ifinish::events::ifinish_events)
        .service(providers::ifinish::sync::sync_ifinish)
        .service(providers::mysamay::sync::sync_mysamay)
        .service(handlers::event::list_events)
        .service(handlers::event::get_event)
        .service(handlers::user::events_info::events_info)
        .service(handlers::user::get_events_info::get_user_events)
        .service(docs::swagger::swagger_ui());
}

pub fn cors() -> Cors {
    Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header()
        .max_age(3600)
}
