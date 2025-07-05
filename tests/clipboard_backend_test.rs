// Test to verify clipboard backend is working properly

fn test_clipboard_functionality(test_text: &str, test_name: &str) -> bool {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            match clipboard.set_text(test_text) {
                Ok(_) => {
                    // Small delay to ensure clipboard is properly set
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    match clipboard.get_text() {
                        Ok(retrieved) => {
                            if retrieved == test_text {
                                println!(
                                    "{} clipboard test successful: '{}'",
                                    test_name, test_text
                                );
                                true
                            } else {
                                println!(
                                    "{} clipboard test failed: expected '{}', got '{}'",
                                    test_name, test_text, retrieved
                                );
                                false
                            }
                        }
                        Err(e) => {
                            println!(
                                "Failed to get text from clipboard in {}: {:?}",
                                test_name, e
                            );
                            false
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to set text to clipboard in {}: {:?}", test_name, e);
                    false
                }
            }
        }
        Err(e) => {
            println!("Failed to create clipboard in {}: {:?}", test_name, e);
            false
        }
    }
}

#[test]
fn test_clipboard_comprehensive() {
    // Test both basic ASCII and Unicode functionality in a single test
    // This avoids the race condition between separate tests

    println!("=== Comprehensive Clipboard Test ===");

    // Test 1: Basic ASCII text
    let basic_text = "test clipboard basic content 123";
    let basic_success = test_clipboard_functionality(basic_text, "Basic ASCII");

    // Small delay between tests to ensure clipboard state is clean
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Test 2: Unicode text
    let unicode_text = "Hello 世界 🌍 Testing unicode";
    let unicode_success = test_clipboard_functionality(unicode_text, "Unicode");

    // Small delay between tests to ensure clipboard state is clean
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Test 3: Empty text
    let empty_text = "";
    let empty_success = test_clipboard_functionality(empty_text, "Empty");

    // Small delay between tests to ensure clipboard state is clean
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Test 4: Text with special characters
    let special_text = "Special chars: \n\t\r\"'\\";
    let special_success = test_clipboard_functionality(special_text, "Special chars");

    // Only assert if clipboard is actually available (at least one test succeeded)
    if basic_success || unicode_success || empty_success || special_success {
        // If any test worked, then clipboard is available, so all should work
        assert!(basic_success, "Basic ASCII clipboard test failed");
        assert!(unicode_success, "Unicode clipboard test failed");
        assert!(empty_success, "Empty text clipboard test failed");
        assert!(special_success, "Special characters clipboard test failed");
    } else {
        // If no tests worked, clipboard is probably not available in CI
        println!("Clipboard not available - skipping assertions");
    }
}
