//! Collections module for the Flux standard library
//! 
//! This module provides collection types like Array, List, Map, and Set
//! with bounds checking, dynamic resizing, and performance optimizations.

use std::collections::HashMap as StdHashMap;
use std::collections::HashSet as StdHashSet;
use std::hash::Hash;
use std::fmt;
use crate::runtime::result::{FluxResult, FluxError};

/// Represents different types of collection errors that can occur
#[derive(Debug, Clone)]
pub enum CollectionError {
    IndexOutOfBounds { index: usize, length: usize },
    KeyNotFound(String),
    InvalidCapacity(usize),
    EmptyCollection,
    DuplicateKey(String),
}

impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CollectionError::IndexOutOfBounds { index, length } => {
                write!(f, "Index {} out of bounds for length {}", index, length)
            },
            CollectionError::KeyNotFound(key) => write!(f, "Key not found: {}", key),
            CollectionError::InvalidCapacity(cap) => write!(f, "Invalid capacity: {}", cap),
            CollectionError::EmptyCollection => write!(f, "Operation on empty collection"),
            CollectionError::DuplicateKey(key) => write!(f, "Duplicate key: {}", key),
        }
    }
}

impl From<CollectionError> for FluxError {
    fn from(error: CollectionError) -> Self {
        FluxError::Custom(error.to_string())
    }
}

/// Fixed-size array with bounds checking
#[derive(Debug, Clone, PartialEq)]
pub struct Array<T> {
    data: Vec<T>,
    capacity: usize,
}

impl<T> Array<T> {
    /// Creates a new array with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Array {
            data: Vec::with_capacity(capacity),
            capacity,
        }
    }
    
    /// Creates a new array with initial values
    pub fn from_vec(data: Vec<T>) -> Self {
        let capacity = data.len();
        Array { data, capacity }
    }
    
    /// Gets an element at the specified index with bounds checking
    pub fn get(&self, index: usize) -> FluxResult<&T, CollectionError> {
        if index >= self.data.len() {
            FluxResult::Err(CollectionError::IndexOutOfBounds {
                index,
                length: self.data.len(),
            })
        } else {
            FluxResult::Ok(&self.data[index])
        }
    }
    
    /// Gets a mutable reference to an element at the specified index
    pub fn get_mut(&mut self, index: usize) -> FluxResult<&mut T, CollectionError> {
        let length = self.data.len();
        if index >= length {
            FluxResult::Err(CollectionError::IndexOutOfBounds { index, length })
        } else {
            FluxResult::Ok(&mut self.data[index])
        }
    }
    
    /// Sets an element at the specified index
    pub fn set(&mut self, index: usize, value: T) -> FluxResult<(), CollectionError> {
        if index >= self.capacity {
            FluxResult::Err(CollectionError::IndexOutOfBounds {
                index,
                length: self.capacity,
            })
        } else {
            // Extend the vector if necessary
            while self.data.len() <= index {
                // This requires T to have a default value, which we can't assume
                // For now, we'll require the array to be filled sequentially
                return FluxResult::Err(CollectionError::IndexOutOfBounds {
                    index,
                    length: self.data.len(),
                });
            }
            self.data[index] = value;
            FluxResult::Ok(())
        }
    }
    
    /// Pushes an element to the end of the array if there's space
    pub fn push(&mut self, value: T) -> FluxResult<(), CollectionError> {
        if self.data.len() >= self.capacity {
            FluxResult::Err(CollectionError::IndexOutOfBounds {
                index: self.data.len(),
                length: self.capacity,
            })
        } else {
            self.data.push(value);
            FluxResult::Ok(())
        }
    }
    
    /// Pops an element from the end of the array
    pub fn pop(&mut self) -> FluxResult<T, CollectionError> {
        self.data.pop().ok_or(CollectionError::EmptyCollection)
            .map_err(|e| e)
            .map(|v| v)
            .into()
    }
    
    /// Returns the length of the array
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Returns the capacity of the array
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    
    /// Returns true if the array is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Returns true if the array is full
    pub fn is_full(&self) -> bool {
        self.data.len() >= self.capacity
    }
    
    /// Clears all elements from the array
    pub fn clear(&mut self) {
        self.data.clear();
    }
    
    /// Returns an iterator over the array elements
    pub fn iter(&self) -> std::slice::Iter<T> {
        self.data.iter()
    }
    
    /// Returns a mutable iterator over the array elements
    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        self.data.iter_mut()
    }
}

/// Dynamic list with automatic resizing
#[derive(Debug, Clone, PartialEq)]
pub struct List<T> {
    data: Vec<T>,
}

impl<T> List<T> {
    /// Creates a new empty list
    pub fn new() -> Self {
        List {
            data: Vec::new(),
        }
    }
    
