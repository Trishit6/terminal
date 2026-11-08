//! Background job tracking.
//!
//! The table owns the operating-system handles for background external
//! processes. Shell-expression jobs run on a detached thread and report
//! completion through [`JobTable::complete`]. A job table is shared by cloned
//! sessions so pipeline clones and the interactive shell observe the same jobs.

use std::process::Child;

/// Lifecycle state of a background job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    /// The job is still executing.
    Running,
    /// The job finished with a successful status.
    Done,
    /// The job finished with a failing status.
    Failed,
    /// The job was terminated through `kill`.
    Killed,
}

impl JobStatus {
    /// Returns `true` while the job is still running.
    #[must_use]
    pub const fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }
}

/// Public snapshot of one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobInfo {
    /// Shell-assigned job number.
    pub id: u32,
    /// Command text associated with the job.
    pub command: String,
    /// Current lifecycle state.
    pub status: JobStatus,
    /// Exit code when the job completed normally.
    pub exit_code: Option<i32>,
}

impl JobInfo {
    /// Returns `true` when the job has finished successfully.
    #[must_use]
    pub const fn success(&self) -> bool {
        matches!(self.status, JobStatus::Done)
    }
}

#[derive(Debug)]
struct JobRecord {
    id: u32,
    command: String,
    status: JobStatus,
    exit_code: Option<i32>,
    child: Option<Child>,
}

/// Collection of background jobs owned by a session.
#[derive(Debug, Default)]
pub struct JobTable {
    next_id: u32,
    jobs: Vec<JobRecord>,
}

impl JobTable {
    /// Creates an empty job table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a shell-expression job whose completion will be reported later.
    pub fn add_shell(&mut self, command: impl Into<String>) -> u32 {
        self.add(command.into(), None)
    }

    /// Registers a background external process.
    pub fn add_external(&mut self, command: impl Into<String>, child: Child) -> u32 {
        self.add(command.into(), Some(child))
    }

    fn add(&mut self, command: String, child: Option<Child>) -> u32 {
        self.next_id = self.next_id.saturating_add(1).max(1);
        let id = self.next_id;
        self.jobs.push(JobRecord {
            id,
            command,
            status: JobStatus::Running,
            exit_code: None,
            child,
        });
        id
    }

    /// Reaps completed external children without blocking.
    pub fn refresh(&mut self) {
        for job in &mut self.jobs {
            if !job.status.is_running() {
                continue;
            }

            let Some(child) = job.child.as_mut() else {
                continue;
            };

            match child.try_wait() {
                Ok(Some(status)) => {
                    job.exit_code = status.code();
                    job.status = if status.success() {
                        JobStatus::Done
                    } else {
                        JobStatus::Failed
                    };
                    job.child = None;
                }
                Ok(None) => {}
                Err(_) => {
                    job.status = JobStatus::Failed;
                    job.child = None;
                }
            }
        }
    }

    /// Returns a snapshot of every job, oldest first.
    pub fn snapshot(&mut self) -> Vec<JobInfo> {
        self.refresh();
        self.jobs
            .iter()
            .map(|job| JobInfo {
                id: job.id,
                command: job.command.clone(),
                status: job.status,
                exit_code: job.exit_code,
            })
            .collect()
    }

    /// Returns one job after refreshing external status.
    pub fn get(&mut self, id: u32) -> Option<JobInfo> {
        self.refresh();
        self.jobs
            .iter()
            .find(|job| job.id == id)
            .map(|job| JobInfo {
                id: job.id,
                command: job.command.clone(),
                status: job.status,
                exit_code: job.exit_code,
            })
    }

    /// Marks a shell-expression job as finished.
    ///
    /// A job that was killed is not resurrected by a late completion report.
    pub fn complete(&mut self, id: u32, success: bool, exit_code: Option<i32>) {
        let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) else {
            return;
        };

        if !job.status.is_running() {
            return;
        }

        job.status = if success {
            JobStatus::Done
        } else {
            JobStatus::Failed
        };
        job.exit_code = exit_code;
    }

    /// Terminates a running job.
    ///
    /// External processes are killed through their owned [`Child`] handle.
    /// Shell-expression jobs cannot be interrupted safely from another thread;
    /// they are marked killed and may finish independently.
    pub fn kill(&mut self, id: u32) -> Result<JobInfo, String> {
        self.refresh();
        let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) else {
            return Err(format!("job {id} does not exist"));
        };

        if !job.status.is_running() {
            return Err(format!("job {id} is not running"));
        }

        if let Some(mut child) = job.child.take() {
            child
                .kill()
                .map_err(|error| format!("job {id}: could not terminate process: {error}"))?;
            let _ = child.wait();
        }

        job.status = JobStatus::Killed;
        job.exit_code = None;

        Ok(JobInfo {
            id: job.id,
            command: job.command.clone(),
            status: job.status,
            exit_code: job.exit_code,
        })
    }

    /// Removes a job from the table and returns its final snapshot.
    pub fn remove(&mut self, id: u32) -> Option<JobInfo> {
        self.refresh();
        let index = self.jobs.iter().position(|job| job.id == id)?;
        let job = self.jobs.remove(index);
        Some(JobInfo {
            id: job.id,
            command: job.command,
            status: job.status,
            exit_code: job.exit_code,
        })
    }

    /// Returns the most recently created job.
    pub fn last_id(&mut self) -> Option<u32> {
        self.refresh();
        self.jobs.last().map(|job| job.id)
    }

    /// Returns `true` when every tracked job has finished.
    pub fn all_finished(&mut self) -> bool {
        self.refresh();
        self.jobs.iter().all(|job| !job.status.is_running())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn shell_jobs_complete_and_keep_their_status() {
        let mut table = JobTable::new();
        let id = table.add_shell("echo background");

        assert_eq!(table.snapshot()[0].status, JobStatus::Running);

        table.complete(id, true, Some(0));

        let info = table.get(id).expect("job should exist");
        assert_eq!(info.status, JobStatus::Done);
        assert_eq!(info.exit_code, Some(0));
        assert!(info.success());
    }

    #[test]
    fn a_killed_shell_job_is_not_resurrected_by_completion() {
        let mut table = JobTable::new();
        let id = table.add_shell("sleep");

        let killed = table.kill(id).expect("job should be killable");
        assert_eq!(killed.status, JobStatus::Killed);

        table.complete(id, true, Some(0));
        assert_eq!(table.get(id).unwrap().status, JobStatus::Killed);
    }

    #[test]
    fn external_jobs_are_reaped_and_removed() {
        let child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "exit", "0"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("cmd should spawn")
        } else {
            Command::new("true")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("true should spawn")
        };

        let mut table = JobTable::new();
        let id = table.add_external("true", child);

        let mut attempts = 0;
        while table.get(id).is_some_and(|job| job.status.is_running()) && attempts < 100 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            attempts += 1;
        }

        let info = table.get(id).expect("job should exist");
        assert!(!info.status.is_running());

        let removed = table.remove(id).expect("job should be removable");
        assert_eq!(removed.id, id);
        assert!(table.snapshot().is_empty());
    }
}
