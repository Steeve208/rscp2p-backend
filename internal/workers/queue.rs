//! Redis-backed retry queue with exponential backoff and dead-letter support.
//!
//! | Key pattern | Type | Purpose |
//! |-------------|------|---------|
//! | `worker:schedule:{queue}` | ZSET | job_id scored by run-at epoch ms |
//! | `worker:job:{id}` | STRING | JSON [`WorkerJob`] payload |
//! | `worker:dlq:{queue}` | LIST | dead-lettered job JSON blobs |

use std::time::Duration;

use redis::aio::ConnectionManager;
use serde_json::Value;
use sqlx::PgPool;
use tracing::{error, warn};
use uuid::Uuid;

use crate::internal::config::WorkerConfig;

use super::backoff::exponential_backoff;
use super::error::{WorkerError, WorkerResult};
use super::job::WorkerJob;

const JOB_PREFIX: &str = "worker:job:";
const SCHEDULE_PREFIX: &str = "worker:schedule:";
const DLQ_PREFIX: &str = "worker:dlq:";

#[derive(Clone)]
pub struct RetryQueue {
    redis: ConnectionManager,
    db: PgPool,
    config: WorkerConfig,
}

impl RetryQueue {
    pub fn new(redis: ConnectionManager, db: PgPool, config: WorkerConfig) -> Self {
        Self { redis, db, config }
    }

    /// Enqueue a new job to run immediately.
    pub async fn enqueue(
        &self,
        queue: &str,
        kind: &str,
        payload: Value,
    ) -> WorkerResult<String> {
        self.enqueue_delayed(queue, kind, payload, Duration::ZERO)
            .await
    }

    /// Enqueue with an explicit initial delay (also used for retries).
    pub async fn enqueue_delayed(
        &self,
        queue: &str,
        kind: &str,
        payload: Value,
        delay: Duration,
    ) -> WorkerResult<String> {
        let job = WorkerJob::new(queue, kind, payload, self.config.retry_max_attempts);
        let job_id = job.id.clone();
        self.save_and_schedule(&job, delay).await?;
        Ok(job_id)
    }

