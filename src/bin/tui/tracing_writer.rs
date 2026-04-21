use std::sync::{Arc, Mutex};
use std::{collections::VecDeque, time::SystemTime};
use super::constants::MAX_LOGS;

fn format_system_time(time: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(time)
        .format("%H:%M:%S.%3f")
        .to_string()
}

#[derive(Clone)]
pub struct TracingWriter {
    logs: Arc<Mutex<VecDeque<String>>>,
}

impl TracingWriter {
    #[must_use]
    pub fn new(logs: Arc<Mutex<VecDeque<String>>>) -> Self {
        Self { logs }
    }
}

impl std::io::Write for TracingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(s) = std::str::from_utf8(buf) {
            let cleaned = p2p_app::logging::strip_ansi_codes(s);
            let trimmed = cleaned.trim();
            if !trimmed.is_empty() {
                let ts = format_system_time(SystemTime::now());
                let formatted = format!("[{}] {}", ts, trimmed);
                if let Ok(mut l) = self.logs.lock() {
                    l.push_back(formatted);
                    if l.len() > MAX_LOGS {
                        l.pop_front();
                    }
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
