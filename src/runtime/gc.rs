//! Garbage collection implementation
//! 
//! Provides mark-and-sweep garbage collection for Flux programs.

use crate::error::{RuntimeError, RuntimeErrorKind};
use std::collections::{HashSet, HashMap};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::any::TypeId;
use std::hash::{Hash, Hasher};

/// Convert TypeId to u32 for storage
trait TypeIdExt {
    fn into(self) -> u32;
}

impl TypeIdExt for TypeId {
    fn into(self) -> u32 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish() as u32
    }
}

/// Object header for garbage collection metadata
#[repr(C)]
pub struct ObjectHeader {
    /// Mark bit for garbage collection
    pub marked: bool,
    /// Size of the object in bytes (including header)
    pub size: usize,
    /// Type information for runtime type checking
    pub type_id: u32,
    /// Generation for generational GC (future enhancement)
    pub generation: u8,
    /// Reference count for debugging
    pub ref_count: u32,
    /// Allocation timestamp for debugging
    pub allocated_at: u64,
}

impl ObjectHeader {
    /// Create a new object header
    pub fn new(size: usize, type_id: u32) -> Self {
        static ALLOCATION_COUNTER: AtomicUsize = AtomicUsize::new(0);
        
        Self {
            marked: false,
            size,
            type_id,
            generation: 0,
            ref_count: 0,
            allocated_at: ALLOCATION_COUNTER.fetch_add(1, Ordering::SeqCst) as u64,
        }
    }
    
    /// Get the object data pointer from header
    pub fn data_ptr(&self) -> *mut u8 {
        unsafe {
            (self as *const Self as *mut u8).add(std::mem::size_of::<Self>())
        }
    }
    
    /// Get the header from an object pointer
    pub unsafe fn from_object_ptr<T: 'static>(ptr: *mut T) -> *mut ObjectHeader {
        (ptr as *mut u8).sub(std::mem::size_of::<ObjectHeader>()) as *mut ObjectHeader
    }
}

/// Garbage collector using mark-and-sweep algorithm
pub struct GarbageCollector {
    heap: Heap,
    mark_stack: Vec<NonNull<ObjectHeader>>,
    roots: HashSet<NonNull<ObjectHeader>>,
    /// GC statistics
    stats: GcStats,
    /// Memory allocation tracking for debugging
    allocation_tracker: AllocationTracker,
    /// GC configuration
    config: GcConfig,
}

/// Garbage collection statistics
#[derive(Debug, Clone, Default)]
pub struct GcStats {
    pub collections_performed: usize,
    pub total_collection_time: Duration,
    pub objects_collected: usize,
    pub bytes_collected: usize,
    pub peak_memory_usage: usize,
    pub current_memory_usage: usize,
}

/// Memory allocation tracker for debugging
#[derive(Debug, Default)]
pub struct AllocationTracker {
    /// Track allocations by type
    allocations_by_type: HashMap<u32, AllocationInfo>,
    /// Track all active allocations
    active_allocations: HashMap<*mut ObjectHeader, AllocationDebugInfo>,
    /// Enable detailed tracking (performance impact)
    detailed_tracking: bool,
}

#[derive(Debug, Clone)]
pub struct AllocationInfo {
    count: usize,
    total_size: usize,
    peak_count: usize,
    peak_size: usize,
}

#[derive(Debug, Clone)]
pub struct AllocationDebugInfo {
    size: usize,
    type_id: u32,
    allocated_at: Instant,
    stack_trace: Option<String>, // Placeholder for stack trace
}

/// Garbage collector configuration
#[derive(Debug, Clone)]
pub struct GcConfig {
    /// Trigger GC when heap size exceeds this threshold
    pub heap_size_threshold: usize,
    /// Trigger GC when allocation count exceeds this threshold
    pub allocation_count_threshold: usize,
    /// Enable detailed memory tracking (debug mode)
    pub enable_detailed_tracking: bool,
    /// Enable automatic GC triggering
    pub auto_gc_enabled: bool,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            heap_size_threshold: 64 * 1024 * 1024, // 64MB
            allocation_count_threshold: 10000,
            enable_detailed_tracking: false,
            auto_gc_enabled: true,
        }
    }
}

impl GarbageCollector {
    /// Create a new garbage collector
    pub fn new() -> Self {
        Self::with_config(GcConfig::default())
    }
    
