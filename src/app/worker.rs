//! Bounded background work coordination for UI-owned generations.

use std::{
    collections::VecDeque,
    io,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

/// Fixed number of background workers.
pub const WORKER_COUNT: usize = 2;
/// Maximum number of waiting requests and waiting completions.
pub const QUEUE_CAPACITY: usize = 8;
/// Maximum bytes owned by waiting/running requests and queued successful outputs.
pub const MAX_IN_FLIGHT_BYTES: usize = 64 * 1024 * 1024;

/// A rollover-safe identifier for one application-state generation.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Generation {
    epoch: u64,
    sequence: u64,
}

impl Generation {
    /// Constructs a generation. Primarily useful for restoring or testing a clock boundary.
    #[must_use]
    pub const fn from_parts(epoch: u64, sequence: u64) -> Self {
        Self { epoch, sequence }
    }

    fn next(self) -> Option<Self> {
        if self.sequence == u64::MAX {
            self.epoch
                .checked_add(1)
                .map(|epoch| Self { epoch, sequence: 0 })
        } else {
            Some(Self {
                epoch: self.epoch,
                sequence: self.sequence + 1,
            })
        }
    }
}

/// The generation clock exhausted both of its 64-bit components.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("worker generation counter is exhausted")]
pub struct GenerationExhausted;

