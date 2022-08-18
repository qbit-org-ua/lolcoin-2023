use actix_web::web;

mod resources;
mod schemas;

pub(crate) fn register_services(app: &mut web::ServiceConfig) {
    app.service(web::resource("/send-transfer").route(web::post().to(resources::send_transfer)));
}
