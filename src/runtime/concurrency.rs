//! Concurrency runtime implementation
//! 
//! Provides goroutines, scheduling, and concurrency primitives for Flux programs.

// Runtime errors are now defined in result.rs
use crate::runtime::{GoroutineId, GoroutineHandle, Channel};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, Condvar, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Goroutine state
#[derive(Debug, Clone, PartialEq)]
pub enum GoroutineState {
    Ready,
    Running,
    Blocked,
    Finished,
}

/// Goroutine context for saving/restoring execution state
#[derive(Debug)]
pub struct Context {
    // Placeholder for CPU registers and stack pointer
    // In a real implementation, this would contain actual register values
    pub stack_pointer: usize,
    pub instruction_pointer: usize,
}

impl Context {
    pub fn new() -> Self {
        Self {
            stack_pointer: 0,
            instruction_pointer: 0,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

/// Stack for goroutine execution
#[derive(Debug)]
pub struct Stack {
    data: Vec<u8>,
    size: usize,
}

impl Stack {
    /// Create a new stack with the specified size
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
            size,
        }
    }
    
    /// Get the stack pointer
    pub fn stack_pointer(&self) -> *mut u8 {
        self.data.as_ptr() as *mut u8
    }
    
    /// Get the stack size
    pub fn size(&self) -> usize {
        self.size
    }
}

/// Goroutine structure
#[derive(Debug)]
pub struct Goroutine {
    pub id: GoroutineId,
    pub stack: Stack,
    pub state: GoroutineState,
    pub context: Context,
    pub function: Option<fn()>,
}

impl Goroutine {
    /// Create a new goroutine
    pub fn new(id: GoroutineId, func: fn()) -> Self {
        Self {
            id,
            stack: Stack::new(8192), // 8KB stack
            state: GoroutineState::Ready,
            context: Context::new(),
            function: Some(func),
        }
    }
    
    /// Run the goroutine function
    pub fn run(&mut self) {
        if let Some(func) = self.function {
            self.state = GoroutineState::Running;
            func();
            self.state = GoroutineState::Finished;
        }
    }
}

/// Scheduler for managing goroutines with round-robin scheduling
pub struct Scheduler {
    goroutines: Arc<Mutex<HashMap<GoroutineId, Goroutine>>>,
    ready_queue: Arc<Mutex<VecDeque<GoroutineId>>>,
    current: Arc<Mutex<Option<GoroutineId>>>,
    next_id: Arc<Mutex<GoroutineId>>,
    running: Arc<Mutex<bool>>,
    /// Condition variable for waking up the scheduler
    scheduler_cv: Arc<Condvar>,
    /// Worker threads for executing goroutines
    worker_threads: Vec<JoinHandle<()>>,
    /// Channel for sending work to workers
    work_sender: mpsc::Sender<WorkItem>,
    work_receiver: Arc<Mutex<mpsc::Receiver<WorkItem>>>,
    /// Statistics
    stats: Arc<Mutex<SchedulerStats>>,
}

/// Work item for the scheduler
#[derive(Debug)]
enum WorkItem {
    Execute(GoroutineId),
    Shutdown,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self::with_worker_count(num_cpus::get())
    }
    
    /// Create a new scheduler with specified number of worker threads
    pub fn with_worker_count(worker_count: usize) -> Self {
        let (work_sender, work_receiver) = mpsc::channel();
        let work_receiver = Arc::new(Mutex::new(work_receiver));
        
        let goroutines = Arc::new(Mutex::new(HashMap::new()));
        let ready_queue = Arc::new(Mutex::new(VecDeque::new()));
        let current = Arc::new(Mutex::new(None));
        let next_id = Arc::new(Mutex::new(1));
        let running = Arc::new(Mutex::new(false));
        let scheduler_cv = Arc::new(Condvar::new());
        let stats = Arc::new(Mutex::new(SchedulerStats::default()));
        
        let mut worker_threads = Vec::new();
        
        // Spawn worker threads
        for worker_id in 0..worker_count {
            let goroutines_clone = Arc::clone(&goroutines);
            let work_receiver_clone = Arc::clone(&work_receiver);
            let stats_clone = Arc::clone(&stats);
            
            let handle = thread::spawn(move || {
                Self::worker_thread(worker_id, goroutines_clone, work_receiver_clone, stats_clone);
            });
            
            worker_threads.push(handle);
        }
        
        Self {
            goroutines,
            ready_queue,
            current,
            next_id,
            running,
            scheduler_cv,
            worker_threads,
            work_sender,
            work_receiver,
            stats,
        }
    }
    
