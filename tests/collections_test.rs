//! Tests for the Flux standard library collections module

use flux_compiler::std::collections::*;
use flux_compiler::runtime::result::FluxResult;

#[test]
fn test_array_creation_and_basic_operations() {
    let mut array = Array::<i32>::new(5);
    
    assert_eq!(array.len(), 0);
    assert_eq!(array.capacity(), 5);
    assert!(array.is_empty());
    assert!(!array.is_full());
    
    // Test push operations
    array.push(1).unwrap();
    array.push(2).unwrap();
    array.push(3).unwrap();
    
    assert_eq!(array.len(), 3);
    assert!(!array.is_empty());
    assert!(!array.is_full());
    
    // Test get operations
    assert_eq!(*array.get(0).unwrap(), 1);
    assert_eq!(*array.get(1).unwrap(), 2);
    assert_eq!(*array.get(2).unwrap(), 3);
    
    // Test bounds checking
    assert!(array.get(5).is_err());
}

#[test]
fn test_array_bounds_checking() {
    let mut array = Array::<i32>::new(3);
    
    // Fill the array
    array.push(1).unwrap();
    array.push(2).unwrap();
    array.push(3).unwrap();
    
    assert!(array.is_full());
    
    // Try to push beyond capacity
    let result = array.push(4);
    assert!(result.is_err());
    
    // Test pop operation
    let popped = array.pop().unwrap();
    assert_eq!(popped, 3);
    assert_eq!(array.len(), 2);
    assert!(!array.is_full());
}

#[test]
fn test_array_from_vec() {
    let vec = vec![1, 2, 3, 4, 5];
    let array = Array::from_vec(vec);
    
    assert_eq!(array.len(), 5);
    assert_eq!(array.capacity(), 5);
    assert!(array.is_full());
    
    for i in 0..5 {
        assert_eq!(*array.get(i).unwrap(), i as i32 + 1);
    }
}

#[test]
fn test_list_creation_and_basic_operations() {
    let mut list = List::<i32>::new();
    
    assert_eq!(list.len(), 0);
    assert!(list.is_empty());
    
    // Test push operations
    list.push(1);
    list.push(2);
    list.push(3);
    
    assert_eq!(list.len(), 3);
    assert!(!list.is_empty());
    
    // Test get operations
    assert_eq!(*list.get(0).unwrap(), 1);
    assert_eq!(*list.get(1).unwrap(), 2);
    assert_eq!(*list.get(2).unwrap(), 3);
    
    // Test bounds checking
    assert!(list.get(5).is_err());
}

#[test]
fn test_list_dynamic_operations() {
    let mut list = List::<i32>::new();
    
    // Test insert operations
    list.insert(0, 1).unwrap();
    list.insert(1, 3).unwrap();
    list.insert(1, 2).unwrap(); // Insert in the middle
    
    assert_eq!(list.len(), 3);
    assert_eq!(*list.get(0).unwrap(), 1);
    assert_eq!(*list.get(1).unwrap(), 2);
    assert_eq!(*list.get(2).unwrap(), 3);
    
    // Test remove operations
    let removed = list.remove(1).unwrap();
    assert_eq!(removed, 2);
    assert_eq!(list.len(), 2);
    assert_eq!(*list.get(0).unwrap(), 1);
    assert_eq!(*list.get(1).unwrap(), 3);
    
    // Test pop operation
    let popped = list.pop().unwrap();
    assert_eq!(popped, 3);
    assert_eq!(list.len(), 1);
}

#[test]
fn test_list_capacity_management() {
    let mut list = List::<i32>::with_capacity(10);
    
    assert_eq!(list.len(), 0);
    assert!(list.capacity() >= 10);
    
    // Add elements
    for i in 0..5 {
        list.push(i);
    }
    
    // Reserve more capacity
    list.reserve(20);
    assert!(list.capacity() >= 25);
    
    // Shrink to fit
    list.shrink_to_fit();
    assert_eq!(list.capacity(), list.len());
}

#[test]
fn test_list_extend() {
    let mut list = List::<i32>::new();
    list.push(1);
    list.push(2);
    
    let additional = vec![3, 4, 5];
    list.extend(additional);
    
    assert_eq!(list.len(), 5);
    for i in 0..5 {
        assert_eq!(*list.get(i).unwrap(), i as i32 + 1);
    }
}

