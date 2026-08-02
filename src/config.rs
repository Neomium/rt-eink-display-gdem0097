/// Runtime configuration for display timing and refresh behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Maximum time, in microseconds, to wait for the BUSY pin to become idle.
    pub busy_timeout_us: u32,

    /// Delay, in microseconds, between consecutive BUSY-pin reads.
    pub busy_poll_interval_us: u32,

    /// Whether full and partial refresh methods wait until the display is idle.
    ///
    /// Reset and initialization operations always wait because subsequent
    /// commands require the controller to be ready.
    pub wait_after_refresh: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            busy_timeout_us: 15_000_000,
            busy_poll_interval_us: 100,
            wait_after_refresh: true,
        }
    }
}
