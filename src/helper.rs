pub fn is_local(ap_id: String) -> bool {
    let pattern = format!(r#"\w+?://{}/(.+)"#, *crate::SERVER_NAME);

    if let Ok(re) = regex::Regex::new(&pattern) {
        if re.is_match(&ap_id) {
            log::debug!("looks local");
            true
        } else {
            log::debug!("looks remote");
            false
        }
    } else {
        false
    }
}

pub fn get_local_username_from_ap_id(ap_id: String) -> Option<String> {
    let pattern = format!(r#"\w+?://{}/user/(.+)"#, *crate::SERVER_NAME);

    if let Ok(re) = regex::Regex::new(&pattern) {
        if let Some(ap_id_match) = re.captures(&ap_id) {
            log::debug!("username_match: {:#?}", ap_id_match);
            Option::from(ap_id_match.get(1).unwrap().as_str().to_string())
        } else {
            Option::None
        }
    } else {
        Option::None
    }
}
