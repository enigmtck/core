use rocket::get;
use rocket::http::{RawStr, Status};
use rocket::request::FromParam;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RegistrationInformation {
    pub enabled: bool,
    pub approval_required: bool,
    pub message: Option<String>,
}

impl Default for RegistrationInformation {
    fn default() -> Self {
        RegistrationInformation {
            enabled: *crate::REGISTRATION_ENABLED,
            approval_required: *crate::REGISTRATION_APPROVAL_REQUIRED,
            message: Option::from((*crate::REGISTRATION_MESSAGE).to_string()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ContactInformation {
    pub contact: String,
}

impl Default for ContactInformation {
    fn default() -> Self {
        ContactInformation {
            contact: (*crate::INSTANCE_CONTACT).to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct InstanceInformation {
    pub domain: String,
    pub url: String,
    pub title: String,
    pub version: String,
    pub source_url: String,
    pub description: String,
    pub registrations: RegistrationInformation,
    pub contact: ContactInformation,
}

impl Default for InstanceInformation {
    fn default() -> Self {
        InstanceInformation {
            domain: (*crate::SERVER_NAME).to_string(),
            url: (*crate::SERVER_URL).to_string(),
            title: (*crate::INSTANCE_TITLE).to_string(),
            version: (*crate::INSTANCE_VERSION).to_string(),
            source_url: (*crate::INSTANCE_SOURCE_URL).to_string(),
            description: (*crate::INSTANCE_DESCRIPTION).to_string(),
            registrations: RegistrationInformation::default(),
            contact: ContactInformation::default(),
        }
    }
}

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

#[get("/.well-known/host-meta")]
pub async fn host_meta() -> Result<String, Status> {
    Ok(r#"<?xml version="1.0" encoding="UTF-8"?><XRD xmlns="http://docs.oasis-open.org/ns/xri/xrd-1.0"><Link rel="lrdd" template="https://enigmatick.jdt.dev/.well-known/webfinger?resource={uri}" type="application/json" /></XRD>"#.to_string())
}

#[get("/api/<_version>/instance")]
pub async fn instance_information(
    _version: ApiVersion<'_>,
) -> Result<Json<InstanceInformation>, Status> {
    Ok(Json(InstanceInformation::default()))
}

// #[get("/test-get/<username>?<url>")]
// pub async fn test_get(conn: Db, username: String, url: String) -> Result<String, Status> {
//     if let Ok(response) = maybe_signed_get(
//         get_profile_by_username(&conn, username).await,
//         urlencoding::decode(&url),
//         true,
//     )
//     .await
//     {
//         response.text().await
//     } else {
//         Err(Status::NotFound)
//     }
// }
