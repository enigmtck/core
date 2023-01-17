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
