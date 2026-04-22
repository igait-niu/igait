//! Email utilities for the backend.
//!
//! This module provides email sending capabilities using the shared
//! email client from igait-lib, with some backend-specific wrappers.

use std::time::SystemTime;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::{AppState, Arc};
use igait_lib::microservice::EmailTemplates;

use super::lib::Job;

/// Sends an email using the app's AWS SES client.
///
/// This is a low-level function - prefer using the higher-level wrappers below.
pub async fn send_email(app: Arc<AppState>, to: &str, subject: &str, body: &str) -> Result<()> {
    app.email_client.send(to, subject, body).await
}

/// Sends a "submission received" welcome email to the user.
///
/// Called when a user uploads a new job for processing.
pub async fn send_welcome_email(
    app: Arc<AppState>,
    job: &Job,
    uid: &str,
    job_key: &str,
) -> Result<()> {
    let dt_now_utc: DateTime<Utc> = SystemTime::now().into();
    let dt_now_cst = dt_now_utc.with_timezone(&chrono_tz::US::Central);

    let (subject, body) = EmailTemplates::submission_received(
        &dt_now_cst.to_string(),
        job.age,
        &job.ethnicity.to_string(),
        job.sex.to_string().chars().next().unwrap_or('O'),
        &job.height,
        job.weight,
        uid,
        job_key,
    );

    send_email(app, &job.email, &subject, &body).await
}

/// Sends a contribution thank-you email.
///
/// Called when a user contributes data to the research study.
pub async fn send_contribution_email(app: Arc<AppState>, email: &str, name: &str) -> Result<()> {
    let (subject, body) = EmailTemplates::contribution_received(name);
    send_email(app, email, &subject, &body).await
}
