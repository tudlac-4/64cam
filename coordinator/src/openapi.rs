use common::models::{
    camera::{Camera, CreateCamera, UpdateCamera},
    node::{CreateNode, NodeResponse, NodeStatus, UpdateNodeStatus},
    retention::{CreateRetentionPolicy, RetentionPolicy, UpdateRetentionPolicy},
    user::{CreateUser, UpdateUser, UserResponse},
};
use utoipa::{
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
    Modify, OpenApi,
};

use crate::routes::{
    auth::{LoginRequest, RefreshRequest, TokenResponse},
    nodes::RegisterNodeResponse,
};

struct BearerAuth;
impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_token",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(title = "64cam Coordinator API", version = "0.1.0"),
    paths(
        crate::routes::auth::login,
        crate::routes::auth::refresh,
        crate::routes::auth::logout,
        crate::routes::users::list_users,
        crate::routes::users::create_user,
        crate::routes::users::get_user,
        crate::routes::users::update_user,
        crate::routes::users::delete_user,
        crate::routes::nodes::list_nodes,
        crate::routes::nodes::create_node,
        crate::routes::nodes::get_node,
        crate::routes::nodes::update_node_status,
        crate::routes::nodes::delete_node,
        crate::routes::cameras::list_cameras,
        crate::routes::cameras::create_camera,
        crate::routes::cameras::get_camera,
        crate::routes::cameras::update_camera,
        crate::routes::cameras::delete_camera,
        crate::routes::retention::list_retention_policies,
        crate::routes::retention::create_retention_policy,
        crate::routes::retention::get_retention_policy,
        crate::routes::retention::update_retention_policy,
        crate::routes::retention::delete_retention_policy,
    ),
    components(schemas(
        LoginRequest, RefreshRequest, TokenResponse,
        UserResponse, CreateUser, UpdateUser,
        NodeResponse, NodeStatus, CreateNode, UpdateNodeStatus, RegisterNodeResponse,
        Camera, CreateCamera, UpdateCamera,
        RetentionPolicy, CreateRetentionPolicy, UpdateRetentionPolicy,
    )),
    modifiers(&BearerAuth),
)]
pub struct ApiDoc;
