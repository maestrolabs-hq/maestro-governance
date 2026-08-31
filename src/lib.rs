//! The parsing and comparison half of governance, separated from the binary so
//! it can be tested without reaching GitHub.
//!
//! Everything here is pure: text in, drift out. The half that runs `gh` lives
//! in the binary, because a test that needs a network is a test that gets
//! disabled.

pub mod baseline;