    /// Creates a new list with the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        List {
            data: Vec::with_capacity(capacity),
        }
    }
    
    /// Creates a list from a vector
    pub fn from_vec(data: Vec<T>) -> Self {
        List { data }
    }
    
    /// Gets an element at the specified index with bounds checking
    pub fn get(&self, index: usize) -> FluxResult<&T, CollectionError> {
        if index >= self.data.len() {
            FluxResult::Err(CollectionError::IndexOutOfBounds {
                index,
                length: self.data.len(),
            })
        } else {
            FluxResult::Ok(&self.data[index])
        }
    }
    
    /// Gets a mutable reference to an element at the specified index
    pub fn get_mut(&mut self, index: usize) -> FluxResult<&mut T, CollectionError> {
        let length = self.data.len();
        if index >= length {
            FluxResult::Err(CollectionError::IndexOutOfBounds { index, length })
        } else {
            FluxResult::Ok(&mut self.data[index])
        }
    }
    
    /// Sets an element at the specified index
    pub fn set(&mut self, index: usize, value: T) -> FluxResult<(), CollectionError> {
        let length = self.data.len();
        if index >= length {
            FluxResult::Err(CollectionError::IndexOutOfBounds { index, length })
        } else {
            self.data[index] = value;
            FluxResult::Ok(())
        }
    }
    
    /// Pushes an element to the end of the list
    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }
    
    /// Pops an element from the end of the list
    pub fn pop(&mut self) -> FluxResult<T, CollectionError> {
        match self.data.pop() {
            Some(value) => FluxResult::Ok(value),
            None => FluxResult::Err(CollectionError::EmptyCollection),
        }
    }
    
    /// Inserts an element at the specified index
    pub fn insert(&mut self, index: usize, value: T) -> FluxResult<(), CollectionError> {
        if index > self.data.len() {
            FluxResult::Err(CollectionError::IndexOutOfBounds {
                index,
                length: self.data.len(),
            })
        } else {
            self.data.insert(index, value);
            FluxResult::Ok(())
        }
    }
    
    /// Removes an element at the specified index
    pub fn remove(&mut self, index: usize) -> FluxResult<T, CollectionError> {
        if index >= self.data.len() {
            FluxResult::Err(CollectionError::IndexOutOfBounds {
                index,
                length: self.data.len(),
            })
        } else {
            FluxResult::Ok(self.data.remove(index))
        }
    }
    
    /// Returns the length of the list
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Returns the capacity of the list
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }
    
    /// Returns true if the list is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Clears all elements from the list
    pub fn clear(&mut self) {
        self.data.clear();
    }
    
    /// Reserves capacity for at least additional more elements
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }
    
    /// Shrinks the capacity to fit the current length
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }
    
    /// Returns an iterator over the list elements
    pub fn iter(&self) -> std::slice::Iter<T> {
        self.data.iter()
    }
    
    /// Returns a mutable iterator over the list elements
    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        self.data.iter_mut()
    }
    
    /// Extends the list with elements from an iterator
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.data.extend(iter);
    }
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash map implementation with key-value pairs
#[derive(Debug, Clone)]
pub struct Map<K, V> 
where
    K: Eq + Hash + Clone,
{
    data: StdHashMap<K, V>,
}

impl<K, V> Map<K, V> 
where
    K: Eq + Hash + Clone + fmt::Display,
{
    /// Creates a new empty map
    pub fn new() -> Self {
        Map {
            data: StdHashMap::new(),
        }
    }
    
    /// Creates a new map with the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Map {
            data: StdHashMap::with_capacity(capacity),
        }
    }
    
    /// Inserts a key-value pair into the map
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.data.insert(key, value)
    }
    
    /// Gets a value by key
    pub fn get(&self, key: &K) -> FluxResult<&V, CollectionError> {
        match self.data.get(key) {
            Some(value) => FluxResult::Ok(value),
            None => FluxResult::Err(CollectionError::KeyNotFound(key.to_string())),
        }
    }
    
    /// Gets a mutable reference to a value by key
    pub fn get_mut(&mut self, key: &K) -> FluxResult<&mut V, CollectionError> {
        match self.data.get_mut(key) {
            Some(value) => FluxResult::Ok(value),
            None => FluxResult::Err(CollectionError::KeyNotFound(key.to_string())),
        }
    }
    
    /// Removes a key-value pair from the map
    pub fn remove(&mut self, key: &K) -> FluxResult<V, CollectionError> {
        match self.data.remove(key) {
            Some(value) => FluxResult::Ok(value),
            None => FluxResult::Err(CollectionError::KeyNotFound(key.to_string())),
        }
    }
    
    /// Returns true if the map contains the specified key
    pub fn contains_key(&self, key: &K) -> bool {
        self.data.contains_key(key)
    }
    
    /// Returns the number of key-value pairs in the map
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Returns true if the map is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Clears all key-value pairs from the map
    pub fn clear(&mut self) {
        self.data.clear();
    }
    
    /// Returns an iterator over the key-value pairs
    pub fn iter(&self) -> std::collections::hash_map::Iter<K, V> {
        self.data.iter()
    }
    
    /// Returns a mutable iterator over the key-value pairs
    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<K, V> {
        self.data.iter_mut()
    }
    
    /// Returns an iterator over the keys
    pub fn keys(&self) -> std::collections::hash_map::Keys<K, V> {
        self.data.keys()
    }
    
    /// Returns an iterator over the values
    pub fn values(&self) -> std::collections::hash_map::Values<K, V> {
        self.data.values()
    }
    
    /// Returns a mutable iterator over the values
    pub fn values_mut(&mut self) -> std::collections::hash_map::ValuesMut<K, V> {
        self.data.values_mut()
    }
    
    /// Reserves capacity for at least additional more elements
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }
    
    /// Shrinks the capacity to fit the current length
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }
}