    /// Worker thread function
    fn worker_thread(
        worker_id: usize,
        goroutines: Arc<Mutex<HashMap<GoroutineId, Goroutine>>>,
        work_receiver: Arc<Mutex<mpsc::Receiver<WorkItem>>>,
        stats: Arc<Mutex<SchedulerStats>>,
    ) {
        loop {
            let work_item = {
                let receiver = work_receiver.lock().unwrap();
                receiver.recv()
            };
            
            match work_item {
                Ok(WorkItem::Execute(goroutine_id)) => {
                    // Execute the goroutine
                    let mut goroutines_guard = goroutines.lock().unwrap();
                    if let Some(goroutine) = goroutines_guard.get_mut(&goroutine_id) {
                        let start_time = Instant::now();
                        goroutine.run();
                        let execution_time = start_time.elapsed();
                        
                        // Update statistics
                        let mut stats_guard = stats.lock().unwrap();
                        stats_guard.total_execution_time += execution_time;
                        stats_guard.goroutines_executed += 1;
                        
                        // Remove finished goroutines
                        if goroutine.state == GoroutineState::Finished {
                            goroutines_guard.remove(&goroutine_id);
                            stats_guard.finished_count += 1;
                        }
                    }
                }
                Ok(WorkItem::Shutdown) => {
                    break;
                }
                Err(_) => {
                    // Channel closed, shutdown
                    break;
                }
            }
        }
    }
    
    /// Spawn a new goroutine
    pub fn spawn(&self, func: fn()) -> GoroutineHandle {
        let id = self.add_goroutine(func);
        GoroutineHandle { id }
    }
    
    /// Add a goroutine to the scheduler
    pub fn add_goroutine(&self, func: fn()) -> GoroutineId {
        let id = {
            let mut next_id_guard = self.next_id.lock().unwrap();
            let id = *next_id_guard;
            *next_id_guard += 1;
            id
        };
        
        let goroutine = Goroutine::new(id, func);
        
        {
            let mut goroutines_guard = self.goroutines.lock().unwrap();
            goroutines_guard.insert(id, goroutine);
        }
        
        {
            let mut ready_queue_guard = self.ready_queue.lock().unwrap();
            ready_queue_guard.push_back(id);
        }
        
        // Update statistics
        {
            let mut stats_guard = self.stats.lock().unwrap();
            stats_guard.total_goroutines += 1;
            stats_guard.ready_count += 1;
        }
        
        // Notify scheduler that work is available
        self.scheduler_cv.notify_one();
        
        id
    }
    
    /// Start the scheduler
    pub fn start(&self) {
        {
            let mut running_guard = self.running.lock().unwrap();
            *running_guard = true;
        }
        
        // Main scheduler loop
        loop {
            let should_continue = {
                let running_guard = self.running.lock().unwrap();
                *running_guard
            };
            
            if !should_continue {
                break;
            }
            
            // Get next goroutine to execute
            let next_goroutine = {
                let mut ready_queue_guard = self.ready_queue.lock().unwrap();
                ready_queue_guard.pop_front()
            };
            
            if let Some(goroutine_id) = next_goroutine {
                // Send work to worker thread
                if let Err(_) = self.work_sender.send(WorkItem::Execute(goroutine_id)) {
                    // Channel closed, shutdown
                    break;
                }
                
                // Update current goroutine
                {
                    let mut current_guard = self.current.lock().unwrap();
                    *current_guard = Some(goroutine_id);
                }
                
                // Update statistics
                {
                    let mut stats_guard = self.stats.lock().unwrap();
                    stats_guard.ready_count = stats_guard.ready_count.saturating_sub(1);
                    stats_guard.running_count += 1;
                }
            } else {
                // No work available, wait for notification
                let ready_queue_guard = self.ready_queue.lock().unwrap();
                let _guard = self.scheduler_cv.wait_timeout(ready_queue_guard, Duration::from_millis(10)).unwrap();
            }
        }
    }
    