/// Cooperative cancellation signal supplied to every task.
#[derive(Clone, Debug)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Whether this generation was superseded or the coordinator is shutting down.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire) || self.shutdown.load(Ordering::Acquire)
    }

    /// A cheap checkpoint for loops and boundaries around expensive operations.
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::Cancelled`] after generation cancellation or shutdown.
    pub fn checkpoint<E>(&self) -> Result<(), TaskError<E>> {
        if self.is_cancelled() {
            Err(TaskError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// Typed outcome from a worker task.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskError<E> {
    /// The generation was superseded or shutdown began.
    #[error("worker task was cancelled")]
    Cancelled,
    /// The operation, such as image decoding, rejected its input.
    #[error("worker task failed: {0}")]
    Decode(E),
    /// Task code panicked; the worker caught the unwind and remained available.
    #[error("worker task panicked")]
    Panicked,
}

/// Immediate rejection from the nonblocking submission path.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SubmitError {
    /// Eight requests are already waiting.
    #[error("worker request queue is full (capacity {QUEUE_CAPACITY})")]
    QueueFull,
    /// Accepting the input would exceed the aggregate input/output byte budget.
    #[error(
        "worker byte budget exceeded: {in_flight} bytes in flight plus {requested} bytes exceeds {limit}"
    )]
    ByteBudgetExceeded {
        /// Bytes currently charged to queued and running work.
        in_flight: usize,
        /// Bytes requested by this submission.
        requested: usize,
        /// Fixed inclusive budget.
        limit: usize,
    },
    /// The caller submitted against a generation that is no longer current.
    #[error("worker request belongs to a stale generation")]
    StaleGeneration,
    /// Shutdown has started and no more work is accepted.
    #[error("worker coordinator is shut down")]
    ShutDown,
}

/// Completion-channel state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReceiveError {
    /// The coordinator has shut down and no completion remains.
    #[error("worker coordinator disconnected")]
    Disconnected,
}

/// One current-generation task completion.
#[derive(Debug)]
pub struct Completion<O, E> {
    /// Generation that owned the task.
    pub generation: Generation,
    /// Successful output or typed task failure.
    pub result: Result<O, TaskError<E>>,
}

/// Exact coordinator counters and instantaneous resource usage.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkerStats {
    /// Requests accepted since construction.
    pub accepted: u64,
    /// Requests rejected since construction.
    pub rejected: u64,
    /// Accepted requests that reached a terminal state.
    pub completed: u64,
    /// Completed requests cancelled before publishing a result.
    pub cancelled: u64,
    /// Published or dropped task panics.
    pub panicked: u64,
    /// Published or dropped typed decode failures.
    pub decode_errors: u64,
    /// Completions discarded due to capacity, budget, or staleness.
    pub dropped_completions: u64,
    /// Successful completions discarded because retaining their output would exceed the budget.
    pub oversized_completions: u64,
    /// Total bytes charged to request inputs and queued successful outputs.
    pub in_flight_bytes: usize,
    /// Bytes charged to queued and running request inputs.
    pub input_bytes: usize,
    /// Bytes charged to successful outputs waiting in the completion queue.
    pub completion_bytes: usize,
    /// Requests waiting for a worker.
    pub queued: usize,
    /// Results waiting for the owner to receive them.
    pub pending_completions: usize,
    /// Worker threads that have not exited.
    pub live_workers: usize,
}

struct Job<I> {
    generation: Generation,
    token: CancellationToken,
    input: I,
    bytes: usize,
}

struct Inner<I, O, E> {
    requests: VecDeque<Job<I>>,
    completions: VecDeque<PendingCompletion<O, E>>,
    stats: WorkerStats,
    shutdown: bool,
}

struct PendingCompletion<O, E> {
    completion: Completion<O, E>,
    output_bytes: usize,
}

struct Shared<I, O, E> {
    inner: Mutex<Inner<I, O, E>>,
    ready: Condvar,
    shutdown: Arc<AtomicBool>,
}

struct Current {
    generation: Generation,
    cancelled: Arc<AtomicBool>,
}

type Processor<I, O, E> = dyn Fn(I, &CancellationToken) -> Result<O, TaskError<E>> + Send + Sync;
type OutputSize<O> = dyn Fn(&O) -> usize + Send + Sync;

/// A fixed-size, bounded worker coordinator reusable by decode, parse, layout, and search jobs.
///
/// Processors must divide expensive work into bounded units and call
/// [`CancellationToken::checkpoint`] between units and around bounded blocking
/// operations. Rust threads cannot terminate arbitrary blocking processor code;
/// shutdown cooperatively cancels and then joins every worker, so a processor
/// that ignores this contract can prevent shutdown from returning.
pub struct WorkerCoordinator<I, O, E> {
    shared: Arc<Shared<I, O, E>>,
    current: Mutex<Current>,
    workers: Vec<JoinHandle<()>>,
}

impl<I, O, E> std::fmt::Debug for WorkerCoordinator<I, O, E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkerCoordinator")
            .field("workers", &self.workers.len())
            .finish_non_exhaustive()
    }
}

impl<I, O, E> WorkerCoordinator<I, O, E>
where
    I: Send + 'static,
    O: Send + 'static,
    E: Send + 'static,
{
    /// Starts two workers at generation zero.
    ///
    /// `output_size` must return the complete owned byte size retained by a
    /// successful output. It runs once before that output can be queued.
    ///
    /// # Errors
    ///
    /// Returns an operating-system error if a worker thread cannot be spawned.
    pub fn new<F, S>(processor: F, output_size: S) -> io::Result<Self>
    where
        F: Fn(I, &CancellationToken) -> Result<O, TaskError<E>> + Send + Sync + 'static,
        S: Fn(&O) -> usize + Send + Sync + 'static,
    {
        Self::with_generation(Generation::default(), processor, output_size)
    }

    /// Starts two workers at an explicit generation clock value.
    ///
    /// # Errors
    ///
    /// Returns an operating-system error if a worker thread cannot be spawned.
    pub fn with_generation<F, S>(
        generation: Generation,
        processor: F,
        output_size: S,
    ) -> io::Result<Self>
    where
        F: Fn(I, &CancellationToken) -> Result<O, TaskError<E>> + Send + Sync + 'static,
        S: Fn(&O) -> usize + Send + Sync + 'static,
    {
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                requests: VecDeque::with_capacity(QUEUE_CAPACITY),
                completions: VecDeque::with_capacity(QUEUE_CAPACITY),
                stats: WorkerStats::default(),
                shutdown: false,
            }),
            ready: Condvar::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
        });
        let processor: Arc<Processor<I, O, E>> = Arc::new(processor);
        let output_size: Arc<OutputSize<O>> = Arc::new(output_size);
        let mut workers = Vec::with_capacity(WORKER_COUNT);

        for index in 0..WORKER_COUNT {
            let worker_shared = Arc::clone(&shared);
            let worker_processor = Arc::clone(&processor);
            let worker_output_size = Arc::clone(&output_size);
            match thread::Builder::new()
                .name(format!("termleaf-worker-{index}"))
                .spawn(move || {
                    worker_loop(&worker_shared, &worker_processor, &worker_output_size);
                }) {
                Ok(worker) => workers.push(worker),
                Err(error) => {
                    stop_shared(&shared);
                    for worker in workers {
                        let _ = worker.join();
                    }
                    return Err(error);
                }
            }
        }

        lock(&shared.inner).stats.live_workers = WORKER_COUNT;
        Ok(Self {
            shared,
            current: Mutex::new(Current {
                generation,
                cancelled: Arc::new(AtomicBool::new(false)),
            }),
            workers,
        })
    }

    /// The generation against which new work should be submitted.
    #[must_use]
    pub fn generation(&self) -> Generation {
        lock(&self.current).generation
    }

    /// Cancels all older work and advances to a distinct generation.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationExhausted`] only after both 64-bit components are exhausted.
    pub fn next_generation(&self) -> Result<Generation, GenerationExhausted> {
        let mut current = lock(&self.current);
        let next = current.generation.next().ok_or(GenerationExhausted)?;
        current.cancelled.store(true, Ordering::Release);
        current.generation = next;
        current.cancelled = Arc::new(AtomicBool::new(false));

        let mut inner = lock(&self.shared.inner);
        let mut retained = VecDeque::with_capacity(QUEUE_CAPACITY);
        while let Some(job) = inner.requests.pop_front() {
            if job.generation == next {
                retained.push_back(job);
            } else {
                finish_cancelled(&mut inner.stats, job.bytes);
            }
        }
        inner.requests = retained;
        let stale = inner.completions.len() as u64;
        while let Some(completion) = inner.completions.pop_front() {
            release_output(&mut inner.stats, completion.output_bytes);
        }
        inner.stats.dropped_completions = inner.stats.dropped_completions.saturating_add(stale);
        inner.stats.queued = inner.requests.len();
        inner.stats.pending_completions = 0;
        Ok(next)
    }

    /// Attempts to enqueue work without waiting for queue space or byte budget.
    ///
    /// `bytes` must be the complete input ownership charged to this job.
    ///
    /// # Errors
    ///
    /// Returns a typed rejection when shutdown has begun, the generation is stale,
    /// the request queue is full, or the aggregate byte budget would be exceeded.
    pub fn try_submit(
        &self,
        generation: Generation,
        input: I,
        bytes: usize,
    ) -> Result<(), SubmitError> {
        let current = lock(&self.current);
        if generation != current.generation {
            lock(&self.shared.inner).stats.rejected += 1;
            return Err(SubmitError::StaleGeneration);
        }

        let mut inner = lock(&self.shared.inner);
        if inner.shutdown {
            inner.stats.rejected += 1;
            return Err(SubmitError::ShutDown);
        }
        if inner.requests.len() == QUEUE_CAPACITY {
            inner.stats.rejected += 1;
            return Err(SubmitError::QueueFull);
        }
        let Some(total) = inner.stats.in_flight_bytes.checked_add(bytes) else {
            inner.stats.rejected += 1;
            return Err(SubmitError::ByteBudgetExceeded {
                in_flight: inner.stats.in_flight_bytes,
                requested: bytes,
                limit: MAX_IN_FLIGHT_BYTES,
            });
        };
        if total > MAX_IN_FLIGHT_BYTES {
            inner.stats.rejected += 1;
            return Err(SubmitError::ByteBudgetExceeded {
                in_flight: inner.stats.in_flight_bytes,
                requested: bytes,
                limit: MAX_IN_FLIGHT_BYTES,
            });
        }

        inner.requests.push_back(Job {
            generation,
            token: CancellationToken {
                cancelled: Arc::clone(&current.cancelled),
                shutdown: Arc::clone(&self.shared.shutdown),
            },
            input,
            bytes,
        });
        inner.stats.accepted += 1;
        inner.stats.in_flight_bytes = total;
        inner.stats.input_bytes += bytes;
        inner.stats.queued = inner.requests.len();
        drop(inner);
        drop(current);
        self.shared.ready.notify_one();
        Ok(())
    }

    /// Removes one completion, discarding any stale generation defensively.
    ///
    /// # Errors
    ///
    /// Returns [`ReceiveError::Disconnected`] after shutdown once the queue is empty.
    pub fn try_recv(&self) -> Result<Option<Completion<O, E>>, ReceiveError> {
        let generation = lock(&self.current).generation;
        let mut inner = lock(&self.shared.inner);
        while let Some(pending) = inner.completions.pop_front() {
            release_output(&mut inner.stats, pending.output_bytes);
            inner.stats.pending_completions = inner.completions.len();
            if pending.completion.generation == generation {
                return Ok(Some(pending.completion));
            }
            inner.stats.dropped_completions += 1;
        }
        if inner.shutdown {
            Err(ReceiveError::Disconnected)
        } else {
            Ok(None)
        }
    }

    /// A consistent accounting snapshot.
    #[must_use]
    pub fn stats(&self) -> WorkerStats {
        lock(&self.shared.inner).stats
    }

    /// Cooperatively requests cancellation without waiting for workers.
    ///
    /// Processor code must follow the type's checkpoint contract; arbitrary
    /// blocking code cannot be forcibly terminated by this coordinator. This
    /// operation is idempotent and safe to call before terminal restoration.
    pub fn request_shutdown(&self) {
        lock(&self.current).cancelled.store(true, Ordering::Release);
        stop_shared(&self.shared);
    }

    /// Joins every worker after cancellation has been requested.
    /// Calling this repeatedly is harmless.
    pub fn join_workers(&mut self) {
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }

    /// Cooperatively cancels all work and joins every worker.
    pub fn shutdown(&mut self) {
        self.request_shutdown();
        self.join_workers();
    }
}