#[test]
fn test_map_creation_and_basic_operations() {
    let mut map = Map::<String, i32>::new();
    
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
    
    // Test insert operations
    map.insert("one".to_string(), 1);
    map.insert("two".to_string(), 2);
    map.insert("three".to_string(), 3);
    
    assert_eq!(map.len(), 3);
    assert!(!map.is_empty());
    
    // Test get operations
    assert_eq!(*map.get(&"one".to_string()).unwrap(), 1);
    assert_eq!(*map.get(&"two".to_string()).unwrap(), 2);
    assert_eq!(*map.get(&"three".to_string()).unwrap(), 3);
    
    // Test key not found
    assert!(map.get(&"four".to_string()).is_err());
}

#[test]
fn test_map_key_operations() {
    let mut map = Map::<String, i32>::new();
    
    map.insert("key1".to_string(), 100);
    map.insert("key2".to_string(), 200);
    
    // Test contains_key
    assert!(map.contains_key(&"key1".to_string()));
    assert!(map.contains_key(&"key2".to_string()));
    assert!(!map.contains_key(&"key3".to_string()));
    
    // Test remove
    let removed = map.remove(&"key1".to_string()).unwrap();
    assert_eq!(removed, 100);
    assert_eq!(map.len(), 1);
    assert!(!map.contains_key(&"key1".to_string()));
    
    // Test remove non-existent key
    assert!(map.remove(&"nonexistent".to_string()).is_err());
}

#[test]
fn test_map_iterators() {
    let mut map = Map::<String, i32>::new();
    map.insert("a".to_string(), 1);
    map.insert("b".to_string(), 2);
    map.insert("c".to_string(), 3);
    
    // Test keys iterator
    let keys: Vec<_> = map.keys().cloned().collect();
    assert_eq!(keys.len(), 3);
    assert!(keys.contains(&"a".to_string()));
    assert!(keys.contains(&"b".to_string()));
    assert!(keys.contains(&"c".to_string()));
    
    // Test values iterator
    let values: Vec<_> = map.values().cloned().collect();
    assert_eq!(values.len(), 3);
    assert!(values.contains(&1));
    assert!(values.contains(&2));
    assert!(values.contains(&3));
    
    // Test iter
    let pairs: Vec<_> = map.iter().map(|(k, v)| (k.clone(), *v)).collect();
    assert_eq!(pairs.len(), 3);
}

#[test]
fn test_set_creation_and_basic_operations() {
    let mut set = Set::<i32>::new();
    
    assert_eq!(set.len(), 0);
    assert!(set.is_empty());
    
    // Test insert operations
    assert!(set.insert(1));
    assert!(set.insert(2));
    assert!(set.insert(3));
    
    // Test duplicate insert
    assert!(!set.insert(1)); // Should return false for duplicate
    
    assert_eq!(set.len(), 3);
    assert!(!set.is_empty());
    
    // Test contains
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
    assert!(!set.contains(&4));
}

#[test]
fn test_set_remove_operations() {
    let mut set = Set::<i32>::new();
    
    set.insert(1);
    set.insert(2);
    set.insert(3);
    
    // Test remove existing element
    assert!(set.remove(&2));
    assert_eq!(set.len(), 2);
    assert!(!set.contains(&2));
    
    // Test remove non-existent element
    assert!(!set.remove(&4));
    assert_eq!(set.len(), 2);
}

#[test]
fn test_set_operations() {
    let mut set1 = Set::<i32>::new();
    set1.insert(1);
    set1.insert(2);
    set1.insert(3);
    
    let mut set2 = Set::<i32>::new();
    set2.insert(2);
    set2.insert(3);
    set2.insert(4);
    
    // Test union
    let union: Vec<_> = set1.union(&set2).cloned().collect();
    assert_eq!(union.len(), 4);
    assert!(union.contains(&1));
    assert!(union.contains(&2));
    assert!(union.contains(&3));
    assert!(union.contains(&4));
    
    // Test intersection
    let intersection: Vec<_> = set1.intersection(&set2).cloned().collect();
    assert_eq!(intersection.len(), 2);
    assert!(intersection.contains(&2));
    assert!(intersection.contains(&3));
    
    // Test difference
    let difference: Vec<_> = set1.difference(&set2).cloned().collect();
    assert_eq!(difference.len(), 1);
    assert!(difference.contains(&1));
    
    // Test subset/superset
    let mut subset = Set::<i32>::new();
    subset.insert(1);
    subset.insert(2);
    
    assert!(subset.is_subset(&set1));
    assert!(set1.is_superset(&subset));
    assert!(!set1.is_disjoint(&set2));
}

