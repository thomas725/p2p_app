#!/usr/bin/env python3
"""
Post-processor for diesel_generate.sh.

Injects #![allow(missing_docs)] and doc comments into the auto-generated
Diesel model files so they compile cleanly under -W missing-docs without
requiring manual edits after each regeneration.

Usage: python3 scripts/add_generated_docs.py
"""

import re

# ---------------------------------------------------------------------------
# Doc strings keyed by field name (shared across all tables)
# ---------------------------------------------------------------------------
FIELD_DOCS: dict[str, str] = {
    # common
    "id":                     "Auto-incremented primary key",
    "created_at":             "Timestamp when this record was created",
    "recorded_at":            "Timestamp when this record was recorded",
    # identities
    "key":                    "Serialised libp2p identity keypair (binary)",
    "last_tcp_port":          "Last TCP port this node listened on",
    "last_quic_port":         "Last QUIC port this node listened on",
    "self_nickname":          "Nickname this node advertises to peers",
    # messages
    "content":                "Message text",
    "peer_id":                "Peer ID of the remote party (None = local/broadcast)",
    "topic":                  "Gossipsub topic this message was published on",
    "sent":                   "Whether this message has been sent (0 = no, 1 = yes)",
    "is_direct":              "Whether this is a direct message (0 = broadcast, 1 = DM)",
    "target_peer":            "Recipient peer ID for direct messages",
    "msg_id":                 "Application-level unique message identifier",
    "sent_at":                "Unix timestamp (seconds) when the message was sent",
    "sender_nickname":        "Nickname of the sender at the time of sending",
    # message_receipts
    "kind":                   "Receipt kind (0 = delivered, 1 = read)",
    "confirmed_at":           "Unix timestamp (seconds) when the receipt was confirmed",
    # peers
    "addresses":              "Comma-separated list of known multiaddrs for this peer",
    "first_seen":             "Timestamp when this peer was first observed",
    "last_seen":              "Timestamp when this peer was most recently observed",
    "peer_local_nickname":    "Nickname we have assigned to this peer locally",
    "self_nickname_for_peer": "Nickname we advertise specifically to this peer",
    "received_nickname":      "Nickname this peer has advertised to us",
    # peer_sessions
    "concurrent_peers":       "Number of peers connected at the time of this snapshot",
}

STRUCT_DOCS: dict[str, str] = {
    "Identity":          "Queryable struct for the `identities` table",
    "MessageReceipt":    "Queryable struct for the `message_receipts` table",
    "Message":           "Queryable struct for the `messages` table",
    "PeerSession":       "Queryable struct for the `peer_sessions` table",
    "Peer":              "Queryable struct for the `peers` table",
    "NewIdentity":       "Insertable struct for the `identities` table",
    "NewMessageReceipt": "Insertable struct for the `message_receipts` table",
    "NewMessage":        "Insertable struct for the `messages` table",
    "NewPeerSession":    "Insertable struct for the `peer_sessions` table",
    "NewPeer":           "Insertable struct for the `peers` table",
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def prepend_module_header(src: str, module_doc: str) -> str:
    """Prepend //! module doc and #![allow(missing_docs)] to source (idempotent)."""
    if "#![allow(missing_docs)]" in src:
        return src
    header = f"//! {module_doc}\n\n#![allow(missing_docs)]\n\n"
    return header + src


def insert_struct_docs(src: str) -> str:
    """Add /// doc comment before each #[derive(...)] / #[diesel(...)] / pub struct block (idempotent)."""
    def replacer(m: re.Match) -> str:
        name = m.group(2)
        # Don't prepend if a doc comment already immediately precedes this block
        start = m.start()
        preceding = src[:start].rstrip()
        if preceding.endswith("///") or re.search(r'///[^\n]*\n\s*$', preceding):
            return m.group(0)
        doc = STRUCT_DOCS.get(name, f"Generated model struct for `{name}`")
        return f"/// {doc}\n{m.group(1)}"

    return re.sub(
        r'(#\[derive\([^\]]+\)\]\n#\[diesel[^\n]+\]\npub struct (\w+))',
        replacer,
        src,
    )


def insert_field_docs(src: str) -> str:
    """Add /// doc comment before each undocumented public struct field (idempotent)."""
    lines = src.splitlines()
    out = []
    for i, line in enumerate(lines):
        m = re.match(r'^( +)pub (\w+)(: [^\n]+)', line)
        if m:
            prev = lines[i - 1].strip() if i > 0 else ""
            if not prev.startswith("///"):
                indent = m.group(1)
                field  = m.group(2)
                rest   = m.group(3)
                doc = FIELD_DOCS.get(field, field.replace("_", " ").capitalize())
                out.append(f"{indent}/// {doc}")
        out.append(line)
    return "\n".join(out) + "\n"


def process_model_file(path: str, module_doc: str) -> None:
    with open(path) as f:
        src = f.read()

    src = prepend_module_header(src, module_doc)
    src = insert_struct_docs(src)
    src = insert_field_docs(src)

    with open(path, "w") as f:
        f.write(src)

    print(f"  Documented {path}")


def process_schema(path: str) -> None:
    """Insert #![allow(missing_docs)] into schema.rs after the @generated comment."""
    with open(path) as f:
        src = f.read()

    marker = "// @generated automatically by Diesel CLI."
    if "#![allow(missing_docs)]" not in src:
        src = src.replace(marker, marker + "\n\n#![allow(missing_docs)]")

    with open(path, "w") as f:
        f.write(src)

    print(f"  Documented {path}")


def process_columns(path: str) -> None:
    """Prepend module doc + allow to columns.rs, and doc the SCHEMA_ENTRIES constant."""
    with open(path) as f:
        src = f.read()

    if "#![allow(missing_docs)]" not in src:
        header = "//! Auto-generated column constants from the Diesel schema.\n\n#![allow(missing_docs)]\n\n"
        src = header + src

    # Add doc comment to SCHEMA_ENTRIES if missing
    if "#[doc(hidden)]" not in src:
        src = src.replace(
            "pub const SCHEMA_ENTRIES",
            "#[doc(hidden)]\npub const SCHEMA_ENTRIES",
        )

    with open(path, "w") as f:
        f.write(src)

    print(f"  Documented {path}")



# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    process_schema("src/generated/schema.rs")
    process_columns("src/generated/columns.rs")
    process_model_file(
        "src/generated/models_queryable.rs",
        "Auto-generated queryable model structs from the Diesel schema.",
    )
    process_model_file(
        "src/generated/models_insertable.rs",
        "Auto-generated insertable (New*) model structs from the Diesel schema.",
    )
    print("Done.")
