use crate::helper::lib::User;

use firebase_rs::*;
use anyhow::{ Context, Result, anyhow };
use uuid::Uuid;

use super::lib::{Job, JobStatus};

/// Checks if a firebase_rs error is a "not found / null body" — meaning the
/// RTDB path doesn't exist. This is expected for new users or empty arrays
/// and should NOT be treated as a failure. All other errors (network, auth,
/// deserialization) are real problems that must propagate.
fn is_not_found(err: &RequestError) -> bool {
    matches!(err, RequestError::NotFoundOrNullBody)
}

/// A wrapper class on the Firebase database to make it easier to interact with.
#[derive( Debug )]
pub struct Database {
    _state: Firebase
}
impl Database {
    /// Initializes the Firebase wrapper class.
    /// 
    /// # Fails
    /// * If the Firebase URL is invalid
    /// * If the Firebase access key is missing
    /// 
    /// # Returns
    /// * The Firebase wrapper class
    /// 
    /// # Notes
    /// * The Firebase URL is the URL to the Firebase database, not the URL to the Firebase console.
    /// * The Firebase access key is the key that allows you to access the Firebase database.
    /// * The Firebase access key should be stored in the system environment as `FIREBASE_ACCESS_KEY`.
    pub async fn init () -> Result<Self> {
        Ok(Self {
            _state: Firebase::auth("https://network-technology-project-default-rtdb.firebaseio.com/", &std::env::var("FIREBASE_ACCESS_KEY").context("Missing FIREBASE_ACCESS_KEY! Make sure it's set in your system environment.")?)
                .map_err(|e| anyhow!("{e:?}"))
                .context("Couldn't unwrap the URL while trying to initialize the Firebase wrapper class!")?
                .at("users")
        })
    }

    /// Fetches all jobs for a user as a Vec, reading from an RTDB object
    /// keyed by string indices (e.g. `{"0": {...}, "1": {...}}`).
    ///
    /// Logs a warning for each entry that fails to deserialize.
    async fn get_jobs(&self, uid: &str) -> Result<Vec<Job>> {
        let raw = match self._state.at(uid).at("jobs").get::<serde_json::Value>().await {
            Ok(v) => v,
            Err(e) if is_not_found(&e) => return Ok(vec![]),
            Err(e) => return Err(anyhow!("{e:?}"))
                .context(format!("Failed to fetch jobs for user {uid}")),
        };

        let obj = match raw.as_object() {
            Some(o) => o,
            None => {
                eprintln!("WARNING: jobs path for user {uid} is not an object: {raw}");
                return Ok(vec![]);
            }
        };

        let mut jobs = Vec::with_capacity(obj.len());
        for (key, v) in obj {
            if v.is_null() {
                continue;
            }
            match serde_json::from_value::<Job>(v.clone()) {
                Ok(job) => jobs.push(job),
                Err(e) => {
                    eprintln!(
                        "WARNING: failed to deserialize job '{key}' for user {uid}: {e}"
                    );
                }
            }
        }
        Ok(jobs)
    }

    /// Returns the number of jobs a user has by counting keys in the
    /// RTDB jobs object.
    async fn job_count(&self, uid: &str) -> Result<usize> {
        let raw = match self._state.at(uid).at("jobs").get::<serde_json::Value>().await {
            Ok(v) => v,
            Err(e) if is_not_found(&e) => return Ok(0),
            Err(e) => return Err(anyhow!("{e:?}"))
                .context(format!("Failed to fetch jobs for user {uid}")),
        };

        let obj = match raw.as_object() {
            Some(o) => o,
            None => return Ok(0),
        };

        Ok(obj.len())
    }

    /// Checks whether a specific job key exists in Firebase RTDB.
    async fn job_exists(&self, uid: &str, job_id: &str) -> Result<bool> {
        match self._state.at(uid).at("jobs").at(job_id).get::<serde_json::Value>().await {
            Ok(v) => Ok(!v.is_null()),
            Err(e) if is_not_found(&e) => Ok(false),
            Err(e) => Err(anyhow!("{e:?}"))
                .context(format!("Failed to check existence of job {job_id} for user {uid}")),
        }
    }

    /// Ensures that a user exists in the database.
    /// 
    /// # Arguments
    /// * `uid` - The user ID to ensure exists.
    /// 
    /// # Fails
    /// * If the user doesn't exist and can't be created
    /// * If the user can't be updated
    /// 
    /// # Returns
    /// * A successful result if the user exists
    /// 
    /// # Notes
    /// * This function creates a new user if the user doesn't exist.
    
