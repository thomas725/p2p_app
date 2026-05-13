#[test]
fn test_tracing_filter_is_targets() {
    let filter = p2p_app::tracing_filter();
    // Just verify it returns without panicking and has the right type
    let _ = filter.clone();
}
