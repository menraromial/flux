//! Tests for the Flux standard library string module

use flux_compiler::std::string::*;
use flux_compiler::runtime::result::FluxResult;
use std::collections::HashMap;

#[test]
fn test_flux_string_creation() {
    let s1 = FluxString::new();
    assert!(s1.is_empty());
    assert_eq!(s1.len(), 0);
    assert_eq!(s1.char_len(), 0);
    
    let s2 = FluxString::from_str("Hello, World!");
    assert!(!s2.is_empty());
    assert_eq!(s2.len(), 13);
    assert_eq!(s2.char_len(), 13);
    assert_eq!(s2.as_str(), "Hello, World!");
    
    let s3 = FluxString::from_string("Test".to_string());
    assert_eq!(s3.as_str(), "Test");
}

#[test]
fn test_unicode_support() {
    let s = FluxString::from_str("Hello, 世界! 🌍");
    
    // Byte length vs character length
    assert_eq!(s.len(), 19); // Bytes (UTF-8 encoding)
    assert_eq!(s.char_len(), 12); // Unicode characters
    
    // Character access
    assert_eq!(s.char_at(0).unwrap(), 'H');
    assert_eq!(s.char_at(7).unwrap(), '世');
    assert_eq!(s.char_at(8).unwrap(), '界');
    assert_eq!(s.char_at(11).unwrap(), '🌍');
    
    // Out of bounds
    assert!(s.char_at(12).is_err());
}

#[test]
fn test_substring_operations() {
    let s = FluxString::from_str("Hello, World!");
    
    // Valid substring
    let sub = s.substring(0, 5).unwrap();
    assert_eq!(sub.as_str(), "Hello");
    
    let sub2 = s.substring(7, 12).unwrap();
    assert_eq!(sub2.as_str(), "World");
    
    // Full string
    let full = s.substring(0, s.char_len()).unwrap();
    assert_eq!(full.as_str(), "Hello, World!");
    
    // Invalid ranges
    assert!(s.substring(5, 3).is_err()); // start > end
    assert!(s.substring(0, 20).is_err()); // end > length
    assert!(s.substring(20, 25).is_err()); // start > length
}

#[test]
fn test_string_splitting() {
    let s = FluxString::from_str("apple,banana,cherry");
    
    let parts = s.split(",");
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].as_str(), "apple");
    assert_eq!(parts[1].as_str(), "banana");
    assert_eq!(parts[2].as_str(), "cherry");
    
    let s2 = FluxString::from_str("  hello   world  test  ");
    let words = s2.split_whitespace();
    assert_eq!(words.len(), 3);
    assert_eq!(words[0].as_str(), "hello");
    assert_eq!(words[1].as_str(), "world");
    assert_eq!(words[2].as_str(), "test");
    
    let s3 = FluxString::from_str("line1\nline2\nline3");
    let lines = s3.lines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].as_str(), "line1");
    assert_eq!(lines[1].as_str(), "line2");
    assert_eq!(lines[2].as_str(), "line3");
}

#[test]
fn test_string_joining() {
    let delimiter = FluxString::from_str(", ");
    let strings = vec![
        FluxString::from_str("apple"),
        FluxString::from_str("banana"),
        FluxString::from_str("cherry"),
    ];
    
    let joined = delimiter.join(&strings);
    assert_eq!(joined.as_str(), "apple, banana, cherry");
    
    // Empty vector
    let empty_joined = delimiter.join(&[]);
    assert_eq!(empty_joined.as_str(), "");
    
    // Single element
    let single = delimiter.join(&[FluxString::from_str("solo")]);
    assert_eq!(single.as_str(), "solo");
}

#[test]
fn test_string_trimming() {
    let s = FluxString::from_str("  hello world  ");
    
    assert_eq!(s.trim().as_str(), "hello world");
    assert_eq!(s.trim_start().as_str(), "hello world  ");
    assert_eq!(s.trim_end().as_str(), "  hello world");
    
    let s2 = FluxString::from_str("***hello***");
    assert_eq!(s2.trim_matches('*').as_str(), "hello");
}

#[test]
fn test_case_conversions() {
    let s = FluxString::from_str("Hello World");
    
    assert_eq!(s.to_lowercase().as_str(), "hello world");
    assert_eq!(s.to_uppercase().as_str(), "HELLO WORLD");
    assert_eq!(s.capitalize().as_str(), "Hello World");
    
    let s2 = FluxString::from_str("hello world test");
    assert_eq!(s2.to_title_case().as_str(), "Hello World Test");
    
    let s3 = FluxString::from_str("lowercase");
    assert_eq!(s3.capitalize().as_str(), "Lowercase");
}

#[test]
fn test_string_searching() {
    let s = FluxString::from_str("Hello, World! Hello again!");
    
    assert!(s.starts_with("Hello"));
    assert!(!s.starts_with("World"));
    
    assert!(s.ends_with("again!"));
    assert!(!s.ends_with("Hello"));
    
    assert!(s.contains("World"));
    assert!(s.contains("Hello"));
    assert!(!s.contains("xyz"));
    
    assert_eq!(s.find("Hello"), Some(0));
    assert_eq!(s.find("World"), Some(7));
    assert_eq!(s.find("xyz"), None);
    
    assert_eq!(s.rfind("Hello"), Some(14));
    assert_eq!(s.rfind("World"), Some(7));
}

