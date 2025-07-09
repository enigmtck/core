use crate::routes::instance::InstanceInformation;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
};

pub async fn host_meta() -> impl IntoResponse {
    r#"<?xml version="1.0" encoding="UTF-8"?><XRD xmlns="http://docs.oasis-open.org/ns/xri/xrd-1.0"><Link rel="lrdd" template="https://enigmatick.jdt.dev/.well-known/webfinger?resource={uri}" type="application/json" /></XRD>"#.to_string()
}

pub async fn instance_information(
    Path(version): Path<String>,
) -> Result<Json<InstanceInformation>, StatusCode> {
    if version == "v1" || version == "v2" {
        Ok(Json(InstanceInformation::default()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