    /// Stop the scheduler
    pub fn shutdown(&self) {
        {
            let mut running_guard = self.running.lock().unwrap();
            *running_guard = false;
        }
        
        // Send shutdown signal to all worker threads
        for _ in 0..self.worker_threads.len() {
            let _ = self.work_sender.send(WorkItem::Shutdown);
        }
        
        self.scheduler_cv.notify_all();
    }
    
    /// Wait for all worker threads to finish
    pub fn join(self) {
        for handle in self.worker_threads {
            let _ = handle.join();
        }
    }
    
    /// Yield execution to another goroutine
    pub fn yield_now(&self) {
        let current_id = {
            let current_guard = self.current.lock().unwrap();
            *current_guard
        };
        
        if let Some(current_id) = current_id {
            let mut goroutines_guard = self.goroutines.lock().unwrap();
            if let Some(goroutine) = goroutines_guard.get_mut(&current_id) {
                if goroutine.state == GoroutineState::Running {
                    goroutine.state = GoroutineState::Ready;
                    
                    // Add back to ready queue
                    let mut ready_queue_guard = self.ready_queue.lock().unwrap();
                    ready_queue_guard.push_back(current_id);
                    
                    // Update statistics
                    let mut stats_guard = self.stats.lock().unwrap();
                    stats_guard.ready_count += 1;
                    stats_guard.running_count = stats_guard.running_count.saturating_sub(1);
                }
            }
        }
        
        self.scheduler_cv.notify_one();
    }
    
    /// Block the current goroutine
    pub fn block_current(&self) {
        let current_id = {
            let current_guard = self.current.lock().unwrap();
            *current_guard
        };
        
        if let Some(current_id) = current_id {
            let mut goroutines_guard = self.goroutines.lock().unwrap();
            if let Some(goroutine) = goroutines_guard.get_mut(&current_id) {
                goroutine.state = GoroutineState::Blocked;
                
                // Update statistics
                let mut stats_guard = self.stats.lock().unwrap();
                stats_guard.blocked_count += 1;
                stats_guard.running_count = stats_guard.running_count.saturating_sub(1);
            }
        }
    }
    
    /// Unblock a goroutine and make it ready
    pub fn unblock(&self, id: GoroutineId) {
        let mut goroutines_guard = self.goroutines.lock().unwrap();
        if let Some(goroutine) = goroutines_guard.get_mut(&id) {
            if goroutine.state == GoroutineState::Blocked {
                goroutine.state = GoroutineState::Ready;
                
                // Add to ready queue
                let mut ready_queue_guard = self.ready_queue.lock().unwrap();
                ready_queue_guard.push_back(id);
                
                // Update statistics
                let mut stats_guard = self.stats.lock().unwrap();
                stats_guard.blocked_count = stats_guard.blocked_count.saturating_sub(1);
                stats_guard.ready_count += 1;
                
                // Notify scheduler
                self.scheduler_cv.notify_one();
            }
        }
    }
    
    /// Get scheduler statistics
    pub fn stats(&self) -> SchedulerStats {
        let stats_guard = self.stats.lock().unwrap();
        stats_guard.clone()
    }
    
    /// Get the number of active goroutines
    pub fn active_goroutine_count(&self) -> usize {
        let goroutines_guard = self.goroutines.lock().unwrap();
        goroutines_guard.len()
    }
    
