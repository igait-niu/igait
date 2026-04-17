//! Microservice types and utilities for the iGait pipeline.
//! 
//! This module provides shared types, traits, and utilities used by all
//! stage microservices in the iGait pipeline.

mod types;
mod storage;
mod queue;
mod backend_status;

#[cfg(feature = "microservice")]
mod worker;

#[cfg(feature = "microservice")]
mod retry;

#[cfg(feature = "email")]
mod email;

pub use types::*;
pub use storage::*;
pub use queue::*;
pub use backend_status::*;

#[cfg(feature = "microservice")]
pub use worker::*;

#[cfg(feature = "microservice")]
pub use retry::*;

#[cfg(feature = "email")]
pub use email::*;
