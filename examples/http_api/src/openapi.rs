use crate::todos::{controllers as tc, viewmodels as tvm};
use http_components::viewmodels::HTTPError;
use utoipa::{
    openapi::{
        self,
        security::{Http, HttpAuthScheme, SecurityScheme},
    },
    Modify, OpenApi,
};

#[derive(OpenApi)]
#[openapi(
  paths(
    tc::post, tc::get, tc::list, tc::delete,
  ),
  components(
    schemas(
      HTTPError,
      tvm::ToDoRequest, tvm::ToDoResponse,
    )
  ),
  tags(
    (name = "examples", description = "Examples endpoints."),
  ),
  modifiers(&SecurityAddon),
  info(
    title = "Hedro HTTP API",
    version = "v0.0.1",
    description = "Hedro HTTP API's built in rust"
  ),
)]
#[cfg_attr(debug_assertions, openapi(
  servers(
    (url = "http://localhost:4444", description = "Local server"),
  ),
))]
#[cfg_attr(not(debug_assertions), openapi(
  servers(
    (url = "https://stg-api.com.br", description = "Staging server"),
  ),
))]
pub struct ApiDoc;

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        )
    }
}