#[test]
fn test_string_replacement() {
    let s = FluxString::from_str("Hello, World! Hello again!");
    
    let replaced = s.replace("Hello", "Hi");
    assert_eq!(replaced.as_str(), "Hi, World! Hi again!");
    
    let replaced_once = s.replacen("Hello", "Hi", 1);
    assert_eq!(replaced_once.as_str(), "Hi, World! Hello again!");
    
    let no_match = s.replace("xyz", "abc");
    assert_eq!(no_match.as_str(), "Hello, World! Hello again!");
}

#[test]
fn test_string_padding_and_alignment() {
    let s = FluxString::from_str("test");
    
    assert_eq!(s.pad_left(8).as_str(), "    test");
    assert_eq!(s.pad_right(8).as_str(), "test    ");
    assert_eq!(s.center(8).as_str(), "  test  ");
    
    // No padding needed
    assert_eq!(s.pad_left(3).as_str(), "test");
    assert_eq!(s.pad_right(4).as_str(), "test");
    assert_eq!(s.center(2).as_str(), "test");
}

#[test]
fn test_string_repeat_and_reverse() {
    let s = FluxString::from_str("abc");
    
    assert_eq!(s.repeat(3).as_str(), "abcabcabc");
    assert_eq!(s.repeat(0).as_str(), "");
    assert_eq!(s.repeat(1).as_str(), "abc");
    
    assert_eq!(s.reverse().as_str(), "cba");
    
    // Unicode reverse
    let unicode = FluxString::from_str("Hello, 世界!");
    assert_eq!(unicode.reverse().as_str(), "!界世 ,olleH");
}

#[test]
fn test_string_validation() {
    let numeric = FluxString::from_str("12345");
    assert!(numeric.is_numeric());
    
    let not_numeric = FluxString::from_str("123a5");
    assert!(!not_numeric.is_numeric());
    
    let alphabetic = FluxString::from_str("hello");
    assert!(alphabetic.is_alphabetic());
    
    let not_alphabetic = FluxString::from_str("hello123");
    assert!(!not_alphabetic.is_alphabetic());
    
    let alphanumeric = FluxString::from_str("hello123");
    assert!(alphanumeric.is_alphanumeric());
    
    let not_alphanumeric = FluxString::from_str("hello 123");
    assert!(!not_alphanumeric.is_alphanumeric());
    
    let ascii = FluxString::from_str("Hello World");
    assert!(ascii.is_ascii());
    
    let not_ascii = FluxString::from_str("Hello 世界");
    assert!(!not_ascii.is_ascii());
}

#[test]
fn test_string_mutation() {
    let mut s = FluxString::from_str("Hello");
    
    let world = FluxString::from_str(" World");
    s.append(&world);
    assert_eq!(s.as_str(), "Hello World");
    
    let greeting = FluxString::from_str("Hi, ");
    s.prepend(&greeting);
    assert_eq!(s.as_str(), "Hi, Hello World");
    
    s.insert_str(3, "there ").unwrap();
    assert_eq!(s.as_str(), "Hi,there  Hello World");
    
    // Test invalid insert
    assert!(s.insert_str(100, "test").is_err());
}

#[test]
fn test_regex_operations() {
    let regex = FluxRegex::new(r"\d+").unwrap();
    
    assert!(regex.is_match("There are 123 apples"));
    assert!(!regex.is_match("No numbers here"));
    
    let text = "I have 5 apples and 10 oranges";
    let matches = regex.find_all(text);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0], (7, 8)); // "5"
    assert_eq!(matches[1], (20, 22)); // "10"
    
    let first_match = regex.find(text);
    assert_eq!(first_match, Some((7, 8)));
    
    let replaced = regex.replace(text, "X");
    assert_eq!(replaced, "I have X apples and 10 oranges");
    
    let replaced_all = regex.replace_all(text, "X");
    assert_eq!(replaced_all, "I have X apples and X oranges");
    
    let split = regex.split("a1b2c3d");
    assert_eq!(split, vec!["a", "b", "c", "d"]);
}

#[test]
fn test_regex_captures() {
    let regex = FluxRegex::new(r"(\w+)\s+(\d+)").unwrap();
    let text = "apple 123";
    
    let captures = regex.captures(text).unwrap();
    assert_eq!(captures.len(), 3); // Full match + 2 groups
    assert_eq!(captures[0], "apple 123"); // Full match
    assert_eq!(captures[1], "apple"); // First group
    assert_eq!(captures[2], "123"); // Second group
}

#[test]
fn test_invalid_regex() {
    let result = FluxRegex::new(r"[invalid");
    assert!(result.is_err());
}

