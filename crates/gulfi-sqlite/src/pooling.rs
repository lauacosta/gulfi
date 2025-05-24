use std::{sync::Arc, time::Duration};

use crossbeam::queue::ArrayQueue;
use rusqlite::Connection;
use tokio::{
    sync::{AcquireError, OwnedSemaphorePermit, Semaphore, TryAcquireError},
    time::timeout,
};

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Timeout acquiring connection")]
    Timeout,
    #[error("Failed to acquire semaphore: {0}")]
    Acquire(#[from] AcquireError),
    #[error("Failed to try acquire semaphore: {0}")]
    TryAcquire(#[from] TryAcquireError),
}

#[derive(Clone, Debug)]
pub struct ConnectionPool {
    inner: Arc<ArrayQueue<Connection>>,
}

#[derive(Debug)]
pub struct ConnectionHandle {
    connection: Option<Connection>,
    inner: Arc<ArrayQueue<Connection>>,
}

impl ConnectionPool {
    /// Creates a new connection pool with the specified capacity
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of connections in the pool
    /// * `conn_fn` - Function to create new database connections
    ///
    /// # Example
    /// ``` rust
    /// use gulfi_sqlite::pooling::ConnectionPool;
    /// use rusqlite::Connection;
    ///
    /// let pool = ConnectionPool::new(6, || {
    ///     Connection::open(":memory:")
    /// }).unwrap();
    /// ```
    pub fn new<F>(capacity: usize, conn_fn: F) -> Result<Self, PoolError>
    where
        F: Fn() -> Result<Connection, rusqlite::Error>,
    {
        if capacity == 0 {
            return Err(PoolError::Database(rusqlite::Error::InvalidParameterName(
                "Capacity must be greater than 0".to_owned(),
            )));
        }

        let inner = Arc::new(ArrayQueue::new(capacity));

        for _ in 0..capacity {
            let conn = conn_fn()?;

            inner.push(conn).map_err(|_| {
                PoolError::Database(rusqlite::Error::InvalidParameterName(
                    "Queue capacity exceded".to_owned(),
                ))
            })?;
        }

        Ok(Self { inner })
    }

    pub fn try_get(&self) -> Option<ConnectionHandle> {
        self.inner.pop().map(|connection| ConnectionHandle {
            connection: Some(connection),
            inner: self.inner.clone(),
        })
    }

    /// Get the capacity of the pool
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Checks if all connections are ready
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// Get the current number of available connections
    pub fn available(&self) -> usize {
        self.inner.len()
    }

    /// Check if the pool is empty (no connections available)
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Close the pool and all connections
    pub fn close(&self) -> Vec<Result<(), (Connection, rusqlite::Error)>> {
        let mut result = vec![];
        while let Some(conn) = self.inner.pop() {
            result.push(conn.close());
        }
        result
    }

    /// Check if the pool might be corrupted
    /// This can happen if connections were lost during drop failures
    pub fn is_corrupted(&self) -> bool {
        self.available() < self.capacity()
    }

    /// Get detailed corruption info
    pub fn corruption_info(&self) -> Option<CorruptionInfo> {
        // FIX: This is logic is wrong. I need a way to know if I lost a connection.
        if self.is_corrupted() {
            let available = self.available();
            let capacity = self.capacity();

            Some(CorruptionInfo {
                expected_connections: capacity,
                actual_connections: available,
                missing_connections: capacity.saturating_sub(available),
            })
        } else {
            None
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            capacity: self.capacity(),
            available: self.available(),
            in_use: self.capacity() - self.available(),
        }
    }
}

impl AsRef<Connection> for ConnectionHandle {
    fn as_ref(&self) -> &Connection {
        self
    }
}

impl core::ops::Deref for ConnectionHandle {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        self.connection.as_ref().expect("A")
    }
}

impl core::ops::DerefMut for ConnectionHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.connection.as_mut().expect("A")
    }
}

impl core::borrow::Borrow<Connection> for ConnectionHandle {
    fn borrow(&self) -> &Connection {
        self
    }
}

impl core::borrow::BorrowMut<Connection> for ConnectionHandle {
    fn borrow_mut(&mut self) -> &mut Connection {
        &mut *self
    }
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        let conn = self.connection.take().expect("should not be None");
        self.inner
            .push(conn)
            .expect("Shouldn't surpass queue capacity");
    }
}