    /// Fetch up to `limit` jobs whose scheduled time has passed.
    pub async fn dequeue_due(&self, queue: &str, limit: usize) -> WorkerResult<Vec<WorkerJob>> {
        let schedule_key = format!("{SCHEDULE_PREFIX}{queue}");
        let now = chrono::Utc::now().timestamp_millis();
        let mut conn = self.redis.clone();

        let ids: Vec<String> = redis::cmd("ZRANGEBYSCORE")
            .arg(&schedule_key)
            .arg(0)
            .arg(now)
            .arg("LIMIT")
            .arg(0)
            .arg(limit as i64)
            .query_async(&mut conn)
            .await?;

        let mut jobs = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(job) = self.take_job(&schedule_key, &id).await? {
                jobs.push(job);
            }
        }
        Ok(jobs)
    }

    /// Remove a successfully processed job.
    pub async fn ack(&self, job_id: &str) -> WorkerResult<()> {
        let mut conn = self.redis.clone();
        let _: () = redis::cmd("DEL")
            .arg(format!("{JOB_PREFIX}{job_id}"))
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    /// Record failure — retry with backoff or move to dead-letter queue.
    pub async fn nack(&self, mut job: WorkerJob, err: &str) -> WorkerResult<()> {
        job.attempts += 1;
        job.last_error = Some(err.to_string());

        if job.attempts >= job.max_attempts {
            warn!(
                job_id = %job.id,
                queue = %job.queue,
                kind = %job.kind,
                attempts = job.attempts,
                error = %err,
                "worker job moved to dead-letter queue"
            );
            self.move_to_dlq(&job).await?;
            self.ack(&job.id).await?;
        } else {
            let delay = exponential_backoff(
                job.attempts,
                self.config.retry_base_ms,
                self.config.retry_max_ms,
            );
            tracing::info!(
                job_id = %job.id,
                queue = %job.queue,
                kind = %job.kind,
                attempts = job.attempts,
                delay_ms = delay.as_millis(),
                "worker job scheduled for retry"
            );
            self.save_and_schedule(&job, delay).await?;
        }
        Ok(())
    }

    /// Peek at recent dead-lettered jobs (newest first).
    pub async fn list_dlq(&self, queue: &str, limit: usize) -> WorkerResult<Vec<WorkerJob>> {
        let key = format!("{DLQ_PREFIX}{queue}");
        let mut conn = self.redis.clone();
        let entries: Vec<String> = redis::cmd("LRANGE")
            .arg(&key)
            .arg(0)
            .arg((limit as i64).saturating_sub(1))
            .query_async(&mut conn)
            .await?;

        entries
            .into_iter()
            .map(|e| serde_json::from_str(&e).map_err(WorkerError::from))
            .collect()
    }

    /// Re-queue a dead-lettered job for manual replay (resets attempts).
    pub async fn replay_dlq(&self, queue: &str, job_id: &str) -> WorkerResult<bool> {
        let key = format!("{DLQ_PREFIX}{queue}");
        let mut conn = self.redis.clone();
        let entries: Vec<String> = redis::cmd("LRANGE")
            .arg(&key)
            .arg(0)
            .arg(-1)
            .query_async(&mut conn)
            .await?;

        for (idx, entry) in entries.iter().enumerate() {
            let job: WorkerJob = serde_json::from_str(entry)?;
            if job.id == job_id {
                let _: () = redis::cmd("LREM")
                    .arg(&key)
                    .arg(1)
                    .arg(entry)
                    .query_async(&mut conn)
                    .await?;

                let mut replay = job;
                replay.attempts = 0;
                replay.last_error = None;
                replay.id = Uuid::new_v4().to_string();
                self.save_and_schedule(&replay, Duration::ZERO).await?;
                return Ok(true);
            }
            let _ = idx;
        }
        Ok(false)
    }

    async fn save_and_schedule(&self, job: &WorkerJob, delay: Duration) -> WorkerResult<()> {
        let mut conn = self.redis.clone();
        let job_key = format!("{JOB_PREFIX}{}", job.id);
        let json = serde_json::to_string(job)?;
        let run_at = chrono::Utc::now().timestamp_millis() + delay.as_millis() as i64;
        let schedule_key = format!("{SCHEDULE_PREFIX}{}", job.queue);

        let _: () = redis::cmd("SET")
            .arg(&job_key)
            .arg(&json)
            .query_async(&mut conn)
            .await?;

        let _: () = redis::cmd("ZADD")
            .arg(&schedule_key)
            .arg(run_at)
            .arg(&job.id)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    async fn take_job(&self, schedule_key: &str, job_id: &str) -> WorkerResult<Option<WorkerJob>> {
        let mut conn = self.redis.clone();

        let removed: i64 = redis::cmd("ZREM")
            .arg(schedule_key)
            .arg(job_id)
            .query_async(&mut conn)
            .await?;

        if removed == 0 {
            return Ok(None);
        }

        let job_key = format!("{JOB_PREFIX}{job_id}");
        let json: Option<String> = redis::cmd("GET")
            .arg(&job_key)
            .query_async(&mut conn)
            .await?;

        match json {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    async fn move_to_dlq(&self, job: &WorkerJob) -> WorkerResult<()> {
        let key = format!("{DLQ_PREFIX}{}", job.queue);
        let json = serde_json::to_string(job)?;
        let mut conn = self.redis.clone();

        let _: () = redis::cmd("LPUSH")
            .arg(&key)
            .arg(&json)
            .query_async(&mut conn)
            .await?;

        let _: () = redis::cmd("LTRIM")
            .arg(&key)
            .arg(0)
            .arg(999)
            .query_async(&mut conn)
            .await?;

        if self.config.dlq_audit_enabled {
            if let Err(e) = self.audit_dlq(job).await {
                error!(job_id = %job.id, error = %e, "failed to persist dead-letter audit row");
            }
        }

        Ok(())
    }

    async fn audit_dlq(&self, job: &WorkerJob) -> WorkerResult<()> {
        sqlx::query(
            r#"
            INSERT INTO worker_dead_letters (job_id, queue_name, job_kind, payload, attempts, last_error)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(&job.id)
        .bind(&job.queue)
        .bind(&job.kind)
        .bind(&job.payload)
        .bind(job.attempts as i32)
        .bind(&job.last_error)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::workers::job::kinds;

    #[test]
    fn worker_job_round_trip_json() {
        let job = WorkerJob::new(
            "deposit",
            kinds::DEPOSIT_RECORD,
            serde_json::json!({ "tx_hash": "0xabc" }),
            5,
        );
        let json = serde_json::to_string(&job).unwrap();
        let parsed: WorkerJob = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kind, kinds::DEPOSIT_RECORD);
        assert_eq!(parsed.max_attempts, 5);
    }
}