    /// Create a new garbage collector with custom configuration
    pub fn with_config(config: GcConfig) -> Self {
        let mut allocation_tracker = AllocationTracker::default();
        allocation_tracker.detailed_tracking = config.enable_detailed_tracking;
        
        Self {
            heap: Heap::new(),
            mark_stack: Vec::new(),
            roots: HashSet::new(),
            stats: GcStats::default(),
            allocation_tracker,
            config,
        }
    }
    
    /// Allocate memory for an object
    pub fn allocate<T: 'static>(&mut self, value: T) -> Result<NonNull<T>, RuntimeError> {
        self.allocate_with_type_id(value, TypeIdExt::into(std::any::TypeId::of::<T>()))
    }
    
    /// Allocate memory for an object with specific type ID
    pub fn allocate_with_type_id<T: 'static>(&mut self, value: T, type_id: u32) -> Result<NonNull<T>, RuntimeError> {
        let obj_size = std::mem::size_of::<T>();
        let total_size = obj_size + std::mem::size_of::<ObjectHeader>();
        
        // Check if we should trigger GC before allocation
        if self.config.auto_gc_enabled && self.should_collect() {
            self.collect();
        }
        
        let ptr = self.heap.allocate(total_size)?;
        
        // Initialize object header
        unsafe {
            let header = ptr.as_ptr() as *mut ObjectHeader;
            (*header) = ObjectHeader::new(total_size, type_id);
            
            // Initialize the actual object
            let obj_ptr = header.as_ref().unwrap().data_ptr() as *mut T;
            std::ptr::write(obj_ptr, value);
            
            // Track allocation for debugging
            self.track_allocation(header, total_size, type_id);
            
            // Update statistics
            self.stats.current_memory_usage += total_size;
            if self.stats.current_memory_usage > self.stats.peak_memory_usage {
                self.stats.peak_memory_usage = self.stats.current_memory_usage;
            }
            
            Ok(NonNull::new_unchecked(obj_ptr))
        }
    }
    
    /// Deallocate memory for an object
    pub unsafe fn deallocate<T: 'static>(&mut self, ptr: NonNull<T>) -> Result<(), RuntimeError> {
        let header_ptr = ObjectHeader::from_object_ptr(ptr.as_ptr());
        let header = &*header_ptr;
        let size = header.size;
        
        // Untrack allocation
        self.untrack_allocation(header_ptr);
        
        // Update statistics
        self.stats.current_memory_usage = self.stats.current_memory_usage.saturating_sub(size);
        
        // Deallocate from heap
        self.heap.deallocate(NonNull::new_unchecked(header_ptr as *mut u8), size)?;
        
        Ok(())
    }
    
    /// Check if garbage collection should be triggered
    fn should_collect(&self) -> bool {
        let heap_stats = self.heap.stats();
        heap_stats.current_allocated >= self.config.heap_size_threshold ||
        heap_stats.object_count >= self.config.allocation_count_threshold
    }
    
    /// Track an allocation for debugging
    fn track_allocation(&mut self, header_ptr: *mut ObjectHeader, size: usize, type_id: u32) {
        // Update type-based statistics
        let info = self.allocation_tracker.allocations_by_type
            .entry(type_id)
            .or_insert_with(|| AllocationInfo {
                count: 0,
                total_size: 0,
                peak_count: 0,
                peak_size: 0,
            });
        
        info.count += 1;
        info.total_size += size;
        
        if info.count > info.peak_count {
            info.peak_count = info.count;
        }
        if info.total_size > info.peak_size {
            info.peak_size = info.total_size;
        }
        
        // Track individual allocation if detailed tracking is enabled
        if self.allocation_tracker.detailed_tracking {
            self.allocation_tracker.active_allocations.insert(
                header_ptr,
                AllocationDebugInfo {
                    size,
                    type_id,
                    allocated_at: Instant::now(),
                    stack_trace: None, // TODO: Capture stack trace
                },
            );
        }
    }
    
    /// Untrack an allocation
    fn untrack_allocation(&mut self, header_ptr: *mut ObjectHeader) {
        if let Some(debug_info) = self.allocation_tracker.active_allocations.remove(&header_ptr) {
            // Update type-based statistics
            if let Some(info) = self.allocation_tracker.allocations_by_type.get_mut(&debug_info.type_id) {
                info.count = info.count.saturating_sub(1);
                info.total_size = info.total_size.saturating_sub(debug_info.size);
            }
        }
    }
    
    /// Add a root object that should not be collected
    pub fn add_root<T: 'static>(&mut self, ptr: NonNull<T>) {
        unsafe {
            let header_ptr = (ptr.as_ptr() as *mut u8)
                .sub(std::mem::size_of::<ObjectHeader>()) as *mut ObjectHeader;
            self.roots.insert(NonNull::new_unchecked(header_ptr));
        }
    }
    
    /// Remove a root object
    pub fn remove_root<T: 'static>(&mut self, ptr: NonNull<T>) {
        unsafe {
            let header_ptr = (ptr.as_ptr() as *mut u8)
                .sub(std::mem::size_of::<ObjectHeader>()) as *mut ObjectHeader;
            self.roots.remove(&NonNull::new_unchecked(header_ptr));
        }
    }
    
    /// Perform garbage collection
    pub fn collect(&mut self) {
        let start_time = Instant::now();
        let initial_memory = self.stats.current_memory_usage;
        let initial_objects = self.heap.stats().object_count;
        
        self.mark_phase();
        self.sweep_phase();
        
        // Update statistics
        let collection_time = start_time.elapsed();
        self.stats.collections_performed += 1;
        self.stats.total_collection_time += collection_time;
        
        let final_objects = self.heap.stats().object_count;
        let objects_collected = initial_objects.saturating_sub(final_objects);
        let bytes_collected = initial_memory.saturating_sub(self.stats.current_memory_usage);
        
        self.stats.objects_collected += objects_collected;
        self.stats.bytes_collected += bytes_collected;
        
        #[cfg(debug_assertions)]
        {
            println!("GC: Collected {} objects ({} bytes) in {:?}", 
                     objects_collected, bytes_collected, collection_time);
        }
    }
    
    /// Force garbage collection (for testing and debugging)
    pub fn force_collect(&mut self) {
        self.collect();
    }
    
    /// Mark phase: mark all reachable objects
    fn mark_phase(&mut self) {
        // Clear previous marks
        self.heap.clear_marks();
        
        // Mark all root objects
        let roots: Vec<_> = self.roots.iter().copied().collect();
        for root in roots {
            self.mark_object(root);
        }
        
        // Process mark stack
        while let Some(obj) = self.mark_stack.pop() {
            self.mark_object(obj);
        }
    }
    
    /// Mark an object as reachable
    fn mark_object(&mut self, obj: NonNull<ObjectHeader>) {
        unsafe {
            let header = obj.as_ref();
            if !header.marked {
                // Mark the object
                (obj.as_ptr() as *mut ObjectHeader).as_mut().unwrap().marked = true;
                
                // Add referenced objects to mark stack
                // This is a placeholder - real implementation would traverse object references
            }
        }
    }
    
    /// Sweep phase: deallocate unmarked objects
    fn sweep_phase(&mut self) {
        self.heap.sweep();
    }
    
    /// Get heap statistics
    pub fn heap_stats(&self) -> HeapStats {
        self.heap.stats()
    }
    
    /// Get garbage collection statistics
    pub fn gc_stats(&self) -> &GcStats {
        &self.stats
    }
    
    /// Get allocation statistics by type
    pub fn allocation_stats(&self) -> &HashMap<u32, AllocationInfo> {
        &self.allocation_tracker.allocations_by_type
    }
    
    /// Get detailed allocation information (debug mode only)
    pub fn active_allocations(&self) -> Option<&HashMap<*mut ObjectHeader, AllocationDebugInfo>> {
        if self.allocation_tracker.detailed_tracking {
            Some(&self.allocation_tracker.active_allocations)
        } else {
            None
        }
    }
    
    /// Enable or disable detailed allocation tracking
    pub fn set_detailed_tracking(&mut self, enabled: bool) {
        self.allocation_tracker.detailed_tracking = enabled;
        if !enabled {
            self.allocation_tracker.active_allocations.clear();
        }
    }
    
    /// Get memory usage summary
    pub fn memory_summary(&self) -> MemorySummary {
        let heap_stats = self.heap.stats();
        MemorySummary {
            current_usage: self.stats.current_memory_usage,
            peak_usage: self.stats.peak_memory_usage,
            total_allocated: heap_stats.total_allocated,
            total_freed: heap_stats.total_freed,
            active_objects: heap_stats.object_count,
            collections_performed: self.stats.collections_performed,
            total_collection_time: self.stats.total_collection_time,
        }
    }
}