    /// Get the number of ready goroutines
    pub fn ready_goroutine_count(&self) -> usize {
        let ready_queue_guard = self.ready_queue.lock().unwrap();
        ready_queue_guard.len()
    }
    
    /// Check if the scheduler is running
    pub fn is_running(&self) -> bool {
        let running_guard = self.running.lock().unwrap();
        *running_guard
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub total_goroutines: usize,
    pub ready_count: usize,
    pub running_count: usize,
    pub blocked_count: usize,
    pub finished_count: usize,
    pub goroutines_executed: usize,
    pub total_execution_time: Duration,
}

impl std::fmt::Display for SchedulerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "Scheduler Statistics:\n\
             Total Goroutines: {}\n\
             Ready: {}\n\
             Running: {}\n\
             Blocked: {}\n\
             Finished: {}\n\
             Executed: {}\n\
             Total Execution Time: {:?}",
            self.total_goroutines,
            self.ready_count,
            self.running_count,
            self.blocked_count,
            self.finished_count,
            self.goroutines_executed,
            self.total_execution_time
        )
    }
}

/// Async runtime for handling async/await
pub struct AsyncRuntime {
    executor: Arc<Mutex<Executor>>,
    waker_registry: Arc<Mutex<WakerRegistry>>,
}

impl AsyncRuntime {
    /// Create a new async runtime
    pub fn new() -> Self {
        Self {
            executor: Arc::new(Mutex::new(Executor::new())),
            waker_registry: Arc::new(Mutex::new(WakerRegistry::new())),
        }
    }
    
    /// Spawn an async task
    pub fn spawn<F>(&self, future: F) -> TaskHandle
    where
        F: FluxFuture<Output = ()> + Send + 'static,
    {
        let mut executor = self.executor.lock().unwrap();
        executor.spawn(Box::new(future))
    }
    
    /// Run the async runtime
    pub fn run(&self) {
        let mut executor = self.executor.lock().unwrap();
        executor.run();
    }
    
    /// Block on a future until completion
    pub fn block_on<F, T>(&self, future: F) -> T
    where
        F: FluxFuture<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let mut executor = self.executor.lock().unwrap();
        executor.block_on(Box::new(future))
    }
    
    /// Create a new waker for the given task
    pub fn create_waker(&self, task_id: u64) -> FluxWaker {
        let mut registry = self.waker_registry.lock().unwrap();
        registry.create_waker(task_id)
    }
}

impl Default for AsyncRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Flux Future trait - similar to std::future::Future but for Flux runtime
pub trait FluxFuture {
    type Output;
    
    /// Poll the future for completion
    fn poll(&mut self, waker: &FluxWaker) -> FluxPoll<Self::Output>;
}

/// Poll result for Flux futures
#[derive(Debug, Clone, PartialEq)]
pub enum FluxPoll<T> {
    /// Future is ready with a value
    Ready(T),
    /// Future is not ready yet
    Pending,
}

/// Waker for Flux futures
#[derive(Clone)]
pub struct FluxWaker {
    task_id: u64,
    wake_fn: Arc<dyn Fn(u64) + Send + Sync>,
}

impl std::fmt::Debug for FluxWaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FluxWaker")
            .field("task_id", &self.task_id)
            .field("wake_fn", &"<function>")
            .finish()
    }
}

impl FluxWaker {
    /// Create a new waker
    pub fn new(task_id: u64, wake_fn: Arc<dyn Fn(u64) + Send + Sync>) -> Self {
        Self { task_id, wake_fn }
    }
    
    /// Wake the associated task
    pub fn wake(&self) {
        (self.wake_fn)(self.task_id);
    }
    
    /// Get the task ID
    pub fn task_id(&self) -> u64 {
        self.task_id
    }
}

/// Registry for managing wakers
pub struct WakerRegistry {
    wakers: HashMap<u64, FluxWaker>,
    next_id: u64,
}

