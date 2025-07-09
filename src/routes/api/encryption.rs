use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct OtkUpdateParams {
    pub keys: HashMap<String, String>,
    pub account: String,
    pub mutation_of: String,
    pub account_hash: String,
}