impl Default for GarbageCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple heap implementation
pub struct Heap {
    allocated_objects: Vec<NonNull<ObjectHeader>>,
    total_allocated: usize,
    total_freed: usize,
    /// Free list for memory reuse (future optimization)
    free_list: Vec<(NonNull<u8>, usize)>,
}

impl Heap {
    /// Create a new heap
    pub fn new() -> Self {
        Self {
            allocated_objects: Vec::new(),
            total_allocated: 0,
            total_freed: 0,
            free_list: Vec::new(),
        }
    }
    
    /// Allocate memory
    pub fn allocate(&mut self, size: usize) -> Result<NonNull<u8>, RuntimeError> {
        // Try to reuse memory from free list first
        if let Some(index) = self.free_list.iter().position(|(_, free_size)| *free_size >= size) {
            let (ptr, _) = self.free_list.swap_remove(index);
            let obj_ptr = unsafe { NonNull::new_unchecked(ptr.as_ptr() as *mut ObjectHeader) };
            self.allocated_objects.push(obj_ptr);
            return Ok(ptr);
        }
        
        // Allocate new memory
        let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<ObjectHeader>())
            .map_err(|_| RuntimeError {
                kind: RuntimeErrorKind::OutOfMemory,
            })?;
        
        let ptr = unsafe { std::alloc::alloc(layout) };
        
