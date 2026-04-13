//! Email utilities for sending notifications via AWS SES.
//!
//! This module provides standalone email functionality that can be used
//! by both the backend and stage workers.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "email")]
use aws_sdk_sesv2::{
    types::{Body, Content, Destination, EmailContent, Message},
    Client as SesClient,
};

/// Email client for sending messages via AWS SES.
#[cfg(feature = "email")]
#[derive(Clone)]
pub struct EmailClient {
    ses_client: Arc<Mutex<SesClient>>,
    from_address: String,
    from_identity_arn: String,
}

#[cfg(feature = "email")]
impl EmailClient {
    /// Creates a new EmailClient from AWS environment configuration.
    ///
    /// Uses AWS credentials from environment variables or IAM roles.
    pub async fn from_env() -> Result<Self> {
        let config = aws_config::load_from_env().await;
        let ses_client = SesClient::new(&config);

        let from_address = std::env::var("SES_FROM_ADDRESS")
            .unwrap_or_else(|_| "noreply@igaitapp.com".to_string());
        let from_identity_arn = std::env::var("SES_FROM_IDENTITY_ARN")
            .unwrap_or_else(|_| {
                "arn:aws:ses:us-east-2:851725269484:identity/noreply@igaitapp.com".to_string()
            });

        Ok(Self {
            ses_client: Arc::new(Mutex::new(ses_client)),
            from_address,
            from_identity_arn,
        })
    }

    /// Creates an EmailClient with a pre-existing SES client.
    pub fn with_client(
        ses_client: Arc<Mutex<SesClient>>,
        from_address: String,
        from_identity_arn: String,
    ) -> Self {
        Self {
            ses_client,
            from_address,
            from_identity_arn,
        }
    }

    /// Sends an email to the specified address.
    ///
    /// # Arguments
    /// * `to` - The recipient email address
    /// * `subject` - The email subject
    /// * `body_html` - The HTML body of the email
    pub async fn send(&self, to: &str, subject: &str, body_html: &str) -> Result<()> {
        println!("Sending email to '{to}'...");

        let destination = Destination::builder()
            .set_to_addresses(Some(vec![to.into()]))
            .build();

        let content = EmailContent::builder()
            .set_simple(Some(
                Message::builder()
                    .set_subject(Some(
                        Content::builder()
                            .set_data(Some(subject.to_string()))
                            .build()
                            .context("Failed to build email subject")?,
                    ))
                    .set_body(Some(
                        Body::builder()
                            .set_html(Some(
                                Content::builder()
                                    .set_data(Some(body_html.to_string()))
                                    .build()
                                    .context("Failed to build email body")?,
                            ))
                            .build(),
                    ))
                    .build(),
            ))
            .build();

        self.ses_client
            .lock()
            .await
            .send_email()
            .from_email_address(&self.from_address)
            .from_email_address_identity_arn(&self.from_identity_arn)
            .destination(destination)
            .content(content)
            .send()
            .await
            .context("Failed to send email via SES")?;

        println!("Successfully sent email to '{to}'!");
        Ok(())
    }
}

// ============================================================================
// EMAIL TEMPLATES
// ============================================================================

/// Pre-built email templates for common notifications.
pub struct EmailTemplates;

impl EmailTemplates {
    /// Builds a "submission received" welcome email.
    ///
    /// Sent when a user uploads a new job for processing.
    pub fn submission_received(
        datetime: &str,
        age: i16,
        ethnicity: &str,
        sex: char,
        height: &str,
        weight: i16,
        uid: &str,
        job_id: &str,
    ) -> (String, String) {
        let subject = "Welcome to iGait!".to_string();
        let body = format!(
            "Dear iGAIT user,<br><br>\
             Your submission on {} has been successfully received! \
             Please understand that the iGAIT website is still under development. \
             At this point, the research team will review the screening result of your submission. \
             We are working on adding the functionality to automatically email you the result. \
             We hope that will be available soon.<br><br>\
             In the meanwhile, if you have any questions regarding your submission or user experience, \
             or any suggestion to help us improve the website, please don't hesitate to contact us at \
             GaitStudy@niu.edu. Please include the information below about your submission.<br><br>\
             Submission information:<br>\
             Age: {}<br>\
             Ethnicity: {}<br>\
             Sex: {}<br>\
             Height: {}<br>\
             Weight: {}<br><br>\
             User ID: {}<br>\
             Job ID: {}",
            datetime, age, ethnicity, sex, height, weight, uid, job_id
        );
        (subject, body)
    }

    /// Builds a success email with the binary ASD/no-ASD result.
    ///
    /// Sent when the pipeline completes successfully with a prediction.
    pub fn prediction_success(
        datetime: &str,
        is_asd: bool,
        age: Option<i16>,
        ethnicity: Option<&str>,
        sex: Option<char>,
        height: Option<&str>,
        weight: Option<i16>,
        uid: &str,
        job_id: &str,
    ) -> (String, String) {
        let subject = "Your recent submission to iGait App has completed!".to_string();

        let result_text = if is_asd {
            "Submitted videos show walking pattern more similar to autistic children \
             than non-autistic children."
        } else {
            "Submitted videos show walking pattern more similar to non-autistic children \
             than autistic children."
        };

        let body = format!(
            "Dear iGAIT user,<br><br>\
             Your submission on {} has been processed!<br><br>\
             {}<br><br>\
             Submission information:<br>\
             Age: {}<br>\
             Ethnicity: {}<br>\
             Sex: {}<br>\
             Height: {}<br>\
             Weight: {}<br><br>\
             User ID: {}<br>\
             Job ID: {}<br><br>\
             If you have questions about your results, please contact GaitStudy@niu.edu.",
            datetime,
            result_text,
            age.map(|a| a.to_string()).unwrap_or_else(|| "N/A".to_string()),
            ethnicity.unwrap_or("N/A"),
            sex.map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string()),
            height.unwrap_or("N/A"),
            weight.map(|w| w.to_string()).unwrap_or_else(|| "N/A".to_string()),
            uid,
            job_id
        );
        (subject, body)
    }

    /// Builds a failure email when processing fails.
    ///
    /// Sent when the pipeline encounters an error at any stage.
    pub fn processing_failure(
        datetime: &str,
        failed_stage: Option<u8>,
        error: &str,
        uid: &str,
        job_id: &str,
    ) -> (String, String) {
        let subject = "Your recent submission to iGait App failed!".to_string();
        
        let stage_info = failed_stage
            .map(|s| format!("Stage {}", s))
            .unwrap_or_else(|| "Unknown stage".to_string());
        
        let body = format!(
            "Something went wrong with your submission on {}!<br><br>\
             Failed at: {}<br>\
             Error: {}<br><br>\
             User ID: {}<br>\
             Job ID: {}<br><br>\
             Please contact support: GaitStudy@niu.edu",
            datetime, stage_info, error, uid, job_id
        );
        (subject, body)
    }

    /// Builds a contribution thank-you email.
    ///
    /// Sent when a user contributes data to the research study.
    pub fn contribution_received(name: &str) -> (String, String) {
        let subject = "Thank you for your contribution to iGait!".to_string();
        let body = format!(
            "Dear {}!<br><br>\
             Your submission has been successfully received. \
             Thank you for participating in this research study. \
             If you have any questions or would like to follow up, \
             please contact GaitStudy@niu.edu.<br><br>\
             Thank you for your support!",
            name
        );
        (subject, body)
    }
}

#[cfg(feature = "email")]
impl std::fmt::Debug for EmailClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailClient")
            .field("from_address", &self.from_address)
            .finish()
    }
}
