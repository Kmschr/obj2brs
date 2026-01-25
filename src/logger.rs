use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Logger {
    messages: Arc<Mutex<Vec<String>>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn log(&self, message: String) {
        if let Ok(mut messages) = self.messages.lock() {
            messages.push(message);
        }
    }

    pub fn get_messages(&self) -> Vec<String> {
        self.messages.lock().ok()
            .map(|m| m.clone())
            .unwrap_or_default()
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}