        if ptr.is_null() {
            return Err(RuntimeError {
                kind: RuntimeErrorKind::OutOfMemory,
            });
        }
        
        let obj_ptr = unsafe { NonNull::new_unchecked(ptr as *mut ObjectHeader) };
        self.allocated_objects.push(obj_ptr);
        self.total_allocated += size;
        
        Ok(unsafe { NonNull::new_unchecked(ptr) })
    }
    
    /// Deallocate memory
    pub fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) -> Result<(), RuntimeError> {
        let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<ObjectHeader>())
            .map_err(|_| RuntimeError {
                kind: RuntimeErrorKind::OutOfMemory,
            })?;
        
        unsafe {
            std::alloc::dealloc(ptr.as_ptr(), layout);
        }
        
        self.total_freed += size;
        Ok(())
    }
    
    /// Clear all mark bits
    pub fn clear_marks(&mut self) {
        for &obj in &self.allocated_objects {
            unsafe {
                obj.as_ptr().as_mut().unwrap().marked = false;
            }
        }
    }
    
    /// Sweep unmarked objects
    pub fn sweep(&mut self) {
        let mut i = 0;
        while i < self.allocated_objects.len() {
            let obj = self.allocated_objects[i];
            unsafe {
                let header = obj.as_ref();
                if !header.marked {
                    // Deallocate unmarked object
                    let size = header.size;
                    let layout = std::alloc::Layout::from_size_align_unchecked(
                        size,
                        std::mem::align_of::<ObjectHeader>(),
                    );
                    std::alloc::dealloc(obj.as_ptr() as *mut u8, layout);
                    
                    self.total_freed += size;
                    self.allocated_objects.swap_remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }
    
    /// Get heap statistics
    pub fn stats(&self) -> HeapStats {
        HeapStats {
            total_allocated: self.total_allocated,
            total_freed: self.total_freed,
            current_allocated: self.total_allocated - self.total_freed,
            object_count: self.allocated_objects.len(),
        }
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

/// Heap statistics
#[derive(Debug, Clone)]
pub struct HeapStats {
    pub total_allocated: usize,
    pub total_freed: usize,
    pub current_allocated: usize,
    pub object_count: usize,
}

/// Memory usage summary
#[derive(Debug, Clone)]
pub struct MemorySummary {
    pub current_usage: usize,
    pub peak_usage: usize,
    pub total_allocated: usize,
    pub total_freed: usize,
    pub active_objects: usize,
    pub collections_performed: usize,
    pub total_collection_time: Duration,
}

impl std::fmt::Display for MemorySummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, 
            "Memory Summary:\n\
             Current Usage: {} bytes\n\
             Peak Usage: {} bytes\n\
             Total Allocated: {} bytes\n\
             Total Freed: {} bytes\n\
             Active Objects: {}\n\
             GC Collections: {}\n\
             Total GC Time: {:?}",
            self.current_usage,
            self.peak_usage,
            self.total_allocated,
            self.total_freed,
            self.active_objects,
            self.collections_performed,
            self.total_collection_time
        )
    }
}