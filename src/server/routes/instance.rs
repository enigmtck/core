use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
};
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
            message: Some((*crate::REGISTRATION_MESSAGE).to_string()),
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
            url: format!("https://{}", *crate::SERVER_NAME),
            title: (*crate::INSTANCE_TITLE).to_string(),
            version: (*crate::INSTANCE_VERSION).to_string(),
            source_url: (*crate::INSTANCE_SOURCE_URL).to_string(),
            description: (*crate::INSTANCE_DESCRIPTION).to_string(),
            registrations: RegistrationInformation::default(),
            contact: ContactInformation::default(),
        }
    }
}

pub struct _ApiVersion<'r> {
    _version: &'r str,
}

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

/// Redirects to the install.sh script (configurable via INSTALL_SCRIPT_URL env var)
pub async fn install_script() -> impl IntoResponse {
    axum::response::Redirect::temporary(&*crate::INSTALL_SCRIPT_URL)
}