impl<I, O, E> Drop for WorkerCoordinator<I, O, E> {
    fn drop(&mut self) {
        lock(&self.current).cancelled.store(true, Ordering::Release);
        stop_shared(&self.shared);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn worker_loop<I, O, E>(
    shared: &Arc<Shared<I, O, E>>,
    processor: &Arc<Processor<I, O, E>>,
    output_size: &Arc<OutputSize<O>>,
) where
    I: Send + 'static,
    O: Send + 'static,
    E: Send + 'static,
{
    loop {
        let job = {
            let mut inner = lock(&shared.inner);
            while inner.requests.is_empty() && !inner.shutdown {
                inner = shared
                    .ready
                    .wait(inner)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            if inner.shutdown {
                break;
            }
            let job = inner.requests.pop_front().expect("non-empty queue");
            inner.stats.queued = inner.requests.len();
            job
        };

        if job.token.is_cancelled() {
            let mut inner = lock(&shared.inner);
            finish_cancelled(&mut inner.stats, job.bytes);
            continue;
        }

        let (outcome, output_bytes) = catch_unwind(AssertUnwindSafe(|| {
            let outcome = processor(job.input, &job.token);
            let output_bytes = outcome.as_ref().map_or(0, |output| output_size(output));
            (outcome, output_bytes)
        }))
        .unwrap_or((Err(TaskError::Panicked), 0));
        let mut inner = lock(&shared.inner);
        inner.stats.completed += 1;
        inner.stats.in_flight_bytes = inner.stats.in_flight_bytes.saturating_sub(job.bytes);
        inner.stats.input_bytes = inner.stats.input_bytes.saturating_sub(job.bytes);

        if job.token.is_cancelled() || matches!(outcome, Err(TaskError::Cancelled)) {
            inner.stats.cancelled += 1;
            continue;
        }
        match &outcome {
            Err(TaskError::Panicked) => inner.stats.panicked += 1,
            Err(TaskError::Decode(_)) => inner.stats.decode_errors += 1,
            Err(TaskError::Cancelled) | Ok(_) => {}
        }
        if inner.completions.len() == QUEUE_CAPACITY {
            inner.stats.dropped_completions += 1;
        } else if output_bytes > MAX_IN_FLIGHT_BYTES
            || inner
                .stats
                .in_flight_bytes
                .checked_add(output_bytes)
                .is_none_or(|total| total > MAX_IN_FLIGHT_BYTES)
        {
            inner.stats.dropped_completions += 1;
            inner.stats.oversized_completions += 1;
        } else {
            inner.stats.in_flight_bytes += output_bytes;
            inner.stats.completion_bytes += output_bytes;
            inner.completions.push_back(PendingCompletion {
                completion: Completion {
                    generation: job.generation,
                    result: outcome,
                },
                output_bytes,
            });
            inner.stats.pending_completions = inner.completions.len();
        }
    }

    let mut inner = lock(&shared.inner);
    inner.stats.live_workers = inner.stats.live_workers.saturating_sub(1);
}

fn stop_shared<I, O, E>(shared: &Shared<I, O, E>) {
    shared.shutdown.store(true, Ordering::Release);
    let mut inner = lock(&shared.inner);
    inner.shutdown = true;
    while let Some(job) = inner.requests.pop_front() {
        finish_cancelled(&mut inner.stats, job.bytes);
    }
    let dropped = inner.completions.len() as u64;
    while let Some(completion) = inner.completions.pop_front() {
        release_output(&mut inner.stats, completion.output_bytes);
    }
    inner.stats.dropped_completions = inner.stats.dropped_completions.saturating_add(dropped);
    inner.stats.queued = 0;
    inner.stats.pending_completions = 0;
    drop(inner);
    shared.ready.notify_all();
}

fn finish_cancelled(stats: &mut WorkerStats, bytes: usize) {
    stats.completed += 1;
    stats.cancelled += 1;
    stats.in_flight_bytes = stats.in_flight_bytes.saturating_sub(bytes);
    stats.input_bytes = stats.input_bytes.saturating_sub(bytes);
}

fn release_output(stats: &mut WorkerStats, bytes: usize) {
    stats.in_flight_bytes = stats.in_flight_bytes.saturating_sub(bytes);
    stats.completion_bytes = stats.completion_bytes.saturating_sub(bytes);
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Barrier, atomic::AtomicUsize},
        time::{Duration, Instant},
    };

    use super::*;

    const DEADLINE: Duration = Duration::from_secs(2);

    fn wait_for(mut predicate: impl FnMut() -> bool) {
        let deadline = Instant::now() + DEADLINE;
        while !predicate() {
            assert!(Instant::now() < deadline, "worker test deadline elapsed");
            thread::yield_now();
        }
    }

    fn identity_pool() -> WorkerCoordinator<u8, u8, &'static str> {
        WorkerCoordinator::new(
            |value, token| {
                token.checkpoint()?;
                Ok(value)
            },
            |_| 1,
        )
        .expect("workers start")
    }

