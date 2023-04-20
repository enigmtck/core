use rocket::get;
use rocket::http::{RawStr, Status};
use rocket::request::FromParam;
use rocket::serde::json::Json;

use crate::api::instance::InstanceInformation;

pub struct ApiVersion<'r> {
    _version: &'r str,
}

impl<'r> FromParam<'r> for ApiVersion<'r> {
    type Error = &'r RawStr;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        if param == "v1" || param == "v2" {
            Ok(ApiVersion { _version: param })
        } else {
            Err(param.into())
        }
    }
}

#[get("/.well-known/host-meta", format = "application/xrd+xml")]
pub async fn host_meta() -> Result<String, Status> {
    Ok(r#"<?xml version="1.0" encoding="UTF-8"?><XRD xmlns="http://docs.oasis-open.org/ns/xri/xrd-1.0"><Link rel="lrdd" template="https://enigmatick.jdt.dev/.well-known/webfinger?resource={uri}" type="application/json" /></XRD>"#.to_string())
}

#[get("/api/<_version>/instance")]
pub async fn instance_information(
    _version: ApiVersion<'_>,
) -> Result<Json<InstanceInformation>, Status> {
    Ok(Json(InstanceInformation::default()))
}