    pub async fn ensure_user (
        &self,
        uid: &str
    ) -> Result<()> {
        // Create a path to the user in the database
        let user_handle = self._state.at(uid);

        // Check if the user doesn't exist using a raw Value to avoid
        // deserialization failures on existing records (which previously
        // caused silent data corruption via .ok() swallowing errors).
        println!("Verifying user existence...");
        match user_handle.get::<Option<serde_json::Value>>().await {
            Ok(Some(_)) => {
                // User already exists — nothing to do
            }
            Ok(None) => {
                println!("User doesn't exist, creating new user with UID '{uid}'...");

                // Create a new user with only uid and empty jobs.
                // Do NOT set administrator here — it defaults to false on read
                // via serde(default), and explicitly writing false would overwrite
                // an existing admin flag if this ever runs against a partial record.
                user_handle.update(&serde_json::json!({
                    "uid": uid,
                    "jobs": {}
                })).await
                    .map_err(|e| anyhow!("{e:?}"))
                    .context("Failed to create a new user while ensuring they existed!")?;
                println!("Successfully created new user!");
            }
            Err(e) => {
                return Err(anyhow!("{e:?}"))
                    .context("Failed to check user existence in database");
            }
        }

        Ok(())
    }

    /// Checks whether a user has administrator privileges.
    ///
    /// This reads ONLY the `administrator` field, avoiding deserialization
    /// of the full user record (which includes all jobs). This is both
    /// faster and safer for admin-gating endpoints.
    pub async fn is_admin(&self, uid: &str) -> Result<bool> {
        let admin_handle = self._state.at(uid).at("administrator");
        match admin_handle.get::<Option<bool>>().await {
            Ok(Some(val)) => Ok(val),
            Ok(None) => Ok(false),
            Err(e) => Err(anyhow!("{e:?}")).context("Failed to check admin status"),
        }
    }

    /// Fetches a user record from the database.
    ///
    /// # Arguments
    /// * `uid` - The user ID to look up.
    ///
    /// # Fails
    /// * If the user doesn't exist and can't be created
    /// * If the user record can't be read
    ///
    /// # Returns
    /// * The `User` record
    pub async fn get_user(
        &self,
        uid: &str,
    ) -> Result<User> {
        self.ensure_user(uid).await.context("Failed to ensure user!")?;

        self._state
            .at(uid)
            .get::<Option<User>>()
            .await
            .map_err(|e| anyhow!("{e:?}"))
            .context("Failed to get user!")?
            .ok_or_else(|| anyhow!("User not found after ensure!"))
    }

    /// Counts the number of jobs a user has.
    /// 
    /// # Arguments
    /// * `uid` - The user ID to count the jobs of.
    /// 
    /// # Fails
    /// * If the user can't be ensured
    /// * If the jobs can't be counted
    /// 
    /// # Returns
    /// * The number of jobs the user has
    /// 
    /// # Notes
    /// * This function creates a new user if the user doesn't exist.
    /// * This function returns `0` if there are no jobs, but the user exists.
    
    pub async fn count_jobs (
        &self,
        uid:         &str
    ) -> Result<usize> {
        println!("Counting jobs...");

        // First double check that the user actually exists
        self.ensure_user(uid).await.context("Failed to ensure user!")?;

        self.job_count(uid).await
            .context("Failed to count jobs!")
    }

    /// Adds a new job to the user's job list.
    /// 
    /// # Arguments
    /// * `uid` - The user ID to add the job to.
    /// * `job` - The job to add to the user's job list.
    /// 
    /// # Fails
    /// * If the user doesn't exist and can't be created
    /// * If the jobs can't be updated
    /// 
    /// # Returns
    /// * A successful result if the job was added
    /// 
    /// # Notes
    /// * This function creates a new user if the user doesn't exist.
    /// * This function adds the job to the end of the user's job list.
    
    pub async fn new_job (
        &self,
        uid:         &str,
        job:         Job
    ) -> Result<String> {
        // First double check that the user actually exists
        self.ensure_user(uid).await.context("Failed to ensure user!")?;

        // Generate a unique key for this job
        let job_key = Uuid::new_v4().to_string();

        // Write the new job at the generated key
        self._state.at(uid).at("jobs").at(&job_key)
            .update(&job)
            .await
            .map_err(|e| anyhow!("{e:?}"))
            .context("Failed to write new job to database!")?;

        println!("Added new job with key '{job_key}'!");
        Ok(job_key)
    }

