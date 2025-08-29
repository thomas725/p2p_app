// @generated automatically by Diesel CLI.

diesel::table! {
    identities (id) {
        id -> Integer,
        created_at -> Timestamp,
        key -> Binary,
    }
}
