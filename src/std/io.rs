//! I/O operations for the Flux standard library
//! 
//! This module provides file I/O operations, console I/O functions, and buffered I/O
//! with comprehensive error handling.

use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use crate::runtime::result::{FluxResult, FluxError};


/// Represents different types of I/O errors that can occur
#[derive(Debug, Clone)]
pub enum IoError {
    FileNotFound(String),
    PermissionDenied(String),
    InvalidPath(String),
    ReadError(String),
    WriteError(String),
    BufferError(String),
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::FileNotFound(path) => write!(f, "File not found: {}", path),
            IoError::PermissionDenied(path) => write!(f, "Permission denied: {}", path),
            IoError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            IoError::ReadError(msg) => write!(f, "Read error: {}", msg),
            IoError::WriteError(msg) => write!(f, "Write error: {}", msg),
            IoError::BufferError(msg) => write!(f, "Buffer error: {}", msg),
        }
    }
}

impl From<io::Error> for IoError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => IoError::FileNotFound(error.to_string()),
            io::ErrorKind::PermissionDenied => IoError::PermissionDenied(error.to_string()),
            io::ErrorKind::InvalidInput => IoError::InvalidPath(error.to_string()),
            _ => IoError::ReadError(error.to_string()),
        }
    }
}

impl From<IoError> for FluxError {
    fn from(error: IoError) -> Self {
        FluxError::Custom(error.to_string())
    }
}

/// File handle for I/O operations with buffering support
pub struct File {
    inner: fs::File,
    path: String,
    buffered_reader: Option<BufReader<fs::File>>,
    buffered_writer: Option<BufWriter<fs::File>>,
}

