//! Tests for db.rs module - database URL and identity functions

#[test]
fn test_get_database_url_env_set() {
    unsafe { std::env::set_var("DATABASE_URL", "/tmp/test.db") };
    let url = p2p_app::db::get_database_url();
    unsafe { std::env::remove_var("DATABASE_URL") };
    assert_eq!(url, "/tmp/test.db");
}

#[test]
fn test_release_db_lock() {
    p2p_app::db::release_db_lock();
}