impl WakerRegistry {
    /// Create a new waker registry
    pub fn new() -> Self {
        Self {
            wakers: HashMap::new(),
            next_id: 0,
        }
    }
    
    /// Create a new waker for a task
    pub fn create_waker(&mut self, task_id: u64) -> FluxWaker {
        let wake_fn = Arc::new(move |_id: u64| {
            // In a real implementation, this would notify the executor
            // that the task is ready to be polled again
        });
        
        let waker = FluxWaker::new(task_id, wake_fn);
        self.wakers.insert(task_id, waker.clone());
        waker
    }
    
    /// Remove a waker
    pub fn remove_waker(&mut self, task_id: u64) {
        self.wakers.remove(&task_id);
    }
}

impl Default for WakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Task executor for async operations
pub struct Executor {
    tasks: HashMap<u64, Box<dyn FluxFuture<Output = ()> + Send>>,
    ready_tasks: VecDeque<u64>,
    next_task_id: u64,
    waker_registry: WakerRegistry,
}

impl Executor {
    /// Create a new executor
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            ready_tasks: VecDeque::new(),
            next_task_id: 0,
            waker_registry: WakerRegistry::new(),
        }
    }
    
    /// Spawn a new task
    pub fn spawn(&mut self, future: Box<dyn FluxFuture<Output = ()> + Send>) -> TaskHandle {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        
        self.tasks.insert(task_id, future);
        self.ready_tasks.push_back(task_id);
        
        TaskHandle { id: task_id }
    }
    
    /// Run all tasks to completion
    pub fn run(&mut self) {
        while !self.tasks.is_empty() {
            if let Some(task_id) = self.ready_tasks.pop_front() {
                if let Some(mut task) = self.tasks.remove(&task_id) {
                    let waker = self.waker_registry.create_waker(task_id);
                    
                    match task.poll(&waker) {
                        FluxPoll::Ready(()) => {
                            // Task completed, remove it
                            self.waker_registry.remove_waker(task_id);
                        }
                        FluxPoll::Pending => {
                            // Task not ready, put it back
                            self.tasks.insert(task_id, task);
                        }
                    }
                }
            } else {
                // No ready tasks, break to avoid infinite loop
                break;
            }
        }
    }
    
    /// Block on a future until completion
    pub fn block_on<T>(&mut self, mut future: Box<dyn FluxFuture<Output = T> + Send>) -> T
    where
        T: Send + 'static,
    {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        
        let waker = self.waker_registry.create_waker(task_id);
        
        loop {
            match future.poll(&waker) {
                FluxPoll::Ready(value) => {
                    self.waker_registry.remove_waker(task_id);
                    return value;
                }
                FluxPoll::Pending => {
                    // In a real implementation, we would yield to other tasks
                    // For now, just continue polling
                    std::thread::yield_now();
                }
            }
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to an async task
#[derive(Debug, Clone, Copy)]
pub struct TaskHandle {
    pub id: u64,
}

/// Async function wrapper
pub struct AsyncFunction<F, T>
where
    F: FnOnce() -> T + Send,
    T: Send,
{
    func: Option<F>,
    result: Option<T>,
}

impl<F, T> AsyncFunction<F, T>
where
    F: FnOnce() -> T + Send,
    T: Send,
{
    /// Create a new async function
    pub fn new(func: F) -> Self {
        Self {
            func: Some(func),
            result: None,
        }
    }
}

impl<F, T> FluxFuture for AsyncFunction<F, T>
where
    F: FnOnce() -> T + Send,
    T: Send,
{
    type Output = T;
    
    fn poll(&mut self, _waker: &FluxWaker) -> FluxPoll<Self::Output> {
        if let Some(result) = self.result.take() {
            FluxPoll::Ready(result)
        } else if let Some(func) = self.func.take() {
            let result = func();
            FluxPoll::Ready(result)
        } else {
            FluxPoll::Pending
        }
    }
}

/// Async delay future
pub struct Delay {
    duration: Duration,
    start_time: Option<Instant>,
}

impl Delay {
    /// Create a new delay future
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            start_time: None,
        }
    }
}

