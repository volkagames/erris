//! Shared error types for the integration tests.
//!
//! Included via `mod common;` in each test file. Not every file uses every
//! type, so the whole module is `#![allow(dead_code)]`.
#![allow(dead_code)]

use std::fmt;

/// A minimal `std::error::Error` with a fixed message.
#[derive(Debug)]
pub struct TestError;

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("TestError")
    }
}

impl std::error::Error for TestError {}

/// An error carrying a payload, for messages that vary per instance.
#[derive(Debug)]
pub struct Io(pub &'static str);

impl fmt::Display for Io {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "io: {}", self.0)
    }
}

impl std::error::Error for Io {}
