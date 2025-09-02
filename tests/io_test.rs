//! Tests for the Flux standard library I/O module

use flux_compiler::std::io::*;
use flux_compiler::runtime::result::FluxResult;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_file_create_and_write() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_write.txt");
    
    let mut file = File::create(&file_path).unwrap();
    let content = "Hello, Flux!";
    
    file.write_string(content).unwrap();
    
    // Verify file was created and content written
    let written_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(written_content, content);
}

#[test]
fn test_file_open_and_read() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_read.txt");
    let content = "Test content for reading";
    
    // Create file with content
    fs::write(&file_path, content).unwrap();
    
    // Read using our File API
    let mut file = File::open(&file_path).unwrap();
    let read_content = file.read_to_string().unwrap();
    
    assert_eq!(read_content, content);
}

#[test]
fn test_file_read_write_bytes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_bytes.bin");
    let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello" in bytes
    
    // Write bytes
    let mut file = File::create(&file_path).unwrap();
    file.write_bytes(&data).unwrap();
    
    // Read bytes
    let mut file = File::open(&file_path).unwrap();
    let read_data = file.read_to_bytes().unwrap();
    
    assert_eq!(read_data, data);
}

#[test]
fn test_buffered_line_operations() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_lines.txt");
    
    // Write lines with buffering
    let mut file = File::create(&file_path).unwrap();
    file.enable_buffered_writing().unwrap();
    
    file.write_line("First line").unwrap();
    file.write_line("Second line").unwrap();
    file.write_line("Third line").unwrap();
    file.flush().unwrap();
    
    // Read lines with buffering
    let mut file = File::open(&file_path).unwrap();
    file.enable_buffered_reading().unwrap();
    
    assert_eq!(file.read_line().unwrap(), "First line");
    assert_eq!(file.read_line().unwrap(), "Second line");
    assert_eq!(file.read_line().unwrap(), "Third line");
    assert_eq!(file.read_line().unwrap(), ""); // EOF
}

#[test]
fn test_file_not_found_error() {
    let result = File::open("nonexistent_file.txt");
    assert!(result.is_err());
    
    if let FluxResult::Err(error) = result {
        let error_msg = error.to_string();
        assert!(error_msg.contains("File not found") || error_msg.contains("not found"));
    }
}

#[test]
fn test_utility_functions() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_utils.txt");
    let content = "Utility function test";
    
    // Test write_file
    write_file(&file_path, content).unwrap();
    
    // Test file_exists
    assert!(file_exists(&file_path));
    assert!(!file_exists("nonexistent.txt"));
    
    // Test read_file
    let read_content = read_file(&file_path).unwrap();
    assert_eq!(read_content, content);
    
    // Test append_file
    let append_content = "\nAppended line";
    append_file(&file_path, append_content).unwrap();
    
    let final_content = read_file(&file_path).unwrap();
    assert_eq!(final_content, format!("{}{}", content, append_content));
    
    // Test delete_file
    delete_file(&file_path).unwrap();
    assert!(!file_exists(&file_path));
}

#[test]
fn test_directory_operations() {
    let temp_dir = TempDir::new().unwrap();
    let sub_dir = temp_dir.path().join("subdir");
    
    // Test create_dir
    create_dir(&sub_dir).unwrap();
    assert!(sub_dir.exists());
    
    // Create some test files
    let file1 = sub_dir.join("file1.txt");
    let file2 = sub_dir.join("file2.txt");
    write_file(&file1, "content1").unwrap();
    write_file(&file2, "content2").unwrap();
    
    // Test list_dir
    let mut files = list_dir(&sub_dir).unwrap();
    files.sort();
    assert_eq!(files, vec!["file1.txt", "file2.txt"]);
}

#[test]
fn test_console_io_functions() {
    // Test print function (we can't easily test actual output, but we can test it doesn't panic)
    let result = print("Test message");
    assert!(result.is_ok());
    
    // Test println function
    let result = println("Test message with newline");
    assert!(result.is_ok());
}

#[test]
fn test_error_handling() {
    // Test various error conditions
    
    // Invalid path
    let result = File::open("/invalid/path/that/does/not/exist.txt");
    assert!(result.is_err());
    
    // Permission denied (try to write to root directory on Unix systems)
    #[cfg(unix)]
    {
        let _result = File::create("/root_file.txt");
        // This might succeed in some environments, so we don't assert failure
    }
    
    // Test error display
    let error = IoError::FileNotFound("test.txt".to_string());
    assert_eq!(error.to_string(), "File not found: test.txt");
    
    let error = IoError::PermissionDenied("/root".to_string());
    assert_eq!(error.to_string(), "Permission denied: /root");
}

#[test]
fn test_file_path_tracking() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("path_test.txt");
    
    let file = File::create(&file_path).unwrap();
    assert_eq!(file.path(), file_path.to_string_lossy());
}

#[test]
fn test_buffered_performance() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("perf_test.txt");
    
    // Write many lines to test buffering
    let mut file = File::create(&file_path).unwrap();
    file.enable_buffered_writing().unwrap();
    
    for i in 0..1000 {
        file.write_line(&format!("Line {}", i)).unwrap();
    }
    file.flush().unwrap();
    
    // Read back and verify
    let mut file = File::open(&file_path).unwrap();
    file.enable_buffered_reading().unwrap();
    
    for i in 0..1000 {
        let line = file.read_line().unwrap();
        assert_eq!(line, format!("Line {}", i));
    }
    
    // Should be EOF now
    assert_eq!(file.read_line().unwrap(), "");
}

#[test]
fn test_mixed_read_write_operations() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("mixed_ops.txt");
    
    // Create file with initial content
    write_file(&file_path, "Initial content\n").unwrap();
    
    // Open for read-write
    let mut file = File::open_rw(&file_path).unwrap();
    
    // Read initial content
    let content = file.read_to_string().unwrap();
    assert_eq!(content, "Initial content\n");
    
    // Note: In a real implementation, we'd need to handle file positioning
    // For now, we'll test that the file can be opened in read-write mode
    assert_eq!(file.path(), file_path.to_string_lossy());
}