impl File {
    /// Opens a file for reading
    pub fn open<P: AsRef<Path>>(path: P) -> FluxResult<Self, IoError> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        
        match fs::File::open(&path) {
            Ok(file) => FluxResult::Ok(File {
                inner: file,
                path: path_str,
                buffered_reader: None,
                buffered_writer: None,
            }),
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Creates a new file for writing
    pub fn create<P: AsRef<Path>>(path: P) -> FluxResult<Self, IoError> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        
        match fs::File::create(&path) {
            Ok(file) => FluxResult::Ok(File {
                inner: file,
                path: path_str,
                buffered_reader: None,
                buffered_writer: None,
            }),
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Opens a file for both reading and writing
    pub fn open_rw<P: AsRef<Path>>(path: P) -> FluxResult<Self, IoError> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        
        match fs::OpenOptions::new().read(true).write(true).open(&path) {
            Ok(file) => FluxResult::Ok(File {
                inner: file,
                path: path_str,
                buffered_reader: None,
                buffered_writer: None,
            }),
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Reads the entire file content as a string
    pub fn read_to_string(&mut self) -> FluxResult<String, IoError> {
        let mut contents = String::new();
        match self.inner.read_to_string(&mut contents) {
            Ok(_) => FluxResult::Ok(contents),
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Reads the file content as bytes
    pub fn read_to_bytes(&mut self) -> FluxResult<Vec<u8>, IoError> {
        let mut buffer = Vec::new();
        match self.inner.read_to_end(&mut buffer) {
            Ok(_) => FluxResult::Ok(buffer),
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Writes a string to the file
    pub fn write_string(&mut self, content: &str) -> FluxResult<(), IoError> {
        match self.inner.write_all(content.as_bytes()) {
            Ok(_) => {
                match self.inner.flush() {
                    Ok(_) => FluxResult::Ok(()),
                    Err(e) => FluxResult::Err(IoError::from(e)),
                }
            },
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Writes bytes to the file
    pub fn write_bytes(&mut self, data: &[u8]) -> FluxResult<(), IoError> {
        match self.inner.write_all(data) {
            Ok(_) => {
                match self.inner.flush() {
                    Ok(_) => FluxResult::Ok(()),
                    Err(e) => FluxResult::Err(IoError::from(e)),
                }
            },
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Enables buffered reading for better performance
    pub fn enable_buffered_reading(&mut self) -> FluxResult<(), IoError> {
        // We need to create a new file handle for buffering
        match fs::File::open(&self.path) {
            Ok(file) => {
                self.buffered_reader = Some(BufReader::new(file));
                FluxResult::Ok(())
            },
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Enables buffered writing for better performance
    pub fn enable_buffered_writing(&mut self) -> FluxResult<(), IoError> {
        // We need to create a new file handle for buffering
        match fs::OpenOptions::new().write(true).open(&self.path) {
            Ok(file) => {
                self.buffered_writer = Some(BufWriter::new(file));
                FluxResult::Ok(())
            },
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Reads a line from the file (requires buffered reading)
    pub fn read_line(&mut self) -> FluxResult<String, IoError> {
        if self.buffered_reader.is_none() {
            match self.enable_buffered_reading() {
                FluxResult::Ok(_) => {},
                FluxResult::Err(e) => return FluxResult::Err(e),
            }
        }
        
        let mut line = String::new();
        match self.buffered_reader.as_mut().unwrap().read_line(&mut line) {
            Ok(0) => FluxResult::Ok(String::new()), // EOF
            Ok(_) => {
                // Remove trailing newline
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                FluxResult::Ok(line)
            },
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Writes a line to the file with buffering
    pub fn write_line(&mut self, line: &str) -> FluxResult<(), IoError> {
        if self.buffered_writer.is_none() {
            match self.enable_buffered_writing() {
                FluxResult::Ok(_) => {},
                FluxResult::Err(e) => return FluxResult::Err(e),
            }
        }
        
        let writer = self.buffered_writer.as_mut().unwrap();
        match writeln!(writer, "{}", line) {
            Ok(_) => FluxResult::Ok(()),
            Err(e) => FluxResult::Err(IoError::WriteError(e.to_string())),
        }
    }
    
    /// Flushes any buffered data to disk
    pub fn flush(&mut self) -> FluxResult<(), IoError> {
        if let Some(ref mut writer) = self.buffered_writer {
            match writer.flush() {
                Ok(_) => {},
                Err(e) => return FluxResult::Err(IoError::from(e)),
            }
        }
        match self.inner.flush() {
            Ok(_) => FluxResult::Ok(()),
            Err(e) => FluxResult::Err(IoError::from(e)),
        }
    }
    
    /// Gets the file path
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Console I/O functions

/// Prints a message to stdout without a newline
pub fn print(message: &str) -> FluxResult<(), IoError> {
    match io::stdout().write_all(message.as_bytes()) {
        Ok(_) => {
            match io::stdout().flush() {
                Ok(_) => FluxResult::Ok(()),
                Err(e) => FluxResult::Err(IoError::from(e)),
            }
        },
        Err(e) => FluxResult::Err(IoError::from(e)),
    }
}

/// Prints a message to stdout with a newline
pub fn println(message: &str) -> FluxResult<(), IoError> {
    match println!("{}", message) {
        _ => FluxResult::Ok(()),
    }
}

/// Reads a line from stdin
pub fn read_line() -> FluxResult<String, IoError> {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            // Remove trailing newline
            if input.ends_with('\n') {
                input.pop();
                if input.ends_with('\r') {
                    input.pop();
                }
            }
            FluxResult::Ok(input)
        },
        Err(e) => FluxResult::Err(IoError::from(e)),
    }
}

/// Reads input with a prompt
pub fn input(prompt: &str) -> FluxResult<String, IoError> {
    match print(prompt) {
        FluxResult::Ok(_) => read_line(),
        FluxResult::Err(e) => FluxResult::Err(e),
    }
}

/// Utility functions for file operations

/// Reads entire file content as string
pub fn read_file<P: AsRef<Path>>(path: P) -> FluxResult<String, IoError> {
    match fs::read_to_string(path) {
        Ok(content) => FluxResult::Ok(content),
        Err(e) => FluxResult::Err(IoError::from(e)),
    }
}

/// Writes string content to file
pub fn write_file<P: AsRef<Path>>(path: P, content: &str) -> FluxResult<(), IoError> {
    match fs::write(path, content) {
        Ok(_) => FluxResult::Ok(()),
        Err(e) => FluxResult::Err(IoError::from(e)),
    }
}

/// Appends string content to file
pub fn append_file<P: AsRef<Path>>(path: P, content: &str) -> FluxResult<(), IoError> {
    match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => {
            match file.write_all(content.as_bytes()) {
                Ok(_) => FluxResult::Ok(()),
                Err(e) => FluxResult::Err(IoError::from(e)),
            }
        },
        Err(e) => FluxResult::Err(IoError::from(e)),
    }
}

/// Checks if a file exists
pub fn file_exists<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

/// Deletes a file
pub fn delete_file<P: AsRef<Path>>(path: P) -> FluxResult<(), IoError> {
    match fs::remove_file(path) {
        Ok(_) => FluxResult::Ok(()),
        Err(e) => FluxResult::Err(IoError::from(e)),
    }
}

/// Creates a directory
pub fn create_dir<P: AsRef<Path>>(path: P) -> FluxResult<(), IoError> {
    match fs::create_dir_all(path) {
        Ok(_) => FluxResult::Ok(()),
        Err(e) => FluxResult::Err(IoError::from(e)),
    }
}

/// Lists directory contents
pub fn list_dir<P: AsRef<Path>>(path: P) -> FluxResult<Vec<String>, IoError> {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut files = Vec::new();
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        if let Some(name) = entry.file_name().to_str() {
                            files.push(name.to_string());
                        }
                    },
                    Err(e) => return FluxResult::Err(IoError::from(e)),
                }
            }
            FluxResult::Ok(files)
        },
        Err(e) => FluxResult::Err(IoError::from(e)),
    }
}