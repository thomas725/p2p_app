//! camelCase JSON views of the share types for frontend transports.
//!
//! The domain types in [`crate::shares`] keep their `snake_case` serde shape: it
//! is the `/p2p-chat/shares/1.0.0` wire format and the stored `manifest_json`
//! column. Browsers and the mobile event bus, however, use the camelCase shapes
//! generated for Dart. Converting at the boundary keeps both contracts stable
//! instead of silently defaulting every field when the names disagree.

use crate::shares::{ShareEntry, ShareKind, ShareManifest, ShareOffer};
use serde::{Deserialize, Serialize};

/// Browser/event view of one manifest entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareEntryJson {
    /// Relative path inside the share.
    pub path: String,
    /// Content address in the shared blob store.
    pub file_id: String,
    /// Byte length of the file.
    pub size: u64,
    /// Best-effort MIME type hint.
    #[serde(default)]
    pub mime: Option<String>,
}

impl From<&ShareEntry> for ShareEntryJson {
    fn from(entry: &ShareEntry) -> Self {
        Self {
            path: entry.path.clone(),
            file_id: entry.file_id.clone(),
            size: entry.size,
            mime: entry.mime.clone(),
        }
    }
}

impl From<ShareEntryJson> for ShareEntry {
    fn from(entry: ShareEntryJson) -> Self {
        Self {
            path: entry.path,
            file_id: entry.file_id,
            size: entry.size,
            mime: entry.mime,
        }
    }
}

/// Browser/event view of a versioned snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareManifestJson {
    /// Stable share id.
    pub share_id: String,
    /// Human-readable share name.
    pub name: String,
    /// Snapshot revision.
    pub revision: u64,
    /// Files in the snapshot.
    pub entries: Vec<ShareEntryJson>,
}

impl From<&ShareManifest> for ShareManifestJson {
    fn from(manifest: &ShareManifest) -> Self {
        Self {
            share_id: manifest.share_id.clone(),
            name: manifest.name.clone(),
            revision: manifest.revision,
            entries: manifest.entries.iter().map(Into::into).collect(),
        }
    }
}

impl From<ShareManifestJson> for ShareManifest {
    fn from(manifest: ShareManifestJson) -> Self {
        Self {
            share_id: manifest.share_id,
            name: manifest.name,
            revision: manifest.revision,
            entries: manifest.entries.into_iter().map(Into::into).collect(),
        }
    }
}

/// Browser/event view of a share advertisement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareOfferJson {
    /// Stable share id.
    pub share_id: String,
    /// Human-readable share name.
    pub name: String,
    /// `private` or `public`.
    pub kind: ShareKind,
    /// Advertised revision.
    pub revision: u64,
    /// Whether a public share accepts writes.
    pub public_write: bool,
    /// Owner id when known (never verified on the receiving device).
    #[serde(default)]
    pub owner_user_id: Option<String>,
}

impl From<&ShareOffer> for ShareOfferJson {
    fn from(offer: &ShareOffer) -> Self {
        Self {
            share_id: offer.share_id.clone(),
            name: offer.name.clone(),
            kind: offer.kind,
            revision: offer.revision,
            public_write: offer.public_write,
            owner_user_id: offer.owner_user_id.clone(),
        }
    }
}

impl From<ShareOfferJson> for ShareOffer {
    fn from(offer: ShareOfferJson) -> Self {
        Self {
            share_id: offer.share_id,
            name: offer.name,
            kind: offer.kind,
            revision: offer.revision,
            public_write: offer.public_write,
            owner_user_id: offer.owner_user_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_manifest() -> ShareManifest {
        ShareManifest {
            share_id: "share-1".to_string(),
            name: "Docs".to_string(),
            revision: 4,
            entries: vec![ShareEntry {
                path: "docs/readme.md".to_string(),
                file_id: "a".repeat(64),
                size: 12,
                mime: Some("text/markdown".to_string()),
            }],
        }
    }

    #[test]
    fn manifest_round_trip_uses_camel_case_at_the_frontend_boundary() {
        let raw = serde_json::to_value(ShareManifestJson::from(&sample_manifest()))
            .expect("encode manifest");
        assert_eq!(raw["shareId"], "share-1");
        assert_eq!(raw["revision"], 4);
        assert_eq!(raw["entries"][0]["fileId"], "a".repeat(64));
        assert_eq!(raw["entries"][0]["mime"], "text/markdown");

        let parsed: ShareManifestJson = serde_json::from_value(raw).expect("decode manifest");
        assert_eq!(ShareManifest::from(parsed), sample_manifest());
    }

    #[test]
    fn offer_round_trip_keeps_real_values() {
        let offer = ShareOffer {
            share_id: "share-2".to_string(),
            name: "Photos".to_string(),
            kind: ShareKind::Public,
            revision: 2,
            public_write: true,
            owner_user_id: Some("owner".to_string()),
        };
        let raw = serde_json::to_value(ShareOfferJson::from(&offer)).expect("encode offer");
        assert_eq!(
            raw,
            json!({
                "shareId": "share-2",
                "name": "Photos",
                "kind": "public",
                "revision": 2,
                "publicWrite": true,
                "ownerUserId": "owner",
            })
        );
        let parsed: ShareOfferJson = serde_json::from_value(raw).expect("decode offer");
        assert_eq!(ShareOffer::from(parsed), offer);
    }

    #[test]
    fn missing_required_fields_are_an_error_not_a_default() {
        // This is the bug the DTO exists to prevent: `#[serde(default)]` on the
        // domain types would silently turn a name mismatch into empty shares.
        assert!(serde_json::from_value::<ShareOfferJson>(json!({"name": "Photos"})).is_err());
        assert!(serde_json::from_value::<ShareManifestJson>(json!({"shareId": "s"})).is_err());
    }
}