    #[test]
    fn con_001_fixed_limits_and_byte_budget_reject_nonblocking() {
        assert_eq!(WORKER_COUNT, 2);
        assert_eq!(QUEUE_CAPACITY, 8);
        assert_eq!(MAX_IN_FLIGHT_BYTES, 64 * 1024 * 1024);

        let gate = Arc::new(Barrier::new(WORKER_COUNT + 1));
        let release = Arc::new(Barrier::new(WORKER_COUNT + 1));
        let mut pool = WorkerCoordinator::<u8, u8, ()>::new(
            {
                let gate = Arc::clone(&gate);
                let release = Arc::clone(&release);
                move |value, _| {
                    if value >= 100 {
                        gate.wait();
                        release.wait();
                    }
                    Ok(value)
                }
            },
            |_| 1,
        )
        .expect("workers start");
        let generation = pool.generation();
        let worker_count = u8::try_from(WORKER_COUNT).expect("worker count fits u8");
        let queue_capacity = u8::try_from(QUEUE_CAPACITY).expect("queue capacity fits u8");
        for value in 0..worker_count {
            pool.try_submit(generation, value + 100, 1)
                .expect("running job accepted");
        }
        gate.wait();
        for value in 0..queue_capacity {
            pool.try_submit(generation, value, 1)
                .expect("queued job accepted");
        }
        assert_eq!(
            pool.try_submit(generation, 99, 1),
            Err(SubmitError::QueueFull)
        );
        assert_eq!(pool.stats().queued, QUEUE_CAPACITY);
        release.wait();
        wait_for(|| pool.stats().completed == (WORKER_COUNT + QUEUE_CAPACITY) as u64);
        pool.shutdown();

        let pool = identity_pool();
        let generation = pool.generation();
        assert_eq!(
            pool.try_submit(generation, 1, MAX_IN_FLIGHT_BYTES + 1),
            Err(SubmitError::ByteBudgetExceeded {
                in_flight: 0,
                requested: MAX_IN_FLIGHT_BYTES + 1,
                limit: MAX_IN_FLIGHT_BYTES,
            })
        );
    }