impl<K, V> Default for Map<K, V> 
where
    K: Eq + Hash + Clone + fmt::Display,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Hash set implementation with uniqueness guarantees
#[derive(Debug, Clone)]
pub struct Set<T> 
where
    T: Eq + Hash + Clone,
{
    data: StdHashSet<T>,
}

impl<T> Set<T> 
where
    T: Eq + Hash + Clone + fmt::Display,
{
    /// Creates a new empty set
    pub fn new() -> Self {
        Set {
            data: StdHashSet::new(),
        }
    }
    
    /// Creates a new set with the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Set {
            data: StdHashSet::with_capacity(capacity),
        }
    }
    
    /// Inserts a value into the set
    pub fn insert(&mut self, value: T) -> bool {
        self.data.insert(value)
    }
    
    /// Removes a value from the set
    pub fn remove(&mut self, value: &T) -> bool {
        self.data.remove(value)
    }
    
    /// Returns true if the set contains the specified value
    pub fn contains(&self, value: &T) -> bool {
        self.data.contains(value)
    }
    
    /// Returns the number of values in the set
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Returns true if the set is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Clears all values from the set
    pub fn clear(&mut self) {
        self.data.clear();
    }
    
    /// Returns an iterator over the set values
    pub fn iter(&self) -> std::collections::hash_set::Iter<T> {
        self.data.iter()
    }
    
    /// Returns the union of this set and another set
    pub fn union<'a>(&'a self, other: &'a Set<T>) -> std::collections::hash_set::Union<'a, T, std::collections::hash_map::RandomState> {
        self.data.union(&other.data)
    }
    
    /// Returns the intersection of this set and another set
    pub fn intersection<'a>(&'a self, other: &'a Set<T>) -> std::collections::hash_set::Intersection<'a, T, std::collections::hash_map::RandomState> {
        self.data.intersection(&other.data)
    }
    
    /// Returns the difference of this set and another set
    pub fn difference<'a>(&'a self, other: &'a Set<T>) -> std::collections::hash_set::Difference<'a, T, std::collections::hash_map::RandomState> {
        self.data.difference(&other.data)
    }
    
    /// Returns the symmetric difference of this set and another set
    pub fn symmetric_difference<'a>(&'a self, other: &'a Set<T>) -> std::collections::hash_set::SymmetricDifference<'a, T, std::collections::hash_map::RandomState> {
        self.data.symmetric_difference(&other.data)
    }
    
    /// Returns true if this set is a subset of another set
    pub fn is_subset(&self, other: &Set<T>) -> bool {
        self.data.is_subset(&other.data)
    }
    
    /// Returns true if this set is a superset of another set
    pub fn is_superset(&self, other: &Set<T>) -> bool {
        self.data.is_superset(&other.data)
    }
    
    /// Returns true if this set is disjoint from another set
    pub fn is_disjoint(&self, other: &Set<T>) -> bool {
        self.data.is_disjoint(&other.data)
    }
    
    /// Reserves capacity for at least additional more elements
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }
    
    /// Shrinks the capacity to fit the current length
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }
}

impl<T> Default for Set<T> 
where
    T: Eq + Hash + Clone + fmt::Display,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for collections

/// Creates a new array from a slice
pub fn array_from_slice<T: Clone>(slice: &[T]) -> Array<T> {
    Array::from_vec(slice.to_vec())
}

/// Creates a new list from a slice
pub fn list_from_slice<T: Clone>(slice: &[T]) -> List<T> {
    List::from_vec(slice.to_vec())
}

/// Creates a new map from key-value pairs
pub fn map_from_pairs<K, V>(pairs: Vec<(K, V)>) -> Map<K, V> 
where
    K: Eq + Hash + Clone + fmt::Display,
{
    let mut map = Map::new();
    for (key, value) in pairs {
        map.insert(key, value);
    }
    map
}

/// Creates a new set from values
pub fn set_from_values<T>(values: Vec<T>) -> Set<T> 
where
    T: Eq + Hash + Clone + fmt::Display,
{
    let mut set = Set::new();
    for value in values {
        set.insert(value);
    }
    set
}