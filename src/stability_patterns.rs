use chrono::{DateTime, Duration, Local};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Stability patterns:
// Throttling
// Circuit Breaker
// Timeout

struct CircuitBreaker {
    error_count: u16,
    max_trips: u16,
    last_error: DateTime<Local>,
    created_at: DateTime<Local>,
}

struct Throttler {
    limit: u16,
    current: u16,
    last_request: DateTime<Local>,
    time_window: Duration,
}

pub struct StabilityPatterns {
    circuitbreakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    throttlers: Arc<Mutex<HashMap<String, Throttler>>>,
}

impl CircuitBreaker {
    fn new(max_trips: u16) -> Self {
        let s = Self {
            max_trips,
            error_count: 0,
            last_error: Local::now(),
            created_at: Local::now(),
        };
        return s;
    }

    fn trip(&mut self) -> bool {
        self.error_count += 1;
        self.last_error = Local::now();
        if self.error_count > self.max_trips {
            return false;
        }
        return true;
    }

    fn reset(&mut self) {
        self.error_count = 0;
    }
}

impl Throttler {
    fn new(limit: u16, time_window: Duration) -> Self {
        Self {
            limit,
            current: 0,
            last_request: Local::now(),
            time_window,
        }
    }
    fn check(&mut self) -> bool {
        if (self.last_request - Local::now()) < self.time_window {
            self.current += 1;
            if self.current > self.limit {
                return false;
            }
        } else {
            self.current = 1;
        }
        return true;
    }
}

impl StabilityPatterns {
    pub fn new() -> Self {
        let s = Self {
            circuitbreakers: Arc::new(Mutex::new(HashMap::new())),
            throttlers: Arc::new(Mutex::new(HashMap::new())),
        };
        return s;
    }

    pub fn circuitbreaker(&mut self, name: String, max_trips: u16) {
        let cb = CircuitBreaker::new(max_trips);
        self.circuitbreakers.lock().unwrap().insert(name, cb);
    }

    pub fn trip(&mut self, name: String) -> bool {
        self.circuitbreakers
            .lock()
            .unwrap()
            .get_mut(&name)
            .unwrap()
            .trip()
    }

    pub fn reset(&mut self, name: String) {
        self.circuitbreakers
            .lock()
            .unwrap()
            .get_mut(&name)
            .unwrap()
            .reset()
    }

    pub fn throttler(&mut self, name: String, limit: u16, time_window: Duration) {
        let tt = Throttler::new(limit, time_window);
        self.throttlers.lock().unwrap().insert(name, tt);
    }

    pub fn throttle(&mut self, name: String) -> bool {
        self.throttlers
            .lock()
            .unwrap()
            .get_mut(&name)
            .unwrap()
            .check()
    }
}
