use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub socket_timeout: Duration,
    pub max_retries: u32,
    pub max_connections: usize,
    pub idle_timeout: Duration,
    pub reap_interval: Duration,
    pub verbose: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            socket_timeout: Duration::from_secs(5),
            max_retries: 3,
            max_connections: 30,
            idle_timeout: Duration::from_secs(1800),
            reap_interval: Duration::from_secs(300),
            verbose: false,
        }
    }
}