    /// Updates the status of a job.
    /// 
    /// # Arguments
    /// * `uid` - The user ID to update the job of.
    /// * `job_id` - The ID of the job to update.
    /// * `status` - The new status of the job.
    /// 
    /// # Fails
    /// * If the user doesn't exist and can't be created
    /// * If the job ID doesn't exist
    ///  
    /// # Returns
    /// * A successful result if the status was updated
    /// 
    /// # Notes
    /// * This function creates a new user if the user doesn't exist.
    /// * This function overwrites the status of the job with the new status.
    
    pub async fn update_status (
        &self,
        uid:         &str,
        job_key:     &str,
        status:      JobStatus
    ) -> Result<()> {
        println!("Updating status...");

        // First double check that the user actually exists
        self.ensure_user(uid).await.context("Failed to ensure user!")?;

        // Verify the job key exists
        if !self.job_exists(uid, job_key).await? {
            return Err(anyhow!("Job key '{}' does not exist!", job_key));
        }

        // Write only the specific job's status — never touch the administrator field
        self._state.at(uid).at("jobs").at(job_key).at("status")
            .update(&status)
            .await
            .map_err(|e| anyhow!("{e:?}"))
            .context("Failed to update the job status in the database!")?;

        // Return as successful
        let code = status.code();
        let value = status.description();
        println!("Updated status successfully to {code} with message '{value}'!");
        Ok(())
    }

    /// Gets the status of a job.
    /// 
    /// # Arguments
    /// * `uid` - The user ID to get the job status of.
    /// * `job_id` - The ID of the job to get the status of.
    /// 
    /// # Fails
    /// * If the user doesn't exist and can't be created
    /// * If the job ID doesn't exist
    /// 
    /// # Returns
    /// * The status of the job
    /// 
    /// # Notes
    /// * This function creates a new user if the user doesn't exist.
    
    pub async fn _get_status (
        &self,
        uid:         &str,
        job_key:     &str
    ) -> Result<JobStatus> {
        println!("Getting status...");

        // First double check that the user actually exists
        self.ensure_user(uid).await.context("Failed to ensure user!")?;

        // Read just this job's status directly
        self._state.at(uid).at("jobs").at(job_key).at("status")
            .get::<JobStatus>()
            .await
            .map_err(|e| anyhow!("{e:?}"))
            .context(format!("Failed to get status for job {}", job_key))
    }

    /// Gets a job given a user ID and a job ID.
    /// 
    /// # Arguments
    /// * `uid` - The user ID to get the job of.
    /// * `job_id` - The ID of the job to get.
    /// 
    /// # Fails
    /// * If the user doesn't exist and can't be created
    /// * If the job ID doesn't exist
    /// 
    /// # Returns
    /// * The job
    /// 
    /// # Notes
    /// * This function creates a new user if the user doesn't exist.
    
    pub async fn get_job (
        &self,
        uid:         &str,
        job_key:     &str
    ) -> Result<Job> {
        println!("Getting job...");

        // First double check that the user actually exists
        self.ensure_user(uid).await.context("Failed to ensure user!")?;

        // Read just this specific job directly
        self._state.at(uid).at("jobs").at(job_key)
            .get::<Job>()
            .await
            .map_err(|e| anyhow!("{e:?}"))
            .context(format!("Failed to get job {}", job_key))
    }

    /// Gets all jobs of a user.
    /// 
    /// # Arguments
    /// * `uid` - The user ID to get the jobs of.
    /// 
    /// # Fails
    /// * If the user doesn't exist and can't be created
    /// 
    /// # Returns
    /// * The jobs of the user
    /// 
    /// # Notes
    /// * This function creates a new user if the user doesn't exist.
    
    pub async fn get_all_jobs (
        &self,
        uid:         &str
    ) -> Result<Vec<Job>> {
        println!("Getting all jobs...");

        // First double check that the user actually exists
        self.ensure_user(uid).await.context("Failed to ensure user!")?;

        // Get the jobs (returns empty vec if none exist)
        self.get_jobs(uid).await
            .context("Failed to get jobs!")
    }
}