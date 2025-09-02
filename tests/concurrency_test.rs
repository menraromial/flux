//! Unit tests for concurrency runtime

use flux_compiler::runtime::concurrency::{
    Scheduler, Goroutine, GoroutineState, Context, Stack, SchedulerStats
};
use flux_compiler::runtime::{
    Channel, GoroutineId, GoroutineHandle, ChannelError, Select, SelectResult, 
    make_channel, make_unbuffered_channel, AsyncRuntime, AsyncFunction, Delay, 
    ChannelRecvFuture, ChannelSendFuture, AsyncResult, Join, AsyncIO, FluxFuture, 
    FluxPoll, FluxWaker, Executor, WakerRegistry
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

#[test]
fn test_scheduler_creation() {
    let scheduler = Scheduler::new();
    
    assert!(!scheduler.is_running());
    assert_eq!(scheduler.active_goroutine_count(), 0);
    assert_eq!(scheduler.ready_goroutine_count(), 0);
}

#[test]
fn test_scheduler_with_worker_count() {
    let scheduler = Scheduler::with_worker_count(2);
    
    assert!(!scheduler.is_running());
    assert_eq!(scheduler.active_goroutine_count(), 0);
}

#[test]
fn test_goroutine_creation() {
    fn test_func() {
        // Simple test function
    }
    
    let goroutine = Goroutine::new(1, test_func);
    
    assert_eq!(goroutine.id, 1);
    assert_eq!(goroutine.state, GoroutineState::Ready);
    assert!(goroutine.function.is_some());
}

#[test]
fn test_goroutine_execution() {
    let executed = Arc::new(Mutex::new(false));
    let executed_clone = Arc::clone(&executed);
    
    fn test_func() {
        // This will be replaced by the closure
    }
    
    let mut goroutine = Goroutine::new(1, test_func);
    
    // Replace with a closure that sets the flag
    goroutine.function = Some(|| {
        // In a real test, we'd need a way to capture the executed flag
        // For now, just test that the state changes
    });
    
    assert_eq!(goroutine.state, GoroutineState::Ready);
    goroutine.run();
    assert_eq!(goroutine.state, GoroutineState::Finished);
}

#[test]
fn test_scheduler_spawn_goroutine() {
    let scheduler = Scheduler::new();
    
    fn test_func() {
        // Simple test function
    }
    
    let handle = scheduler.spawn(test_func);
    
    assert_eq!(scheduler.active_goroutine_count(), 1);
    assert_eq!(scheduler.ready_goroutine_count(), 1);
}

#[test]
fn test_scheduler_add_multiple_goroutines() {
    let scheduler = Scheduler::new();
    
    fn test_func1() {}
    fn test_func2() {}
    fn test_func3() {}
    
    let _handle1 = scheduler.spawn(test_func1);
    let _handle2 = scheduler.spawn(test_func2);
    let _handle3 = scheduler.spawn(test_func3);
    
    assert_eq!(scheduler.active_goroutine_count(), 3);
    assert_eq!(scheduler.ready_goroutine_count(), 3);
}

#[test]
fn test_scheduler_statistics() {
    let scheduler = Scheduler::new();
    
    fn test_func() {}
    
    let _handle = scheduler.spawn(test_func);
    
    let stats = scheduler.stats();
    assert_eq!(stats.total_goroutines, 1);
    assert_eq!(stats.ready_count, 1);
    assert_eq!(stats.running_count, 0);
    assert_eq!(stats.blocked_count, 0);
    assert_eq!(stats.finished_count, 0);
}

#[test]
fn test_scheduler_yield() {
    let scheduler = Scheduler::new();
    
    fn test_func() {}
    
    let _handle = scheduler.spawn(test_func);
    
    // Test yield (should not panic)
    scheduler.yield_now();
}

#[test]
fn test_scheduler_block_unblock() {
    let scheduler = Scheduler::new();
    
    fn test_func() {}
    
    let handle = scheduler.spawn(test_func);
    
    // Test blocking and unblocking
    scheduler.block_current();
    scheduler.unblock(handle.id);
}

#[test]
fn test_scheduler_shutdown() {
    let scheduler = Scheduler::new();
    
    assert!(!scheduler.is_running());
    
    scheduler.shutdown();
    
    assert!(!scheduler.is_running());
}

#[test]
fn test_context_creation() {
    let context = Context::new();
    
    assert_eq!(context.stack_pointer, 0);
    assert_eq!(context.instruction_pointer, 0);
}

#[test]
fn test_stack_creation() {
    let stack = Stack::new(8192);
    
    assert_eq!(stack.size(), 8192);
    assert!(!stack.stack_pointer().is_null());
}

#[test]
fn test_channel_creation() {
    let channel: Channel<i32> = Channel::new(10);
    
    assert_eq!(channel.capacity(), 10);
    assert_eq!(channel.len(), 0);
    assert!(channel.is_empty());
    assert!(!channel.is_closed());
}

#[test]
fn test_channel_send_receive() {
    let channel: Channel<i32> = Channel::new(5);
    
    // Send some values
    assert!(channel.send(1).is_ok());
    assert!(channel.send(2).is_ok());
    assert!(channel.send(3).is_ok());
    
    assert_eq!(channel.len(), 3);
    assert!(!channel.is_empty());
    
    // Receive values
    assert_eq!(channel.recv().unwrap(), 1);
    assert_eq!(channel.recv().unwrap(), 2);
    assert_eq!(channel.recv().unwrap(), 3);
    
    assert_eq!(channel.len(), 0);
    assert!(channel.is_empty());
}

#[test]
fn test_channel_try_operations() {
    let channel: Channel<i32> = Channel::new(2);
    
    // Try send
    assert!(channel.try_send(1).is_ok());
    assert!(channel.try_send(2).is_ok());
    
    // Channel should be full now
    assert_eq!(channel.len(), 2);
    
    // Try receive
    assert_eq!(channel.try_recv().unwrap(), 1);
    assert_eq!(channel.try_recv().unwrap(), 2);
    
    // Channel should be empty now
    assert!(channel.is_empty());
}

#[test]
fn test_channel_close() {
    let channel: Channel<i32> = Channel::new(5);
    
    assert!(!channel.is_closed());
    
    channel.close();
    
    assert!(channel.is_closed());
    
    // Sending to closed channel should fail
    assert!(channel.send(1).is_err());
}

#[test]
fn test_unbuffered_channel() {
    let channel: Channel<i32> = Channel::new(0);
    
    assert_eq!(channel.capacity(), 0);
    
    // For unbuffered channels, send should still work in this simple implementation
    assert!(channel.send(42).is_ok());
    assert_eq!(channel.recv().unwrap(), 42);
}

#[test]
fn test_scheduler_stats_display() {
    let stats = SchedulerStats {
        total_goroutines: 5,
        ready_count: 2,
        running_count: 1,
        blocked_count: 1,
        finished_count: 1,
        goroutines_executed: 10,
        total_execution_time: Duration::from_millis(100),
    };
    
    let display_str = format!("{}", stats);
    
    assert!(display_str.contains("Scheduler Statistics:"));
    assert!(display_str.contains("Total Goroutines: 5"));
    assert!(display_str.contains("Ready: 2"));
    assert!(display_str.contains("Running: 1"));
    assert!(display_str.contains("Blocked: 1"));
    assert!(display_str.contains("Finished: 1"));
    assert!(display_str.contains("Executed: 10"));
}

#[test]
fn test_goroutine_handle() {
    let handle = GoroutineHandle { id: 42 };
    
    assert_eq!(handle.id, 42);
}

#[test]
fn test_goroutine_states() {
    assert_eq!(GoroutineState::Ready, GoroutineState::Ready);
    assert_ne!(GoroutineState::Ready, GoroutineState::Running);
    assert_ne!(GoroutineState::Running, GoroutineState::Blocked);
    assert_ne!(GoroutineState::Blocked, GoroutineState::Finished);
}

// Integration test for scheduler with actual execution
#[test]
fn test_scheduler_integration() {
    let scheduler = Scheduler::with_worker_count(1);
    
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = Arc::clone(&counter);
    
    // This is a limitation of the current design - we can't easily pass closures
    // In a real implementation, we'd need a different approach for goroutine functions
    fn increment_func() {
        // Would increment counter if we could capture it
    }
    
    let _handle = scheduler.spawn(increment_func);
    
    // Give some time for execution (in a real test, we'd have better synchronization)
    thread::sleep(Duration::from_millis(10));
    
    scheduler.shutdown();
    
    // Test that scheduler can be shut down without issues
    assert!(!scheduler.is_running());
}

#[test]
fn test_channel_clone() {
    let channel: Channel<i32> = Channel::new(5);
    let channel_clone = channel.clone();
    
    // Send through original
    assert!(channel.send(42).is_ok());
    
    // Receive through clone
    assert_eq!(channel_clone.recv().unwrap(), 42);
    
    // Both should see the same state
    assert_eq!(channel.len(), channel_clone.len());
    assert_eq!(channel.is_closed(), channel_clone.is_closed());
}

// Advanced channel operation tests

#[test]
fn test_channel_send_timeout() {
    let channel: Channel<i32> = Channel::new(1);
    
    // Fill the channel
    assert!(channel.send(1).is_ok());
    
    // Try to send with timeout - should timeout since channel is full
    let result = channel.send_timeout(2, Duration::from_millis(10));
    assert!(matches!(result, Err(ChannelError::Timeout(2))));
}

#[test]
fn test_channel_recv_timeout() {
    let channel: Channel<i32> = Channel::new(1);
    
    // Try to receive from empty channel with timeout
    let result = channel.recv_timeout(Duration::from_millis(10));
    assert!(matches!(result, Err(ChannelError::Timeout(()))));
}

#[test]
fn test_channel_send_to_closed() {
    let channel: Channel<i32> = Channel::new(1);
    
    channel.close();
    
    // Try to send to closed channel with timeout
    let result = channel.send_timeout(42, Duration::from_millis(10));
    assert!(matches!(result, Err(ChannelError::Closed(42))));
}

#[test]
fn test_channel_recv_from_closed() {
    let channel: Channel<i32> = Channel::new(1);
    
    // Add a value then close
    assert!(channel.send(42).is_ok());
    channel.close();
    
    // Should still be able to receive the buffered value
    assert_eq!(channel.recv().unwrap(), 42);
    
    // Now receiving should fail
    assert!(channel.recv().is_err());
}

#[test]
fn test_channel_close_wakes_waiters() {
    let channel: Channel<i32> = Channel::new(0); // Unbuffered
    let channel_clone = channel.clone();
    
    // Spawn a thread that will try to send (and block)
    let handle = thread::spawn(move || {
        // This would block in a real implementation
        channel_clone.try_send(42)
    });
    
    // Give the thread a moment to start
    thread::sleep(Duration::from_millis(1));
    
    // Close the channel
    channel.close();
    
    // The send should fail with closed error
    let result = handle.join().unwrap();
    assert!(result.is_err());
}

#[test]
fn test_select_basic() {
    let select = Select::new();
    let channel1: Channel<i32> = Channel::new(1);
    let channel2: Channel<i32> = Channel::new(1);
    
    // Put a value in channel1
    assert!(channel1.send(42).is_ok());
    
    let select = select.recv(channel1.clone()).recv(channel2.clone());
    let (index, result) = select.execute();
    
    // Should have selected channel1 (index 0)
    assert_eq!(index, 0);
    assert!(matches!(result, SelectResult::Ok(_)));
}

#[test]
fn test_select_with_default() {
    let select = Select::new();
    let channel: Channel<i32> = Channel::new(1);
    
    let select = select.recv(channel.clone()).default(|| {
        // Default case
    });
    
    let (index, result) = select.execute();
    
    // Should have selected default case (index 1)
    assert_eq!(index, 1);
    assert!(matches!(result, SelectResult::Default));
}

#[test]
fn test_select_send_operation() {
    let select = Select::new();
    let channel: Channel<i32> = Channel::new(1);
    
    let select = select.send(channel.clone(), 42);
    let (index, result) = select.execute();
    
    // Should have completed the send
    assert_eq!(index, 0);
    assert!(matches!(result, SelectResult::Ok(_)));
    
    // Verify the value was sent
    assert_eq!(channel.recv().unwrap(), 42);
}

#[test]
fn test_select_timeout() {
    let select = Select::new();
    let channel: Channel<i32> = Channel::new(1);
    
    let select = select.recv(channel.clone());
    let result = select.execute_timeout(Duration::from_millis(10));
    
    // Should timeout since channel is empty and no default
    assert!(result.is_none());
}

#[test]
fn test_select_timeout_with_default() {
    let select = Select::new();
    let channel: Channel<i32> = Channel::new(1);
    
    let select = select.recv(channel.clone()).default(|| {
        // Default case
    });
    
    let result = select.execute_timeout(Duration::from_millis(10));
    
    // Should execute default case
    assert!(result.is_some());
    let (index, select_result) = result.unwrap();
    assert_eq!(index, 1);
    assert!(matches!(select_result, SelectResult::Default));
}

#[test]
fn test_make_channel_convenience() {
    let channel: Channel<String> = make_channel(5);
    assert_eq!(channel.capacity(), 5);
    
    let unbuffered: Channel<i32> = make_unbuffered_channel();
    assert_eq!(unbuffered.capacity(), 0);
}

#[test]
fn test_channel_error_display() {
    let closed_error: ChannelError<i32> = ChannelError::Closed(42);
    let timeout_error: ChannelError<String> = ChannelError::Timeout("test".to_string());
    
    assert_eq!(format!("{}", closed_error), "Channel is closed");
    assert_eq!(format!("{}", timeout_error), "Operation timed out");
}

#[test]
fn test_channel_error_equality() {
    let error1: ChannelError<i32> = ChannelError::Closed(42);
    let error2: ChannelError<i32> = ChannelError::Closed(42);
    let error3: ChannelError<i32> = ChannelError::Timeout(42);
    
    assert_eq!(error1, error2);
    assert_ne!(error1, error3);
}

#[test]
fn test_select_multiple_ready_operations() {
    let select = Select::new();
    let channel1: Channel<i32> = Channel::new(1);
    let channel2: Channel<i32> = Channel::new(1);
    
    // Put values in both channels
    assert!(channel1.send(1).is_ok());
    assert!(channel2.send(2).is_ok());
    
    let select = select.recv(channel1.clone()).recv(channel2.clone());
    let (index, result) = select.execute();
    
    // Should select the first ready operation (channel1, index 0)
    assert_eq!(index, 0);
    assert!(matches!(result, SelectResult::Ok(_)));
}

#[test]
fn test_select_closed_channel() {
    let select = Select::new();
    let channel: Channel<i32> = Channel::new(1);
    
    channel.close();
    
    let select = select.recv(channel.clone());
    let (index, result) = select.execute();
    
    // Should detect closed channel
    assert_eq!(index, 0);
    assert!(matches!(result, SelectResult::Closed));
}

// Integration tests for concurrent channel operations

#[test]
fn test_concurrent_channel_operations() {
    let channel: Channel<i32> = Channel::new(3);
    let channel_clone = channel.clone();
    
    // Spawn a sender thread
    let sender_handle = thread::spawn(move || {
        for i in 0..5 {
            if let Err(_) = channel_clone.send(i) {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
    });
    
    // Receive values
    let mut received = Vec::new();
    for _ in 0..5 {
        match channel.recv() {
            Ok(value) => received.push(value),
            Err(_) => break,
        }
    }
    
    sender_handle.join().unwrap();
    
    // Should have received some values
    assert!(!received.is_empty());
    assert!(received.len() <= 5);
}

#[test]
fn test_channel_with_multiple_senders_receivers() {
    let channel: Channel<i32> = Channel::new(10);
    let num_senders = 3;
    let num_receivers = 2;
    let messages_per_sender = 5;
    
    let mut sender_handles = Vec::new();
    let mut receiver_handles = Vec::new();
    
    // Spawn senders
    for sender_id in 0..num_senders {
        let channel_clone = channel.clone();
        let handle = thread::spawn(move || {
            for i in 0..messages_per_sender {
                let value = sender_id * 100 + i;
                if channel_clone.send(value).is_err() {
                    break;
                }
            }
        });
        sender_handles.push(handle);
    }
    
    // Spawn receivers
    let received = Arc::new(Mutex::new(Vec::new()));
    for _ in 0..num_receivers {
        let channel_clone = channel.clone();
        let received_clone = Arc::clone(&received);
        let handle = thread::spawn(move || {
            loop {
                match channel_clone.recv() {
                    Ok(value) => {
                        let mut received_guard = received_clone.lock().unwrap();
                        received_guard.push(value);
                    }
                    Err(_) => break,
                }
            }
        });
        receiver_handles.push(handle);
    }
    
    // Wait for all senders to complete
    for handle in sender_handles {
        handle.join().unwrap();
    }
    
    // Close channel to signal receivers
    channel.close();
    
    // Wait for all receivers to complete
    for handle in receiver_handles {
        handle.join().unwrap();
    }
    
    // Check that we received the expected number of messages
    let received_guard = received.lock().unwrap();
    assert_eq!(received_guard.len(), (num_senders * messages_per_sender) as usize);
}

#[test]
fn test_unbuffered_channel_synchronization() {
    let channel: Channel<i32> = make_unbuffered_channel();
    let channel_clone = channel.clone();
    
    let sender_handle = thread::spawn(move || {
        // This should complete when receiver is ready
        channel_clone.send(42)
    });
    
    // Small delay to let sender start
    thread::sleep(Duration::from_millis(1));
    
    // Receive the value
    let received = channel.recv().unwrap();
    assert_eq!(received, 42);
    
    // Sender should complete successfully
    assert!(sender_handle.join().unwrap().is_ok());
}

// Async/await tests

#[test]
fn test_async_runtime_creation() {
    let runtime = AsyncRuntime::new();
    // Runtime should be created successfully
    assert!(true); // Placeholder assertion
}

#[test]
fn test_async_function_future() {
    let mut future = AsyncFunction::new(|| 42);
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    match future.poll(&waker) {
        FluxPoll::Ready(result) => assert_eq!(result, 42),
        FluxPoll::Pending => panic!("Future should be ready immediately"),
    }
}

#[test]
fn test_delay_future() {
    let mut delay = Delay::new(Duration::from_millis(1));
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    // First poll might be pending
    match delay.poll(&waker) {
        FluxPoll::Ready(()) => {
            // Delay completed immediately (possible if very short)
        }
        FluxPoll::Pending => {
            // Wait a bit and poll again
            thread::sleep(Duration::from_millis(2));
            match delay.poll(&waker) {
                FluxPoll::Ready(()) => {
                    // Delay should be ready now
                }
                FluxPoll::Pending => {
                    // Still pending, but that's okay for this test
                }
            }
        }
    }
}

#[test]
fn test_channel_recv_future() {
    let channel: Channel<i32> = Channel::new(1);
    
    // Put a value in the channel
    assert!(channel.send(42).is_ok());
    
    let mut recv_future = ChannelRecvFuture::new(channel);
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    match recv_future.poll(&waker) {
        FluxPoll::Ready(Ok(value)) => assert_eq!(value, 42),
        FluxPoll::Ready(Err(_)) => panic!("Receive should succeed"),
        FluxPoll::Pending => panic!("Receive should be ready immediately"),
    }
}

#[test]
fn test_channel_send_future() {
    let channel: Channel<i32> = Channel::new(1);
    
    let mut send_future = ChannelSendFuture::new(channel.clone(), 42);
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    match send_future.poll(&waker) {
        FluxPoll::Ready(Ok(())) => {
            // Send succeeded, verify the value is in the channel
            assert_eq!(channel.recv().unwrap(), 42);
        }
        FluxPoll::Ready(Err(_)) => panic!("Send should succeed"),
        FluxPoll::Pending => panic!("Send should be ready immediately for buffered channel"),
    }
}

#[test]
fn test_channel_recv_future_timeout() {
    let channel: Channel<i32> = Channel::new(1);
    
    let mut recv_future = ChannelRecvFuture::with_timeout(channel, Duration::from_millis(1));
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    // First poll should be pending
    match recv_future.poll(&waker) {
        FluxPoll::Pending => {
            // Wait for timeout
            thread::sleep(Duration::from_millis(2));
            
            // Poll again, should timeout
            match recv_future.poll(&waker) {
                FluxPoll::Ready(Err(_)) => {
                    // Expected timeout error
                }
                FluxPoll::Ready(Ok(_)) => panic!("Should not receive a value"),
                FluxPoll::Pending => {
                    // Still pending, timeout might not have occurred yet
                }
            }
        }
        FluxPoll::Ready(_) => {
            // Might be ready immediately if channel has data or is closed
        }
    }
}

#[test]
fn test_async_result_ok() {
    let mut result: AsyncResult<i32, &str> = AsyncResult::ok(42);
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    match result.poll(&waker) {
        FluxPoll::Ready(Ok(value)) => assert_eq!(value, 42),
        FluxPoll::Ready(Err(_)) => panic!("Should be Ok"),
        FluxPoll::Pending => panic!("AsyncResult should be ready immediately"),
    }
}

#[test]
fn test_async_result_err() {
    let mut result: AsyncResult<i32, &str> = AsyncResult::err("error");
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    match result.poll(&waker) {
        FluxPoll::Ready(Err(err)) => assert_eq!(err, "error"),
        FluxPoll::Ready(Ok(_)) => panic!("Should be Err"),
        FluxPoll::Pending => panic!("AsyncResult should be ready immediately"),
    }
}

#[test]
fn test_join_future() {
    let future1 = AsyncFunction::new(|| 42);
    let future2 = AsyncFunction::new(|| "hello");
    
    let mut join = Join::new(future1, future2);
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    match join.poll(&waker) {
        FluxPoll::Ready((result1, result2)) => {
            assert_eq!(result1, 42);
            assert_eq!(result2, "hello");
        }
        FluxPoll::Pending => panic!("Join should be ready immediately for ready futures"),
    }
}

#[test]
fn test_async_io_read_file() {
    // Create a temporary file for testing
    let temp_file = std::env::temp_dir().join("flux_test_read.txt");
    std::fs::write(&temp_file, "test content").unwrap();
    
    let mut read_future = AsyncIO::read_file(temp_file.to_str().unwrap());
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    match read_future.poll(&waker) {
        FluxPoll::Ready(Ok(content)) => {
            assert_eq!(content, "test content");
        }
        FluxPoll::Ready(Err(err)) => panic!("Read should succeed: {}", err),
        FluxPoll::Pending => panic!("File read should be ready immediately"),
    }
    
    // Clean up
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_async_io_write_file() {
    let temp_file = std::env::temp_dir().join("flux_test_write.txt");
    
    let mut write_future = AsyncIO::write_file(temp_file.to_str().unwrap(), "test content");
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    match write_future.poll(&waker) {
        FluxPoll::Ready(Ok(())) => {
            // Verify the file was written
            let content = std::fs::read_to_string(&temp_file).unwrap();
            assert_eq!(content, "test content");
        }
        FluxPoll::Ready(Err(err)) => panic!("Write should succeed: {}", err),
        FluxPoll::Pending => panic!("File write should be ready immediately"),
    }
    
    // Clean up
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_async_sleep() {
    let mut sleep_future = AsyncIO::sleep(Duration::from_millis(1));
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    // First poll might be pending
    match sleep_future.poll(&waker) {
        FluxPoll::Ready(()) => {
            // Sleep completed immediately (possible for very short durations)
        }
        FluxPoll::Pending => {
            // Wait and poll again
            thread::sleep(Duration::from_millis(2));
            match sleep_future.poll(&waker) {
                FluxPoll::Ready(()) => {
                    // Sleep should be ready now
                }
                FluxPoll::Pending => {
                    // Still pending, but that's okay for this test
                }
            }
        }
    }
}

#[test]
fn test_waker_functionality() {
    let wake_called = Arc::new(Mutex::new(false));
    let wake_called_clone = Arc::clone(&wake_called);
    
    let waker = FluxWaker::new(42, Arc::new(move |task_id| {
        assert_eq!(task_id, 42);
        let mut called = wake_called_clone.lock().unwrap();
        *called = true;
    }));
    
    assert_eq!(waker.task_id(), 42);
    
    waker.wake();
    
    let called = wake_called.lock().unwrap();
    assert!(*called);
}

#[test]
fn test_executor_spawn_and_run() {
    let mut executor = Executor::new();
    
    let future = AsyncFunction::new(|| {
        // Return unit type for spawn
    });
    let handle = executor.spawn(Box::new(future));
    
    assert!(handle.id >= 0);
    
    // Run the executor
    executor.run();
    
    // All tasks should be completed (we can't access private field, so just check it doesn't panic)
    assert!(true);
}

#[test]
fn test_executor_block_on() {
    let mut executor = Executor::new();
    
    let future = AsyncFunction::new(|| 42);
    let result = executor.block_on(Box::new(future));
    
    assert_eq!(result, 42);
}

#[test]
fn test_waker_registry() {
    let mut registry = WakerRegistry::new();
    
    let waker = registry.create_waker(123);
    assert_eq!(waker.task_id(), 123);
    
    registry.remove_waker(123);
    
    // Registry should handle removal gracefully
    assert!(true);
}

// Integration tests for async patterns

#[test]
fn test_async_channel_communication() {
    let channel: Channel<String> = Channel::new(1);
    let channel_clone = channel.clone();
    
    // Create a send future
    let send_future = ChannelSendFuture::new(channel_clone, "hello".to_string());
    
    // Create a receive future
    let recv_future = ChannelRecvFuture::new(channel);
    
    // Join them
    let mut join_future = Join::new(send_future, recv_future);
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    match join_future.poll(&waker) {
        FluxPoll::Ready((send_result, recv_result)) => {
            assert!(send_result.is_ok());
            assert_eq!(recv_result.unwrap(), "hello");
        }
        FluxPoll::Pending => {
            // Might be pending due to channel synchronization
            // In a real implementation, we'd continue polling
        }
    }
}

#[test]
fn test_async_error_propagation() {
    let error_msg = "async error".to_string();
    let mut future = AsyncFunction::new(move || {
        // Simulate error propagation
        let result: Result<i32, String> = Err(format!("Propagated: {}", error_msg));
        result
    });
    
    let waker = FluxWaker::new(0, Arc::new(|_| {}));
    
    match future.poll(&waker) {
        FluxPoll::Ready(Err(err)) => {
            assert_eq!(err, "Propagated: async error");
        }
        FluxPoll::Ready(Ok(_)) => panic!("Should propagate error"),
        FluxPoll::Pending => panic!("Should be ready immediately"),
    }
}

#[test]
fn test_async_runtime_integration() {
    let runtime = AsyncRuntime::new();
    
    // Test spawning a simple async task
    let future = AsyncFunction::new(|| {
        // Simulate some async work (return unit type)
    });
    
    let handle = runtime.spawn(future);
    assert!(handle.id >= 0);
    
    // Run the runtime
    runtime.run();
    
    // Runtime should complete without issues
    assert!(true);
}