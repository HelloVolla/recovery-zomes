use crate::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ContentType {
    Apk,
    AppData,
    Audio,
    Calendar,
    Contact,
    Document,
    Image,
    Message,
    Video,
    Other(String),
}
