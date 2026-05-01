//! This module contains all the helper functions used in the project.
//!
//! To learn more about the helper functions, check out the individual modules.

/// Contains the database helper functions.
pub mod database;

/// Contains the email helper functions.
pub mod email;

/// Contains the filesystem helper functions and custom types.
pub mod lib;

/// Publishes the central stage registry to RTDB on startup so the
/// frontend can render stage tabs directly from the source of truth.
pub mod registry_publish;