#[test]
fn test_string_formatting() {
    let args: Vec<&dyn std::fmt::Display> = vec![&"World", &42, &true];
    let formatted = format_string("Hello {0}! The answer is {1}. Is it true? {2}", &args).unwrap();
    assert_eq!(formatted, "Hello World! The answer is 42. Is it true? true");
    
    let mut values = HashMap::new();
    values.insert("name".to_string(), "Alice".to_string());
    values.insert("age".to_string(), "30".to_string());
    
    let interpolated = interpolate("Hello {name}, you are {age} years old!", &values);
    assert_eq!(interpolated, "Hello Alice, you are 30 years old!");
}

#[test]
fn test_utility_functions() {
    // Test join_strings
    let joined = join_strings(&["a", "b", "c"], ", ");
    assert_eq!(joined, "a, b, c");
    
    // Test split_string
    let split = split_string("a,b,c", ",");
    assert_eq!(split.len(), 3);
    assert_eq!(split[0].as_str(), "a");
    assert_eq!(split[1].as_str(), "b");
    assert_eq!(split[2].as_str(), "c");
    
    // Test email validation
    assert!(is_valid_email("test@example.com"));
    assert!(is_valid_email("user.name+tag@domain.co.uk"));
    assert!(!is_valid_email("invalid.email"));
    assert!(!is_valid_email("@domain.com"));
    assert!(!is_valid_email("user@"));
    
    // Test URL validation
    assert!(is_valid_url("https://example.com"));
    assert!(is_valid_url("http://test.org/path"));
    assert!(!is_valid_url("not-a-url"));
    assert!(!is_valid_url("ftp://example.com"));
}

#[test]
fn test_html_escaping() {
    let html = "<script>alert('xss')</script>";
    let escaped = escape_html(html);
    assert_eq!(escaped, "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
    
    let unescaped = unescape_html(&escaped);
    assert_eq!(unescaped, html);
    
    let complex = r#"<div class="test" data-value='123'>Content & more</div>"#;
    let escaped_complex = escape_html(complex);
    let unescaped_complex = unescape_html(&escaped_complex);
    assert_eq!(unescaped_complex, complex);
}

#[test]
fn test_case_conversions_utility() {
    // Test kebab-case
    assert_eq!(to_kebab_case("HelloWorld"), "hello-world");
    assert_eq!(to_kebab_case("hello_world"), "hello-world");
    assert_eq!(to_kebab_case("hello world"), "hello-world");
    assert_eq!(to_kebab_case("XMLHttpRequest"), "xmlhttp-request");
    
    // Test snake_case
    assert_eq!(to_snake_case("HelloWorld"), "hello_world");
    assert_eq!(to_snake_case("hello-world"), "hello_world");
    assert_eq!(to_snake_case("hello world"), "hello_world");
    assert_eq!(to_snake_case("XMLHttpRequest"), "xmlhttp_request");
    
    // Test camelCase
    assert_eq!(to_camel_case("hello_world"), "helloWorld");
    assert_eq!(to_camel_case("hello-world"), "helloWorld");
    assert_eq!(to_camel_case("hello world"), "helloWorld");
    assert_eq!(to_camel_case("XML_HTTP_REQUEST"), "xmlHttpRequest");
    
    // Test PascalCase
    assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
    assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
    assert_eq!(to_pascal_case("hello world"), "HelloWorld");
    assert_eq!(to_pascal_case("xml_http_request"), "XmlHttpRequest");
}

#[test]
fn test_levenshtein_distance() {
    assert_eq!(levenshtein_distance("", ""), 0);
    assert_eq!(levenshtein_distance("hello", "hello"), 0);
    assert_eq!(levenshtein_distance("", "hello"), 5);
    assert_eq!(levenshtein_distance("hello", ""), 5);
    assert_eq!(levenshtein_distance("hello", "hallo"), 1);
    assert_eq!(levenshtein_distance("hello", "world"), 4);
    assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
}

#[test]
fn test_error_handling() {
    // Test invalid index error
    let s = FluxString::from_str("test");
    match s.char_at(10) {
        FluxResult::Err(StringError::InvalidIndex { index, length }) => {
            assert_eq!(index, 10);
            assert_eq!(length, 4);
        },
        _ => panic!("Expected InvalidIndex error"),
    }
    
    // Test invalid range error
    match s.substring(2, 1) {
        FluxResult::Err(StringError::InvalidRange { start, end, length }) => {
            assert_eq!(start, 2);
            assert_eq!(end, 1);
            assert_eq!(length, 4);
        },
        _ => panic!("Expected InvalidRange error"),
    }
}

#[test]
fn test_conversions() {
    let s1: FluxString = "hello".into();
    assert_eq!(s1.as_str(), "hello");
    
    let s2: FluxString = "world".to_string().into();
    assert_eq!(s2.as_str(), "world");
    
    let s3 = FluxString::from_str("test");
    let std_string = s3.to_string();
    assert_eq!(std_string, "test");
}

#[test]
fn test_display_trait() {
    let s = FluxString::from_str("Hello, World!");
    assert_eq!(format!("{}", s), "Hello, World!");
}

#[test]
fn test_default_trait() {
    let s = FluxString::default();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
}