use serde::{Deserialize, Serialize};

/// An opaque encrypted blob wrapping a serialized `ContentMetadata` value.
///
/// Encryption and decryption are performed entirely on the Kotlin side using
/// the `MetadataEncryption` helper in `libraries/crypto`, which drives
/// AES-256-GCM via the Android Keystore.  Rust only stores and transmits
/// these bytes; it never holds or sees the key material.
///
/// # Wire format (produced by Kotlin / `AesEncryption`)
/// ```text
/// [ iv: 12 bytes ][ ciphertext + GCM auth tag (16 bytes) ]
/// ```
///
/// The `nonce` field carries the 12-byte AES-GCM IV; `ciphertext` carries
/// everything after it (encrypted msgpack payload + 16-byte tag).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EncryptedMetadata {
    /// AES-GCM IV — 12 bytes, randomly generated per encryption on the Kotlin side.
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
    /// AES-256-GCM ciphertext including the 16-byte authentication tag.
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
}
