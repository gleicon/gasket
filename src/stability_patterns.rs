use chrono::{DateTime, Duration, Local};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Stability patterns:
// Throttling: Ensure only max_requests can happen on a given timewindow
// Circuit Breaker: once max_tries with errors are reached, trip and interrupt the circuit. it can be reset
// Exponential Backoff: exponentially increses Timeout for each retry

struct CircuitBreaker {
    error_count: u16,
    max_trips: u16,
    last_error: DateTime<Local>,
    created_at: DateTime<Local>,
}

struct Throttler {
    max_requests: i32,
    current_requests: i32,
    last_request: DateTime<Local>,
    time_window: Duration,
}

struct ExponentialBackoff {
    current_timeout: Duration,
    requests: i32,
}

pub struct StabilityPatterns {
    circuitbreakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    throttlers: Arc<Mutex<HashMap<String, Throttler>>>,
    backoffs: Arc<Mutex<HashMap<String, ExponentialBackoff>>>,
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
    fn new(limit: i32, time_window: Duration) -> Self {
        Self {
            max_requests: limit,
            current_requests: 0,
            last_request: Local::now(),
            time_window,
        }
    }
    fn check(&mut self) -> bool {
        if (self.last_request - Local::now()) < self.time_window {
            self.current_requests += 1;
            if self.current_requests > self.max_requests {
                return false;
            }
        } else {
            self.current_requests = 1;
        }
        return true;
    }
}

impl ExponentialBackoff {
    fn new() -> Self {
        Self {
            current_timeout: Duration::milliseconds(100),
            requests: 0,
        }
    }

    fn next(&mut self) -> Duration {
        let base = 2.0f64;
        if self.requests == 0 {
            self.current_timeout = Duration::milliseconds(100);
        } else {
            self.current_timeout = self.current_timeout
                + Duration::milliseconds((base.powi(self.requests) * 10.0) as i64);
        }
        self.requests += 1;
        self.current_timeout
    }

    fn reset(&mut self) {
        self.requests = 0;
        self.current_timeout = Duration::milliseconds(100);
    }
}

impl StabilityPatterns {
    pub fn new() -> Self {
        let s = Self {
            circuitbreakers: Arc::new(Mutex::new(HashMap::new())),
            throttlers: Arc::new(Mutex::new(HashMap::new())),
            backoffs: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn throttler(&mut self, name: String, limit: i32, time_window: Duration) {
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

    pub fn exponential_backoff(&mut self, name: String) {
        let eb = ExponentialBackoff::new();
        self.backoffs.lock().unwrap().insert(name, eb);
    }

    pub fn next_backoff(&mut self, name: String) -> Duration {
        self.backoffs.lock().unwrap().get_mut(&name).unwrap().next()
    }

    pub fn reset_backoff(&mut self, name: String) {
        self.backoffs
            .lock()
            .unwrap()
            .get_mut(&name)
            .unwrap()
            .reset()
    }
}