#[derive(Clone, Debug)]
pub struct AsyncConnectionPool {
    pool: ConnectionPool,
    semaphore: Arc<Semaphore>,
}

#[derive(Debug)]
pub struct AsyncConnectionHandle {
    connection: ConnectionHandle,
    _permit: OwnedSemaphorePermit,
}

impl AsyncConnectionPool {
    pub fn new<F>(capacity: usize, conn_fn: F) -> Result<Self, PoolError>
    where
        F: Fn() -> Result<Connection, rusqlite::Error>,
    {
        let pool = ConnectionPool::new(capacity, conn_fn)?;
        let semaphore = Arc::new(Semaphore::new(capacity));

        Ok(Self { pool, semaphore })
    }

    /// Get the capacity of the pool
    pub fn capacity(&self) -> usize {
        self.pool.capacity()
    }

    /// Checks if all connections are ready
    pub fn is_full(&self) -> bool {
        self.pool.is_full()
    }

    /// Get the current number of available connections
    pub fn available(&self) -> usize {
        self.pool.available()
    }

    /// Asynchronously acquires a connection from the pool.
    ///
    /// Waits until a permit is available via the semaphore. This method yields the task,
    /// allowing other tasks to progress while waiting.
    ///
    /// # Errors
    /// Returns an error if the semaphore is closed or the task is cancelled while waiting.
    pub async fn acquire(&self) -> Result<AsyncConnectionHandle, PoolError> {
        let _permit = Semaphore::acquire_owned(self.semaphore.clone()).await?;
        let conn = self.pool.try_get().expect("Permit guarantees availability");

        Ok(AsyncConnectionHandle {
            connection: conn,
            _permit,
        })
    }

    /// Acquire a connection with a timeout
    pub async fn acquire_timeout(
        &self,
        duration: Duration,
    ) -> Result<AsyncConnectionHandle, PoolError> {
        timeout(duration, self.acquire())
            .await
            .map_err(|_| PoolError::Timeout)?
    }

    /// Attempts to acquire a connection without waiting.
    ///
    /// If no permits are available, returns an error immediately.
    /// Panics if a permit is acquired but no connection is available in the pool.
    /// This indicates internal pool corruption.
    ///
    /// # Errors
    /// Returns an error if the semaphore has no available permits at the moment.
    pub async fn try_acquire(&self) -> Result<AsyncConnectionHandle, PoolError> {
        let _permit = Semaphore::try_acquire_owned(self.semaphore.clone())?;
        let conn = self.pool.try_get().expect("Permit guarantees availability");

        Ok(AsyncConnectionHandle {
            connection: conn,
            _permit,
        })
    }

    /// Close the pool and all connections
    pub fn close(&self) -> Vec<Result<(), (Connection, rusqlite::Error)>> {
        self.semaphore.close();
        self.pool.close()
    }

    /// Returns true if the semaphore is closed
    pub fn is_closed(&self) -> bool {
        self.semaphore.is_closed()
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        self.pool.stats()
    }

    /// Get detailed corruption info
    pub fn corruption_info(&self) -> Option<CorruptionInfo> {
        self.pool.corruption_info()
    }
}

impl AsMut<Connection> for AsyncConnectionHandle {
    fn as_mut(&mut self) -> &mut Connection {
        self
    }
}

impl AsRef<Connection> for AsyncConnectionHandle {
    fn as_ref(&self) -> &Connection {
        self
    }
}

impl AsRef<ConnectionHandle> for AsyncConnectionHandle {
    fn as_ref(&self) -> &ConnectionHandle {
        self
    }
}

impl core::ops::Deref for AsyncConnectionHandle {
    type Target = ConnectionHandle;
    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl core::ops::DerefMut for AsyncConnectionHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

impl core::borrow::Borrow<Connection> for AsyncConnectionHandle {
    fn borrow(&self) -> &Connection {
        self
    }
}

impl core::borrow::BorrowMut<Connection> for AsyncConnectionHandle {
    fn borrow_mut(&mut self) -> &mut Connection {
        &mut *self
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub capacity: usize,
    pub available: usize,
    pub in_use: usize,
}

#[derive(Debug, Clone)]
pub struct CorruptionInfo {
    pub expected_connections: usize,
    pub actual_connections: usize,
    pub missing_connections: usize,
}