    #[test]
    fn con_002_and_prop_009_old_completion_cannot_replace_new() {
        let old_entered = Arc::new(Barrier::new(2));
        let old_release = Arc::new(Barrier::new(2));
        let pool = WorkerCoordinator::<u8, u8, ()>::new(
            {
                let old_entered = Arc::clone(&old_entered);
                let old_release = Arc::clone(&old_release);
                move |value, token| {
                    if value == 1 {
                        old_entered.wait();
                        old_release.wait();
                    }
                    token.checkpoint()?;
                    Ok(value)
                }
            },
            |_| 1,
        )
        .expect("workers start");

        let old = pool.generation();
        pool.try_submit(old, 1, 1).expect("old work accepted");
        old_entered.wait();
        let new = pool.next_generation().expect("generation advances");
        pool.try_submit(new, 2, 1).expect("new work accepted");
        wait_for(|| pool.stats().completed >= 1);
        let current = loop {
            if let Some(completion) = pool.try_recv().expect("still connected") {
                break completion;
            }
            thread::yield_now();
        };
        assert_eq!(current.generation, new);
        assert_eq!(current.result, Ok(2));
        old_release.wait();
        wait_for(|| pool.stats().completed == 2);
        assert!(pool.try_recv().expect("still connected").is_none());
    }

