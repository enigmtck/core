use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct SummaryUpdate {
    pub content: String,
    pub markdown: String,
}
