use crate::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ContentType {
    Apk,
    AppData,
    Calendar,
    Contact,
    Document,
    Image,
    Message,
    Video,
    Other(String),
}
