//! Tests for database selection logic

use std::fs;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn safe_remove_env(key: &str) {
    unsafe { std::env::remove_var(key) }
}

#[test]
fn test_two_instances_get_different_databases_parallel() {
    let temp_dir = TempDir::new().unwrap();
    let cwd = temp_dir.path().to_path_buf();

    fs::write(cwd.join("sqlite.db"), "").unwrap();
    fs::write(cwd.join("sqlite_1.db"), "").unwrap();

    let errors = Arc::new(std::sync::Mutex::new(Vec::new()));
    let barrier = Arc::new(std::sync::Barrier::new(2));

    let cwd_clone = cwd.clone();
    let errors_clone = errors.clone();
    let barrier_clone = barrier.clone();
    thread::spawn(move || {
        barrier_clone.wait();
        std::env::set_current_dir(&cwd_clone).ok();
        safe_remove_env("DATABASE_URL");
        let url = p2p_app::db::get_database_url();
        if let Ok(mut e) = errors_clone.lock() {
            e.push(format!("A: {}", url));
        }
    });

    let cwd_clone = cwd.clone();
    let errors_clone = errors.clone();
    let barrier_clone = barrier.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        barrier_clone.wait();
        std::env::set_current_dir(&cwd_clone).ok();
        safe_remove_env("DATABASE_URL");
        let url = p2p_app::db::get_database_url();
        if let Ok(mut e) = errors_clone.lock() {
            e.push(format!("B: {}", url));
        }
    });

    thread::sleep(Duration::from_millis(200));

    let urls = errors.lock().unwrap();
    println!("URLs: {:?}", urls);

    assert_ne!(urls[0], urls[1], "Both picked same database!");
}