    #[test]
    fn con_003_navigation_cancels_queued_and_running_work() {
        let entered = Arc::new(Barrier::new(WORKER_COUNT + 1));
        let release = Arc::new(Barrier::new(WORKER_COUNT + 1));
        let pool = WorkerCoordinator::<u8, u8, ()>::new(
            {
                let entered = Arc::clone(&entered);
                let release = Arc::clone(&release);
                move |value, token| {
                    entered.wait();
                    release.wait();
                    token.checkpoint()?;
                    Ok(value)
                }
            },
            |_| 1,
        )
        .expect("workers start");
        let old = pool.generation();
        for value in 0..4 {
            pool.try_submit(old, value, 10).expect("work accepted");
        }
        entered.wait();
        pool.next_generation().expect("generation advances");
        assert_eq!(pool.stats().queued, 0);
        assert_eq!(pool.stats().in_flight_bytes, 20);
        release.wait();
        wait_for(|| pool.stats().completed == 4);
        let stats = pool.stats();
        assert_eq!(stats.cancelled, 4);
        assert_eq!(stats.in_flight_bytes, 0);
    }

    #[test]
    fn con_004_panic_decode_error_and_disconnect_are_typed() {
        let mut pool = WorkerCoordinator::<u8, u8, &'static str>::new(
            |value, _| match value {
                0 => panic!("injected panic"),
                1 => Err(TaskError::Decode("bad image")),
                _ => Ok(value),
            },
            |_| 1,
        )
        .expect("workers start");
        let generation = pool.generation();
        pool.try_submit(generation, 0, 1)
            .expect("panic task accepted");
        pool.try_submit(generation, 1, 1)
            .expect("decode task accepted");
        wait_for(|| pool.stats().completed == 2);
        let mut outcomes = Vec::new();
        while let Some(completion) = pool.try_recv().expect("still connected") {
            outcomes.push(completion.result);
        }
        assert!(outcomes.contains(&Err(TaskError::Panicked)));
        assert!(outcomes.contains(&Err(TaskError::Decode("bad image"))));
        assert_eq!(pool.stats().panicked, 1);
        assert_eq!(pool.stats().decode_errors, 1);
        pool.shutdown();
        assert!(matches!(pool.try_recv(), Err(ReceiveError::Disconnected)));
        assert_eq!(
            pool.try_submit(generation, 2, 1),
            Err(SubmitError::ShutDown)
        );
    }

    #[test]
    fn con_005_shutdown_cancels_checkpointing_tasks_and_joins_every_worker() {
        let entered = Arc::new(AtomicUsize::new(0));
        let mut pool = WorkerCoordinator::<u8, u8, ()>::new(
            {
                let entered = Arc::clone(&entered);
                move |_, token| {
                    entered.fetch_add(1, Ordering::Release);
                    loop {
                        token.checkpoint()?;
                        thread::yield_now();
                    }
                }
            },
            |_| 1,
        )
        .expect("workers start");
        let generation = pool.generation();
        let worker_count = u8::try_from(WORKER_COUNT).expect("worker count fits u8");
        let total_capacity =
            u8::try_from(WORKER_COUNT + QUEUE_CAPACITY).expect("total capacity fits u8");
        for value in 0..worker_count {
            pool.try_submit(generation, value, 1)
                .expect("work accepted");
        }
        wait_for(|| entered.load(Ordering::Acquire) == WORKER_COUNT);
        for value in worker_count..total_capacity {
            pool.try_submit(generation, value, 1)
                .expect("work accepted");
        }

        let started = Instant::now();
        pool.shutdown();
        assert!(
            started.elapsed() < DEADLINE,
            "cooperative shutdown exceeded bound"
        );
        let stats = pool.stats();
        assert_eq!(stats.live_workers, 0);
        assert_eq!(stats.completed, (WORKER_COUNT + QUEUE_CAPACITY) as u64);
        assert_eq!(stats.cancelled, stats.completed);
        assert_eq!(stats.in_flight_bytes, 0);
    }

    #[test]
    fn con_006_and_008_repeated_cycles_preserve_accounting_and_bounds() {
        let pool = identity_pool();
        let mut accepted = 0_u64;
        let mut rejected = 0_u64;
        for cycle in 0..64_u8 {
            let generation = pool.generation();
            for value in 0..16_u8 {
                match pool.try_submit(generation, value ^ cycle, 1024) {
                    Ok(()) => accepted += 1,
                    Err(SubmitError::QueueFull) => rejected += 1,
                    Err(error) => panic!("unexpected rejection: {error}"),
                }
            }
            pool.next_generation().expect("generation advances");
            let stats = pool.stats();
            assert!(stats.queued <= QUEUE_CAPACITY);
            assert!(stats.pending_completions <= QUEUE_CAPACITY);
            assert!(stats.in_flight_bytes <= MAX_IN_FLIGHT_BYTES);
            assert!(stats.live_workers <= WORKER_COUNT);
        }
        wait_for(|| pool.stats().completed == pool.stats().accepted);
        let stats = pool.stats();
        assert_eq!(stats.accepted, accepted);
        assert_eq!(stats.rejected, rejected);
        assert_eq!(stats.completed, stats.accepted);
        assert_eq!(stats.in_flight_bytes, 0);
    }

    #[test]
    fn con_007_task_runs_without_coordinator_lock_held() {
        let holder = Arc::new(Mutex::new(
            None::<std::sync::Weak<WorkerCoordinator<u8, u8, ()>>>,
        ));
        let pool = Arc::new(
            WorkerCoordinator::<u8, u8, ()>::new(
                {
                    let holder = Arc::clone(&holder);
                    move |value, _| {
                        let pool = lock(&holder)
                            .as_ref()
                            .expect("pool installed")
                            .upgrade()
                            .expect("pool remains alive");
                        let _ = pool.stats();
                        Ok(value)
                    }
                },
                |_| 1,
            )
            .expect("workers start"),
        );
        *lock(&holder) = Some(Arc::downgrade(&pool));
        pool.try_submit(pool.generation(), 1, 1)
            .expect("work accepted");
        wait_for(|| pool.stats().completed == 1);
        assert_eq!(pool.try_recv().expect("connected").unwrap().result, Ok(1));
    }

    #[test]
    fn completion_queue_is_bounded_and_never_blocks_workers() {
        let pool = identity_pool();
        let generation = pool.generation();
        let mut accepted = 0;
        while accepted < 32 {
            match pool.try_submit(generation, accepted, 1) {
                Ok(()) => accepted += 1,
                Err(SubmitError::QueueFull) => thread::yield_now(),
                Err(error) => panic!("unexpected rejection: {error}"),
            }
        }
        wait_for(|| pool.stats().completed == 32);
        let stats = pool.stats();
        assert_eq!(stats.pending_completions, QUEUE_CAPACITY);
        assert_eq!(stats.dropped_completions, 32 - QUEUE_CAPACITY as u64);
    }

    #[test]
    fn successful_output_bytes_remain_charged_until_receive_or_stale_clear() {
        let pool = WorkerCoordinator::<usize, usize, ()>::new(|bytes, _| Ok(bytes), |bytes| *bytes)
            .expect("workers start");
        let generation = pool.generation();
        pool.try_submit(generation, 4096, 16)
            .expect("work accepted");
        wait_for(|| pool.stats().completed == 1);
        let stats = pool.stats();
        assert_eq!(stats.input_bytes, 0);
        assert_eq!(stats.completion_bytes, 4096);
        assert_eq!(stats.in_flight_bytes, 4096);
        assert_eq!(
            pool.try_recv().expect("connected").unwrap().result,
            Ok(4096)
        );
        assert_eq!(pool.stats().completion_bytes, 0);
        assert_eq!(pool.stats().in_flight_bytes, 0);

        let current = pool.generation();
        pool.try_submit(current, 8192, 8).expect("work accepted");
        wait_for(|| pool.stats().completed == 2);
        assert_eq!(pool.stats().completion_bytes, 8192);
        pool.next_generation().expect("generation advances");
        let stats = pool.stats();
        assert_eq!(stats.completion_bytes, 0);
        assert_eq!(stats.in_flight_bytes, 0);
        assert_eq!(stats.dropped_completions, 1);

        let mut shutdown_pool =
            WorkerCoordinator::<usize, usize, ()>::new(|bytes, _| Ok(bytes), |bytes| *bytes)
                .expect("workers start");
        shutdown_pool
            .try_submit(shutdown_pool.generation(), 2048, 1)
            .expect("work accepted");
        wait_for(|| shutdown_pool.stats().completed == 1);
        shutdown_pool.shutdown();
        let stats = shutdown_pool.stats();
        assert_eq!(stats.in_flight_bytes, 0);
        assert_eq!(stats.completion_bytes, 0);
        assert_eq!(stats.pending_completions, 0);
        assert_eq!(stats.dropped_completions, 1);
        assert_eq!(stats.live_workers, 0);
    }

    #[test]
    fn queued_output_consumes_budget_and_oversized_completion_is_dropped() {
        let retained = 40 * 1024 * 1024;
        let pool = WorkerCoordinator::<usize, usize, ()>::new(|bytes, _| Ok(bytes), |bytes| *bytes)
            .expect("workers start");
        let generation = pool.generation();
        pool.try_submit(generation, retained, 1)
            .expect("work accepted");
        wait_for(|| pool.stats().completed == 1);
        assert_eq!(pool.stats().in_flight_bytes, retained);

        let remaining = MAX_IN_FLIGHT_BYTES - retained;
        assert_eq!(
            pool.try_submit(generation, 0, remaining + 1),
            Err(SubmitError::ByteBudgetExceeded {
                in_flight: retained,
                requested: remaining + 1,
                limit: MAX_IN_FLIGHT_BYTES,
            })
        );
        pool.try_submit(generation, 0, remaining)
            .expect("exact total budget is accepted");
        wait_for(|| pool.stats().completed == 2);
        assert_eq!(pool.stats().in_flight_bytes, retained);

        let oversized =
            WorkerCoordinator::<u8, u8, ()>::new(|value, _| Ok(value), |_| MAX_IN_FLIGHT_BYTES + 1)
                .expect("workers start");
        oversized
            .try_submit(oversized.generation(), 1, 1)
            .expect("input is accepted");
        wait_for(|| oversized.stats().completed == 1);
        let stats = oversized.stats();
        assert_eq!(stats.oversized_completions, 1);
        assert_eq!(stats.dropped_completions, 1);
        assert_eq!(stats.pending_completions, 0);
        assert_eq!(stats.in_flight_bytes, 0);
        assert!(oversized.try_recv().expect("connected").is_none());
    }

    #[test]
    fn con_009_checkpoint_and_generation_rollover_cannot_revive_stale_work() {
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let start = Generation::from_parts(7, u64::MAX);
        let pool = WorkerCoordinator::<u8, u8, ()>::with_generation(
            start,
            {
                let entered = Arc::clone(&entered);
                let release = Arc::clone(&release);
                move |value, token| {
                    entered.wait();
                    release.wait();
                    token.checkpoint()?;
                    Ok(value)
                }
            },
            |_| 1,
        )
        .expect("workers start");
        pool.try_submit(start, 1, 1).expect("old work accepted");
        entered.wait();
        let rolled = pool.next_generation().expect("sequence rolls into epoch");
        assert_eq!(rolled, Generation::from_parts(8, 0));
        assert_eq!(
            pool.try_submit(start, 2, 1),
            Err(SubmitError::StaleGeneration)
        );
        release.wait();
        wait_for(|| pool.stats().completed == 1);
        assert_eq!(pool.stats().cancelled, 1);
        assert!(pool.try_recv().expect("connected").is_none());

        let exhausted = WorkerCoordinator::<u8, u8, ()>::with_generation(
            Generation::from_parts(u64::MAX, u64::MAX),
            |value, _| Ok(value),
            |_| 1,
        )
        .expect("workers start");
        assert_eq!(exhausted.next_generation(), Err(GenerationExhausted));
    }
}
