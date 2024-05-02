mod openapi;
mod todos;

use actix_web::web::{Data, ServiceConfig};
use auth::{auth0::Auth0JwtManager, manager::JwtManager};
use configs::{Configs, Empty};
use configs_builder::ConfigBuilder;
use http_components::CustomServiceConfigure;
use http_server::server::HTTPServer;
use openapi::ApiDoc;
use std::{error::Error, sync::Arc};
use utoipa::OpenApi;

use todos::routes as todos_routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cfgs = default_setup().await?;

    let doc = ApiDoc::openapi();

    let identity_configs = cfgs.identity.clone();

    HTTPServer::new(&cfgs.app)
        //if you need to share structs to all the controllers, use something similar
        .custom_configure(CustomServiceConfigure::new(
            move |cfg: &mut ServiceConfig| {
                let auth0_manager: Arc<dyn JwtManager> = Auth0JwtManager::new(&identity_configs);
                cfg.app_data(Data::<dyn JwtManager>::from(auth0_manager));
            },
        ))
        .custom_configure(todos_routes::basic_routes())
        .openapi(&doc)
        .start()
        .await?;

    Ok(())
}

async fn default_setup<'cfg>() -> Result<Configs<Empty>, Box<dyn Error>> {
    let cfg = ConfigBuilder::new()
        .trace()
        .metric()
        .identity_server()
        .build::<Empty>()
        .await?;

    traces::provider::init(&cfg)?;
    metrics::provider::init(&cfg)?;

    Ok(cfg)
}
