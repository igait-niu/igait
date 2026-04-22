use crate::helper::lib::User;

use anyhow::{anyhow, Context, Result};
use igait_lib::microservice::FirebaseRtdb;
use uuid::Uuid;

use super::lib::{Job, JobStatus};

/// A wrapper around `FirebaseRtdb` scoped to the `users/` subtree.
///
/// Historically this wrapped `firebase_rs::Firebase`, but that crate enforces
/// an HTTPS scheme which blocked the local emulator. Consolidating on
/// `igait_lib::FirebaseRtdb` (a reqwest-based client already used by the
/// stages) unifies the Firebase client across backend + stages.
#[derive(Clone)]
pub struct Database {
    rtdb: FirebaseRtdb,
}

// `FirebaseRtdb` in igait-lib doesn't implement Debug; provide a stub so
// callers that debug-print an axum `State<Database>` still compile.
impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database").finish_non_exhaustive()
    }
}

impl Database {
    fn user_path(uid: &str) -> String {
        format!("users/{uid}")
    }

    fn jobs_path(uid: &str) -> String {
        format!("users/{uid}/jobs")
    }

    fn job_path(uid: &str, job_key: &str) -> String {
        format!("users/{uid}/jobs/{job_key}")
    }

    /// Initializes the Firebase wrapper.
    ///
    /// Reads `FIREBASE_RTDB_URL` and `FIREBASE_ACCESS_KEY` from the
    /// environment. Unlike the old `firebase_rs`-based init, this accepts
    /// both `http://` (emulator) and `https://` (prod) schemes.
    pub async fn init() -> Result<Self> {
        let rtdb = FirebaseRtdb::from_env().context("Failed to initialize FirebaseRtdb client")?;
        Ok(Self { rtdb })
    }