impl FluxFuture for Delay {
    type Output = ();
    
    fn poll(&mut self, waker: &FluxWaker) -> FluxPoll<Self::Output> {
        let start = self.start_time.get_or_insert_with(Instant::now);
        
        if start.elapsed() >= self.duration {
            FluxPoll::Ready(())
        } else {
            // In a real implementation, we would set up a timer to wake the task
            // For now, just wake immediately to continue polling
            waker.wake();
            FluxPoll::Pending
        }
    }
}

/// Async channel receive future
pub struct ChannelRecvFuture<T> {
    channel: Channel<T>,
    timeout: Option<Duration>,
    start_time: Option<Instant>,
}

impl<T> ChannelRecvFuture<T> {
    /// Create a new channel receive future
    pub fn new(channel: Channel<T>) -> Self {
        Self {
            channel,
            timeout: None,
            start_time: None,
        }
    }
    
    /// Create a new channel receive future with timeout
    pub fn with_timeout(channel: Channel<T>, timeout: Duration) -> Self {
        Self {
            channel,
            timeout: Some(timeout),
            start_time: None,
        }
    }
}

impl<T: Send> FluxFuture for ChannelRecvFuture<T> {
    type Output = Result<T, crate::runtime::result::RuntimeError>;
    
    fn poll(&mut self, waker: &FluxWaker) -> FluxPoll<Self::Output> {
        // Check timeout
        if let Some(timeout) = self.timeout {
            let start = self.start_time.get_or_insert_with(Instant::now);
            if start.elapsed() >= timeout {
                return FluxPoll::Ready(Err(crate::runtime::result::RuntimeError {
                    message: "Channel receive timeout".to_string(),
                    kind: crate::runtime::result::RuntimeErrorKind::ChannelClosed,
                }));
            }
        }
        
        // Try to receive
        match self.channel.try_recv() {
            Ok(value) => FluxPoll::Ready(Ok(value)),
            Err(_) => {
                // Channel is empty or closed, wake to try again later
                waker.wake();
                FluxPoll::Pending
            }
        }
    }
}

/// Async channel send future
pub struct ChannelSendFuture<T> {
    channel: Channel<T>,
    value: Option<T>,
    timeout: Option<Duration>,
    start_time: Option<Instant>,
}

impl<T> ChannelSendFuture<T> {
    /// Create a new channel send future
    pub fn new(channel: Channel<T>, value: T) -> Self {
        Self {
            channel,
            value: Some(value),
            timeout: None,
            start_time: None,
        }
    }
    
    /// Create a new channel send future with timeout
    pub fn with_timeout(channel: Channel<T>, value: T, timeout: Duration) -> Self {
        Self {
            channel,
            value: Some(value),
            timeout: Some(timeout),
            start_time: None,
        }
    }
}

impl<T: Send> FluxFuture for ChannelSendFuture<T> {
    type Output = Result<(), crate::runtime::result::RuntimeError>;
    
    fn poll(&mut self, waker: &FluxWaker) -> FluxPoll<Self::Output> {
        // Check timeout
        if let Some(timeout) = self.timeout {
            let start = self.start_time.get_or_insert_with(Instant::now);
            if start.elapsed() >= timeout {
                return FluxPoll::Ready(Err(crate::runtime::result::RuntimeError {
                    message: "Channel send timeout".to_string(),
                    kind: crate::runtime::result::RuntimeErrorKind::ChannelClosed,
                }));
            }
        }
        
        // Try to send
        if let Some(value) = self.value.take() {
            match self.channel.try_send(value) {
                Ok(()) => FluxPoll::Ready(Ok(())),
                Err(err) => {
                    // Put the value back and try again later
                    // Note: This is a simplified approach; in practice we'd need to handle the error properly
                    waker.wake();
                    FluxPoll::Pending
                }
            }
        } else {
            FluxPoll::Ready(Err(crate::runtime::result::RuntimeError {
                message: "Value already consumed".to_string(),
                kind: crate::runtime::result::RuntimeErrorKind::ChannelClosed,
            }))
        }
    }
}

