//! The engine, persistence and synthesizer have no third-party dependencies.
//! `rustc --edition=2021 --test src/lib.rs -o core-tests` works without Cargo.
#![forbid(unsafe_code)]
pub mod engine;
pub mod storage;
pub mod synth;
