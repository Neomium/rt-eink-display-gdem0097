use crate::Config;

#[test]
fn config_default_uses_recommended_timing_and_waits_after_refresh() {
    let config = Config::default();

    assert_eq!(config.busy_timeout_us, 15_000_000);
    assert_eq!(config.busy_poll_interval_us, 10_000);
    assert!(config.wait_after_refresh);
}