/// Async I/O operations
pub struct AsyncIO;

impl AsyncIO {
    /// Async read from a file
    pub fn read_file(path: &str) -> AsyncFunction<impl FnOnce() -> Result<String, std::io::Error> + Send, Result<String, std::io::Error>> {
        let path = path.to_string();
        AsyncFunction::new(move || {
            std::fs::read_to_string(path)
        })
    }
    
    /// Async write to a file
    pub fn write_file(path: &str, content: &str) -> AsyncFunction<impl FnOnce() -> Result<(), std::io::Error> + Send, Result<(), std::io::Error>> {
        let path = path.to_string();
        let content = content.to_string();
        AsyncFunction::new(move || {
            std::fs::write(path, content)
        })
    }
    
    /// Async sleep
    pub fn sleep(duration: Duration) -> Delay {
        Delay::new(duration)
    }
}

/// Async error handling utilities
pub struct AsyncResult<T, E> {
    result: Result<T, E>,
}

impl<T, E> AsyncResult<T, E> {
    /// Create a new async result
    pub fn new(result: Result<T, E>) -> Self {
        Self { result }
    }
    
    /// Create an async Ok result
    pub fn ok(value: T) -> Self {
        Self {
            result: Ok(value),
        }
    }
    
    /// Create an async Err result
    pub fn err(error: E) -> Self {
        Self {
            result: Err(error),
        }
    }
}

impl<T: Send, E: Send> FluxFuture for AsyncResult<T, E> {
    type Output = Result<T, E>;
    
    fn poll(&mut self, _waker: &FluxWaker) -> FluxPoll<Self::Output> {
        // AsyncResult is always ready
        FluxPoll::Ready(std::mem::replace(&mut self.result, Err(unsafe { std::mem::zeroed() })))
    }
}

/// Async combinator for joining multiple futures
pub struct Join<F1, F2>
where
    F1: FluxFuture,
    F2: FluxFuture,
{
    future1: Option<F1>,
    future2: Option<F2>,
    result1: Option<F1::Output>,
    result2: Option<F2::Output>,
}

impl<F1, F2> Join<F1, F2>
where
    F1: FluxFuture,
    F2: FluxFuture,
{
    /// Create a new join future
    pub fn new(future1: F1, future2: F2) -> Self {
        Self {
            future1: Some(future1),
            future2: Some(future2),
            result1: None,
            result2: None,
        }
    }
}

impl<F1, F2> FluxFuture for Join<F1, F2>
where
    F1: FluxFuture,
    F2: FluxFuture,
    F1::Output: Send,
    F2::Output: Send,
{
    type Output = (F1::Output, F2::Output);
    
    fn poll(&mut self, waker: &FluxWaker) -> FluxPoll<Self::Output> {
        // Poll first future if not ready
        if self.result1.is_none() {
            if let Some(mut future1) = self.future1.take() {
                match future1.poll(waker) {
                    FluxPoll::Ready(result) => {
                        self.result1 = Some(result);
                    }
                    FluxPoll::Pending => {
                        self.future1 = Some(future1);
                    }
                }
            }
        }
        
        // Poll second future if not ready
        if self.result2.is_none() {
            if let Some(mut future2) = self.future2.take() {
                match future2.poll(waker) {
                    FluxPoll::Ready(result) => {
                        self.result2 = Some(result);
                    }
                    FluxPoll::Pending => {
                        self.future2 = Some(future2);
                    }
                }
            }
        }
        
        // Check if both are ready
        if let (Some(result1), Some(result2)) = (self.result1.take(), self.result2.take()) {
            FluxPoll::Ready((result1, result2))
        } else {
            FluxPoll::Pending
        }
    }
}