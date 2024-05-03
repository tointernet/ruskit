use crate::todos::controllers;
use actix_web::web::{self, ServiceConfig};
use http_components::CustomServiceConfigure;

pub fn basic_routes() -> CustomServiceConfigure {
    CustomServiceConfigure::new(|cfg: &mut ServiceConfig| {
        cfg.service(
            web::scope("/v1/todos")
                // If you would like to add authentication middleware for all this routes bellow, just use the middleware as follow:
                //
                // .wrap(http_components::middlewares::authentication::AuthenticationMiddleware)
                .service(controllers::post)
                .service(controllers::list)
                .service(controllers::get)
                .service(controllers::delete),
        );
    })
}
