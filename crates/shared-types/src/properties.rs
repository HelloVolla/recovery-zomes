use crate::*;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, SerializedBytes, Clone)]
pub struct Properties {
    #[serde(with = "serde_bytes")]
    pub progenitor_pub_key: Vec<u8>,
}
