//! Microservice types and utilities for the iGait pipeline.
//! 
//! This module provides shared types, traits, and utilities used by all
//! stage microservices in the iGait pipeline.

mod types;
mod storage;
mod queue;
mod backend_status;
mod registry;

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
pub use registry::*;

#[cfg(feature = "microservice")]
pub use worker::*;

#[cfg(feature = "microservice")]
pub use retry::*;

#[cfg(feature = "email")]
pub use email::*;

/// Common env vars every stage worker needs for queue + storage access.
pub const STAGE_REQUIRED_ENV: &[&str] = &[
    "FIREBASE_RTDB_URL",
    "FIREBASE_ACCESS_KEY",
    "IGAIT_S3_BUCKET",
    "AWS_REGION",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
];

/// Env vars the finalize stage needs on top of STAGE_REQUIRED_ENV.
pub const FINALIZE_REQUIRED_ENV: &[&str] = &[
    "SES_FROM_ADDRESS",
    "SES_FROM_IDENTITY_ARN",
];

/// Env vars the backend needs to start.
pub const BACKEND_REQUIRED_ENV: &[&str] = &[
    "GOOGLE_APPLICATION_CREDENTIALS",
    "FIREBASE_PROJECT_ID",
    "FIREBASE_ACCESS_KEY",
    "FIREBASE_RTDB_URL",
    "IGAIT_S3_BUCKET",
    "AWS_REGION",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "SES_FROM_ADDRESS",
    "SES_FROM_IDENTITY_ARN",
];

/// Verifies that every env var in `required` is set. Accumulates all
/// missing names and fails with one error, rather than stopping at the
/// first. Call this at the very top of `main` so a misconfigured
/// environment fails loudly before any service client is constructed.
pub fn check_env(required: &[&str]) -> anyhow::Result<()> {
    let missing: Vec<&&str> = required
        .iter()
        .filter(|v| std::env::var(v).is_err())
        .collect();
    if !missing.is_empty() {
        anyhow::bail!("Missing required env vars: {:?}", missing);
    }
    Ok(())
}