#[test]
fn test_utility_functions() {
    // Test array_from_slice
    let slice = &[1, 2, 3, 4, 5];
    let array = array_from_slice(slice);
    assert_eq!(array.len(), 5);
    assert_eq!(*array.get(0).unwrap(), 1);
    assert_eq!(*array.get(4).unwrap(), 5);
    
    // Test list_from_slice
    let list = list_from_slice(slice);
    assert_eq!(list.len(), 5);
    assert_eq!(*list.get(0).unwrap(), 1);
    assert_eq!(*list.get(4).unwrap(), 5);
    
    // Test map_from_pairs
    let pairs = vec![
        ("one".to_string(), 1),
        ("two".to_string(), 2),
        ("three".to_string(), 3),
    ];
    let map = map_from_pairs(pairs);
    assert_eq!(map.len(), 3);
    assert_eq!(*map.get(&"one".to_string()).unwrap(), 1);
    assert_eq!(*map.get(&"two".to_string()).unwrap(), 2);
    assert_eq!(*map.get(&"three".to_string()).unwrap(), 3);
    
    // Test set_from_values
    let values = vec![1, 2, 3, 2, 1]; // Duplicates should be removed
    let set = set_from_values(values);
    assert_eq!(set.len(), 3);
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
}

#[test]
fn test_error_handling() {
    // Test array bounds error
    let array = Array::<i32>::new(3);
    match array.get(5) {
        FluxResult::Err(CollectionError::IndexOutOfBounds { index, length }) => {
            assert_eq!(index, 5);
            assert_eq!(length, 0);
        },
        _ => panic!("Expected IndexOutOfBounds error"),
    }
    
    // Test list bounds error
    let list = List::<i32>::new();
    match list.get(0) {
        FluxResult::Err(CollectionError::IndexOutOfBounds { index, length }) => {
            assert_eq!(index, 0);
            assert_eq!(length, 0);
        },
        _ => panic!("Expected IndexOutOfBounds error"),
    }
    
    // Test map key not found error
    let map = Map::<String, i32>::new();
    match map.get(&"nonexistent".to_string()) {
        FluxResult::Err(CollectionError::KeyNotFound(key)) => {
            assert_eq!(key, "nonexistent");
        },
        _ => panic!("Expected KeyNotFound error"),
    }
    
    // Test empty collection error
    let mut list = List::<i32>::new();
    match list.pop() {
        FluxResult::Err(CollectionError::EmptyCollection) => {
            // Expected
        },
        _ => panic!("Expected EmptyCollection error"),
    }
}

#[test]
fn test_iterators() {
    // Test array iterator
    let array = Array::from_vec(vec![1, 2, 3, 4, 5]);
    let collected: Vec<_> = array.iter().cloned().collect();
    assert_eq!(collected, vec![1, 2, 3, 4, 5]);
    
    // Test list iterator
    let list = List::from_vec(vec![1, 2, 3, 4, 5]);
    let collected: Vec<_> = list.iter().cloned().collect();
    assert_eq!(collected, vec![1, 2, 3, 4, 5]);
    
    // Test mutable iterators
    let mut list = List::from_vec(vec![1, 2, 3]);
    for item in list.iter_mut() {
        *item *= 2;
    }
    assert_eq!(*list.get(0).unwrap(), 2);
    assert_eq!(*list.get(1).unwrap(), 4);
    assert_eq!(*list.get(2).unwrap(), 6);
}

#[test]
fn test_performance_characteristics() {
    // Test that List can handle large numbers of elements
    let mut list = List::<i32>::new();
    
    // Add 10,000 elements
    for i in 0..10_000 {
        list.push(i);
    }
    
    assert_eq!(list.len(), 10_000);
    assert_eq!(*list.get(0).unwrap(), 0);
    assert_eq!(*list.get(9_999).unwrap(), 9_999);
    
    // Test that Map can handle many key-value pairs
    let mut map = Map::<i32, String>::new();
    
    for i in 0..1_000 {
        map.insert(i, format!("value_{}", i));
    }
    
    assert_eq!(map.len(), 1_000);
    assert_eq!(*map.get(&0).unwrap(), "value_0");
    assert_eq!(*map.get(&999).unwrap(), "value_999");
    
    // Test that Set can handle many unique values
    let mut set = Set::<i32>::new();
    
    for i in 0..1_000 {
        set.insert(i);
    }
    
    assert_eq!(set.len(), 1_000);
    assert!(set.contains(&0));
    assert!(set.contains(&999));
}