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
    Settings,
    Other(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum BackupType {
    KeyValue,
    Full,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FileMetadata {
    pub file_name: String,
    pub relative_path: String,
    pub created: i64,
    pub last_modified: i64,
    pub permissions: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ManifestEntry {
    pub package_name: String,
    /// 39-byte action hash of the StorageEntry for this package's backup
    pub storage_entry_hash: Vec<u8>,
    pub backup_type: BackupType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ContentMetadata {
    Apk {
        package_name: String,
        version_code: i64,
        version_name: String,
        app_name: String,
        is_split: bool,
        split_id: Option<String>,
    },
    AppData {
        package_name: String,
        backup_type: BackupType,
    },
    Audio(FileMetadata),
    Document(FileMetadata),
    Image(FileMetadata),
    Video(FileMetadata),
    OtherFile(FileMetadata),
    Calendar,
    Contact,
    Message,
    Settings,
    /// Encrypted index mapping package names to their StorageEntry action hashes.
    /// One manifest per backup generation, stored as a StorageEntry with no file data.
    Manifest {
        generation: String,
        entries: Vec<ManifestEntry>,
    },
}