    /// Fetches all jobs for a user as a Vec, reading from an RTDB object
    /// keyed by string indices (e.g. `{"0": {...}, "1": {...}}`).
    ///
    /// Logs a warning for each entry that fails to deserialize.
    async fn get_jobs(&self, uid: &str) -> Result<Vec<Job>> {
        let raw: Option<serde_json::Value> = self
            .rtdb
            .get(&Self::jobs_path(uid))
            .await
            .context(format!("Failed to fetch jobs for user {uid}"))?;

        let Some(raw) = raw else { return Ok(vec![]) };

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
                    eprintln!("WARNING: failed to deserialize job '{key}' for user {uid}: {e}");
                }
            }
        }
        Ok(jobs)
    }

    /// Returns the number of jobs a user has by counting keys in the
    /// RTDB jobs object.
    async fn job_count(&self, uid: &str) -> Result<usize> {
        let raw: Option<serde_json::Value> = self
            .rtdb
            .get(&Self::jobs_path(uid))
            .await
            .context(format!("Failed to fetch jobs for user {uid}"))?;

        Ok(raw
            .and_then(|v| v.as_object().map(|o| o.len()))
            .unwrap_or(0))
    }

    /// Checks whether a specific job key exists in Firebase RTDB.
    async fn job_exists(&self, uid: &str, job_id: &str) -> Result<bool> {
        let v: Option<serde_json::Value> = self
            .rtdb
            .get(&Self::job_path(uid, job_id))
            .await
            .context(format!(
                "Failed to check existence of job {job_id} for user {uid}"
            ))?;
        Ok(v.is_some())
    }

    /// Ensures that a user exists in the database.
    ///
    /// Creates a new record with only `uid` and empty `jobs` if absent.
    /// Never touches `administrator` — PATCH semantics preserve the flag on
    /// existing records, and we deliberately omit it on create so the
    /// `serde(default = false)` on `User` applies.
    pub async fn ensure_user(&self, uid: &str) -> Result<()> {
        let path = Self::user_path(uid);

        println!("Verifying user existence...");
        let existing: Option<serde_json::Value> = self
            .rtdb
            .get(&path)
            .await
            .context("Failed to check user existence in database")?;

        if existing.is_some() {
            return Ok(());
        }

        println!("User doesn't exist, creating new user with UID '{uid}'...");
        self.rtdb
            .update(
                &path,
                &serde_json::json!({
                    "uid": uid,
                    "jobs": {}
                }),
            )
            .await
            .context("Failed to create a new user while ensuring they existed!")?;
        println!("Successfully created new user!");
        Ok(())
    }

    /// Checks whether a user has administrator privileges.
    ///
    /// Reads ONLY the `administrator` field, avoiding deserialization of
    /// the full user record (which includes all jobs).
    pub async fn is_admin(&self, uid: &str) -> Result<bool> {
        let path = format!("{}/administrator", Self::user_path(uid));
        let val: Option<bool> = self
            .rtdb
            .get(&path)
            .await
            .context("Failed to check admin status")?;
        Ok(val.unwrap_or(false))
    }

    /// Fetches a user record from the database.
    pub async fn get_user(&self, uid: &str) -> Result<User> {
        self.ensure_user(uid)
            .await
            .context("Failed to ensure user!")?;

        self.rtdb
            .get::<User>(&Self::user_path(uid))
            .await
            .context("Failed to get user!")?
            .ok_or_else(|| anyhow!("User not found after ensure!"))
    }

    /// Counts the number of jobs a user has.
    pub async fn count_jobs(&self, uid: &str) -> Result<usize> {
        println!("Counting jobs...");
        self.ensure_user(uid)
            .await
            .context("Failed to ensure user!")?;
        self.job_count(uid).await.context("Failed to count jobs!")
    }

    /// Adds a new job to the user's job list.
    pub async fn new_job(&self, uid: &str, job: Job) -> Result<String> {
        self.ensure_user(uid)
            .await
            .context("Failed to ensure user!")?;

        let job_key = Uuid::new_v4().to_string();

        // PATCH (update) merges fields into the job object. A fresh key has
        // nothing to merge against, so behavior is equivalent to PUT here.
        self.rtdb
            .update(&Self::job_path(uid, &job_key), &job)
            .await
            .context("Failed to write new job to database!")?;

        println!("Added new job with key '{job_key}'!");
        Ok(job_key)
    }

    /// Updates the status of a job.
    pub async fn update_status(&self, uid: &str, job_key: &str, status: JobStatus) -> Result<()> {
        println!("Updating status...");

        self.ensure_user(uid)
            .await
            .context("Failed to ensure user!")?;

        if !self.job_exists(uid, job_key).await? {
            return Err(anyhow!("Job key '{}' does not exist!", job_key));
        }

        // PUT the status — it's a single value, not a mergeable object.
        let path = format!("{}/status", Self::job_path(uid, job_key));
        self.rtdb
            .set(&path, &status)
            .await
            .context("Failed to update the job status in the database!")?;

        let code = status.code();
        let value = status.description();
        println!("Updated status successfully to {code} with message '{value}'!");
        Ok(())
    }

    /// Gets the status of a job.
    pub async fn _get_status(&self, uid: &str, job_key: &str) -> Result<JobStatus> {
        println!("Getting status...");

        self.ensure_user(uid)
            .await
            .context("Failed to ensure user!")?;

        let path = format!("{}/status", Self::job_path(uid, job_key));
        self.rtdb
            .get::<JobStatus>(&path)
            .await
            .context(format!("Failed to get status for job {}", job_key))?
            .ok_or_else(|| anyhow!("Status not found for job {}", job_key))
    }

    /// Gets a job given a user ID and a job ID.
    pub async fn get_job(&self, uid: &str, job_key: &str) -> Result<Job> {
        println!("Getting job...");

        self.ensure_user(uid)
            .await
            .context("Failed to ensure user!")?;

        self.rtdb
            .get::<Job>(&Self::job_path(uid, job_key))
            .await
            .context(format!("Failed to get job {}", job_key))?
            .ok_or_else(|| anyhow!("Job {} not found", job_key))
    }

    /// Gets all jobs of a user.
    pub async fn get_all_jobs(&self, uid: &str) -> Result<Vec<Job>> {
        println!("Getting all jobs...");

        self.ensure_user(uid)
            .await
            .context("Failed to ensure user!")?;

        self.get_jobs(uid).await.context("Failed to get jobs!")
    }
}
