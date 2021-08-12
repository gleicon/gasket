use chrono::{DateTime, Duration, Local};
use log::info;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{dev, http, HttpResponse, Result};

// Stability patterns:
// Throttling: Ensure only max_requests can happen on a given timewindow
// Circuit Breaker: once max_tries with errors are reached, trip and interrupt the circuit. it can be reset
// Exponential Backoff: exponentially increses Timeout for each retry

#[derive(Clone, Copy)]
pub struct CircuitBreaker {
    error_count: u16,
    max_trips: u16,
    last_error: DateTime<Local>,
    created_at: DateTime<Local>,
}

#[derive(Clone, Copy)]
pub struct ExponentialBackoff {
    current_timeout: Duration,
    requests: i32,
    max_timeout: Duration,
    max_requests: i32,
}

pub struct StabilityPatterns {
    pub circuitbreakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    pub backoffs: Arc<Mutex<HashMap<String, ExponentialBackoff>>>,
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

    fn status(&mut self) -> bool {
        if self.error_count > self.max_trips {
            return false;
        }
        return true;
    }
}

impl ExponentialBackoff {
    fn new() -> Self {
        Self {
            current_timeout: Duration::milliseconds(100),
            requests: 0,
            max_timeout: Duration::seconds(60), // hardcoded 60s ceiling
            max_requests: 50, // arbitrary upper limit before resetting (or failing for good)
        }
    }

    fn current(&mut self) -> Duration {
        self.current_timeout
    }

    fn next(&mut self) -> Duration {
        let base = 2.0f64;
        if self.requests == 0 {
            self.current_timeout = Duration::milliseconds(100);
        } else {
            // cap to max_timeout
            let to = self.current_timeout
                + Duration::milliseconds((base.powi(self.requests) * 10.0) as i64);

            if to > self.max_timeout {
                self.current_timeout = self.max_timeout
            } else {
                self.current_timeout = to
            }
        }
        self.requests += 1;
        self.current_timeout
    }
    // next but resets after max_requests
    // be careful
    fn next_with_reset(&mut self) -> Duration {
        let d = self.next();
        if self.requests > self.max_requests {
            self.reset()
        }
        d
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
            backoffs: Arc::new(Mutex::new(HashMap::new())),
        };
        return s;
    }

    pub fn circuitbreaker(&mut self, name: String, max_trips: u16) {
        let cb = CircuitBreaker::new(max_trips);
        self.circuitbreakers.lock().unwrap().insert(name, cb);
    }

    pub fn trip(&mut self, name: String) -> bool {
        if !self
            .circuitbreakers
            .lock()
            .unwrap()
            .contains_key(&name.clone())
        {
            let cb = CircuitBreaker::new(10); // arbitrary max trips
            self.circuitbreakers
                .lock()
                .unwrap()
                .insert(name.clone(), cb);
        }
        self.circuitbreakers
            .lock()
            .unwrap()
            .get_mut(&name)
            .unwrap()
            .trip()
    }

    pub fn check_cb_status(&mut self, name: String) -> bool {
        if !self
            .circuitbreakers
            .lock()
            .unwrap()
            .contains_key(&name.clone())
        {
            let cb = CircuitBreaker::new(10); // arbitrary max trips
            self.circuitbreakers
                .lock()
                .unwrap()
                .insert(name.clone(), cb);
        }
        self.circuitbreakers
            .lock()
            .unwrap()
            .get_mut(&name)
            .unwrap()
            .status()
    }

    pub fn reset(&mut self, name: String) {
        self.circuitbreakers
            .lock()
            .unwrap()
            .get_mut(&name)
            .unwrap()
            .reset()
    }

    pub fn exponential_backoff(&mut self, name: String) {
        if !self.backoffs.lock().unwrap().contains_key(&name.clone()) {
            let eb = ExponentialBackoff::new();
            self.backoffs.lock().unwrap().insert(name, eb);
        }
    }

    pub fn next_backoff(&mut self, name: String) -> Duration {
        match self.backoffs.lock().unwrap().get_mut(&name.clone()) {
            Some(q) => {
                let n = q.next();
                info!("backing off for {}: {} - {:?}", name.clone(), q.requests, n);
                n
            }
            None => {
                info!(
                    "backing off for {} with 5s Duration (bypassing backoff)",
                    name.clone(),
                );
                Duration::seconds(5) // default http client timeout
            }
        }
    }

    pub fn current_timeout(&mut self, name: String) -> Duration {
        self.backoffs
            .lock()
            .unwrap()
            .get_mut(&name)
            .unwrap()
            .current()
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
