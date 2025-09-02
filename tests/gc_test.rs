//! Unit tests for garbage collection and memory management

use flux_compiler::runtime::gc::{GarbageCollector, GcConfig, ObjectHeader};
use flux_compiler::error::{RuntimeError, RuntimeErrorKind};

#[derive(Debug, Clone, PartialEq)]
struct TestObject {
    value: i32,
    data: Vec<u8>,
}

impl TestObject {
    fn new(value: i32, size: usize) -> Self {
        Self {
            value,
            data: vec![0u8; size],
        }
    }
}

#[test]
fn test_gc_creation() {
    let gc = GarbageCollector::new();
    let stats = gc.gc_stats();
    
    assert_eq!(stats.collections_performed, 0);
    assert_eq!(stats.current_memory_usage, 0);
    assert_eq!(stats.peak_memory_usage, 0);
}

#[test]
fn test_gc_with_config() {
    let config = GcConfig {
        heap_size_threshold: 1024,
        allocation_count_threshold: 10,
        enable_detailed_tracking: true,
        auto_gc_enabled: false,
    };
    
    let gc = GarbageCollector::with_config(config);
    assert!(gc.active_allocations().is_some());
}

#[test]
fn test_basic_allocation() {
    let mut gc = GarbageCollector::new();
    
    let obj = TestObject::new(42, 100);
    let ptr = gc.allocate(obj).expect("Allocation should succeed");
    
    unsafe {
        assert_eq!((*ptr.as_ptr()).value, 42);
        assert_eq!((*ptr.as_ptr()).data.len(), 100);
    }
    
    let stats = gc.gc_stats();
    assert!(stats.current_memory_usage > 0);
    assert!(stats.peak_memory_usage > 0);
}

#[test]
fn test_allocation_with_type_id() {
    let mut gc = GarbageCollector::new();
    
    let obj = TestObject::new(123, 50);
    let type_id = 42u32;
    let ptr = gc.allocate_with_type_id(obj, type_id).expect("Allocation should succeed");
    
    unsafe {
        let header_ptr = ObjectHeader::from_object_ptr(ptr.as_ptr());
        let header = &*header_ptr;
        assert_eq!(header.type_id, type_id);
        assert!(header.size > 0);
        assert!(!header.marked);
    }
}

#[test]
fn test_manual_deallocation() {
    let mut gc = GarbageCollector::new();
    
    let obj = TestObject::new(99, 200);
    let ptr = gc.allocate(obj).expect("Allocation should succeed");
    
    let initial_memory = gc.gc_stats().current_memory_usage;
    assert!(initial_memory > 0);
    
    unsafe {
        gc.deallocate(ptr).expect("Deallocation should succeed");
    }
    
    let final_memory = gc.gc_stats().current_memory_usage;
    assert!(final_memory < initial_memory);
}

#[test]
fn test_root_management() {
    let mut gc = GarbageCollector::new();
    
    let obj = TestObject::new(77, 150);
    let ptr = gc.allocate(obj).expect("Allocation should succeed");
    
    // Add as root
    gc.add_root(ptr);
    
    // Remove from roots
    gc.remove_root(ptr);
    
    // Should not panic
}

#[test]
fn test_garbage_collection() {
    let mut gc = GarbageCollector::new();
    
    // Allocate some objects
    let obj1 = TestObject::new(1, 100);
    let obj2 = TestObject::new(2, 200);
    let obj3 = TestObject::new(3, 300);
    
    let ptr1 = gc.allocate(obj1).expect("Allocation should succeed");
    let _ptr2 = gc.allocate(obj2).expect("Allocation should succeed");
    let ptr3 = gc.allocate(obj3).expect("Allocation should succeed");
    
    // Add some roots
    gc.add_root(ptr1);
    gc.add_root(ptr3);
    
    let initial_collections = gc.gc_stats().collections_performed;
    
    // Force garbage collection
    gc.force_collect();
    
    let final_collections = gc.gc_stats().collections_performed;
    assert_eq!(final_collections, initial_collections + 1);
}

