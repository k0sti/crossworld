//! Background task management for LLM operations

use crate::error::{Error, Result};
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{oneshot, watch};

/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    /// Generate a new unique task ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task-{}", self.0)
    }
}

/// Current status of a task
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is waiting to start
    Pending,
    /// Task is currently running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed(String),
    /// Task was cancelled
    Cancelled,
}

impl TaskStatus {
    /// Check if the task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Cancelled
        )
    }

    /// Check if the task is running
    pub fn is_running(&self) -> bool {
        matches!(self, TaskStatus::Running)
    }
}

/// Handle for controlling and monitoring a background task
#[derive(Debug)]
pub struct TaskHandle<T> {
    /// Unique task identifier
    pub id: TaskId,
    /// Cancellation flag
    cancel_flag: Arc<AtomicBool>,
    /// Status receiver
    status_rx: watch::Receiver<TaskStatus>,
    /// Result receiver (one-shot)
    result_rx: Option<oneshot::Receiver<Result<T>>>,
    /// When the task was created
    created_at: Instant,
}

impl<T> TaskHandle<T> {
    /// Get the current status of the task
    pub fn status(&self) -> TaskStatus {
        self.status_rx.borrow().clone()
    }

    /// Check if the task has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    /// Cancel the task
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    /// Wait for the task to complete and get the result
    pub async fn await_result(mut self) -> Result<T> {
        match self.result_rx.take() {
            Some(rx) => rx.await?,
            None => Err(Error::Channel("Result already consumed".to_string())),
        }
    }

    /// Wait for the task to complete with a timeout
    pub async fn await_result_timeout(self, timeout: Duration) -> Result<T> {
        tokio::time::timeout(timeout, self.await_result())
            .await
            .map_err(|_| Error::Timeout(timeout))?
    }

    /// Get elapsed time since task creation
    pub fn elapsed(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Subscribe to status updates
    pub fn subscribe_status(&self) -> watch::Receiver<TaskStatus> {
        self.status_rx.clone()
    }
}

/// Context provided to task functions for cancellation checking
#[derive(Clone)]
pub struct TaskContext {
    cancel_flag: Arc<AtomicBool>,
    status_tx: watch::Sender<TaskStatus>,
}

impl TaskContext {
    /// Check if cancellation has been requested
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    /// Return an error if cancellation was requested
    pub fn check_cancelled(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(Error::Cancelled("Task cancelled".to_string()))
        } else {
            Ok(())
        }
    }

    /// Update the task status (for custom status reporting)
    pub fn set_status(&self, status: TaskStatus) {
        let _ = self.status_tx.send(status);
    }
}

/// Builder for configuring and spawning background tasks
pub struct TaskBuilder {
    /// Optional timeout for the task
    timeout: Option<Duration>,
}

impl TaskBuilder {
    /// Create a new task builder
    pub fn new() -> Self {
        Self { timeout: None }
    }

    /// Set a timeout for the task
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Spawn a task with the given async function
    pub fn spawn<T, F, Fut>(self, f: F) -> TaskHandle<T>
    where
        T: Send + 'static,
        F: FnOnce(TaskContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>> + Send + 'static,
    {
        let id = TaskId::new();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (status_tx, status_rx) = watch::channel(TaskStatus::Pending);
        let (result_tx, result_rx) = oneshot::channel();

        let ctx = TaskContext {
            cancel_flag: cancel_flag.clone(),
            status_tx: status_tx.clone(),
        };

        let timeout = self.timeout;

        tokio::spawn(async move {
            let _ = status_tx.send(TaskStatus::Running);

            let result = if let Some(timeout_duration) = timeout {
                match tokio::time::timeout(timeout_duration, f(ctx)).await {
                    Ok(res) => res,
                    Err(_) => Err(Error::Timeout(timeout_duration)),
                }
            } else {
                f(ctx).await
            };

            let status = match &result {
                Ok(_) => TaskStatus::Completed,
                Err(Error::Cancelled(_)) => TaskStatus::Cancelled,
                Err(e) => TaskStatus::Failed(e.to_string()),
            };

            let _ = status_tx.send(status);
            let _ = result_tx.send(result);
        });

        TaskHandle {
            id,
            cancel_flag,
            status_rx,
            result_rx: Some(result_rx),
            created_at: Instant::now(),
        }
    }
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn a background task with the given async function
pub fn spawn_task<T, F, Fut>(f: F) -> TaskHandle<T>
where
    T: Send + 'static,
    F: FnOnce(TaskContext) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T>> + Send + 'static,
{
    TaskBuilder::new().spawn(f)
}

/// Spawn a background task with a timeout
pub fn spawn_task_with_timeout<T, F, Fut>(timeout: Duration, f: F) -> TaskHandle<T>
where
    T: Send + 'static,
    F: FnOnce(TaskContext) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T>> + Send + 'static,
{
    TaskBuilder::new().with_timeout(timeout).spawn(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_task_success() {
        let handle = spawn_task(|_ctx| async { Ok(42) });

        let result = handle.await_result().await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_spawn_task_failure() {
        let handle =
            spawn_task(|_ctx| async { Err::<(), _>(Error::Provider("test error".into())) });

        let result = handle.await_result().await;
        assert!(matches!(result, Err(Error::Provider(_))));
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let handle: TaskHandle<()> = spawn_task(|ctx| async move {
            loop {
                ctx.check_cancelled()?;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        // Cancel after a short delay
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.cancel();

        let result = handle.await_result().await;
        assert!(matches!(result, Err(Error::Cancelled(_))));
    }

    #[tokio::test]
    async fn test_task_timeout() {
        let handle = spawn_task_with_timeout(Duration::from_millis(50), |_ctx| async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok(())
        });

        let result = handle.await_result().await;
        assert!(matches!(result, Err(Error::Timeout(_))));
    }

    #[tokio::test]
    async fn test_task_status_updates() {
        let handle = spawn_task(|_ctx| async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(())
        });

        // Should transition from Pending to Running to Completed
        let status_rx = handle.subscribe_status();

        // Wait for completion
        let _ = handle.await_result().await;

        // Final status should be Completed
        assert_eq!(*status_rx.borrow(), TaskStatus::Completed);
    }

    #[test]
    fn test_task_id_uniqueness() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }
}