#[test]
fn test_allocation_tracking() {
    let config = GcConfig {
        enable_detailed_tracking: true,
        ..Default::default()
    };
    let mut gc = GarbageCollector::with_config(config);
    
    let obj = TestObject::new(555, 75);
    let type_id = 42u32; // Use a simple type ID for testing
    let _ptr = gc.allocate_with_type_id(obj, type_id).expect("Allocation should succeed");
    
    let allocation_stats = gc.allocation_stats();
    assert!(!allocation_stats.is_empty());
    
    let active_allocations = gc.active_allocations();
    assert!(active_allocations.is_some());
    assert_eq!(active_allocations.unwrap().len(), 1);
}

#[test]
fn test_memory_summary() {
    let mut gc = GarbageCollector::new();
    
    let obj = TestObject::new(888, 500);
    let _ptr = gc.allocate(obj).expect("Allocation should succeed");
    
    let summary = gc.memory_summary();
    assert!(summary.current_usage > 0);
    assert!(summary.peak_usage > 0);
    assert!(summary.total_allocated > 0);
    assert_eq!(summary.active_objects, 1);
    assert_eq!(summary.collections_performed, 0);
}

#[test]
fn test_detailed_tracking_toggle() {
    let mut gc = GarbageCollector::new();
    
    // Initially disabled
    assert!(gc.active_allocations().is_none());
    
    // Enable detailed tracking
    gc.set_detailed_tracking(true);
    assert!(gc.active_allocations().is_some());
    
    // Disable detailed tracking
    gc.set_detailed_tracking(false);
    assert!(gc.active_allocations().is_none());
}

#[test]
fn test_auto_gc_triggering() {
    let config = GcConfig {
        heap_size_threshold: 1000, // Small threshold
        allocation_count_threshold: 3, // Small threshold
        auto_gc_enabled: true,
        ..Default::default()
    };
    let mut gc = GarbageCollector::with_config(config);
    
    let initial_collections = gc.gc_stats().collections_performed;
    
    // Allocate enough objects to trigger auto GC
    for i in 0..5 {
        let obj = TestObject::new(i, 300);
        let _ptr = gc.allocate(obj).expect("Allocation should succeed");
    }
    
    let final_collections = gc.gc_stats().collections_performed;
    assert!(final_collections > initial_collections);
}

#[test]
fn test_object_header_functionality() {
    let header = ObjectHeader::new(1024, 42);
    
    assert_eq!(header.size, 1024);
    assert_eq!(header.type_id, 42);
    assert!(!header.marked);
    assert_eq!(header.generation, 0);
    assert_eq!(header.ref_count, 0);
    
    let data_ptr = header.data_ptr();
    assert!(!data_ptr.is_null());
}

#[test]
fn test_heap_stats() {
    let mut gc = GarbageCollector::new();
    
    let obj = TestObject::new(111, 250);
    let _ptr = gc.allocate(obj).expect("Allocation should succeed");
    
    let heap_stats = gc.heap_stats();
    assert!(heap_stats.total_allocated > 0);
    assert_eq!(heap_stats.total_freed, 0);
    assert!(heap_stats.current_allocated > 0);
    assert_eq!(heap_stats.object_count, 1);
}

#[test]
fn test_out_of_memory_handling() {
    // This test is difficult to implement without actually exhausting memory
    // In a real implementation, we might mock the allocator
    let mut gc = GarbageCollector::new();
    
    // Try to allocate a reasonably large object that might fail
    let large_obj = TestObject::new(0, 1024 * 1024); // 1MB instead of huge size
    let result = gc.allocate(large_obj);
    
    // This should succeed on most systems
    match result {
        Ok(_) => {
            // Allocation succeeded (expected)
        }
        Err(RuntimeError { kind: RuntimeErrorKind::OutOfMemory }) => {
            // This is also acceptable if system is low on memory
        }
        Err(_) => {
            panic!("Unexpected error type");
        }
    }
}

#[test]
fn test_gc_stats_display() {
    let mut gc = GarbageCollector::new();
    
    let obj = TestObject::new(999, 100);
    let _ptr = gc.allocate(obj).expect("Allocation should succeed");
    
    gc.force_collect();
    
    let summary = gc.memory_summary();
    let display_str = format!("{}", summary);
    
    assert!(display_str.contains("Memory Summary:"));
    assert!(display_str.contains("Current Usage:"));
    assert!(display_str.contains("Peak Usage:"));
    assert!(display_str.contains("GC Collections:"));
}