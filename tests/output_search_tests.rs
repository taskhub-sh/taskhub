use crossterm::event::{KeyCode, KeyModifiers};
use taskhub::db::init_db;
use taskhub::tui::app::{App, SearchMode};
use taskhub::tui::views::terminal::CommandEntry;

async fn create_test_app() -> App {
    let pool = init_db(Some(":memory:".into())).await.unwrap();
    App::new(pool)
}

#[tokio::test]
async fn test_output_search_case_sensitivity_toggle() {
    let mut app = create_test_app().await;

    // Add some test commands to search through
    let entry1 = CommandEntry {
        command: "echo Hello".to_string(),
        output: "Hello World\nGoodbye world".to_string(),
        success: true,
    };
    let entry2 = CommandEntry {
        command: "echo WORLD".to_string(),
        output: "WORLD of testing".to_string(),
        success: true,
    };

    app.command_history.push(entry1);
    app.command_history.push(entry2);

    // Start output search
    app.on_key_code(KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert!(app.output_search_active);
    assert_eq!(app.output_search_mode, SearchMode::CaseInsensitive); // Default is case-insensitive

    // Type "world" to search
    app.on_key('w');
    app.on_key('o');
    app.on_key('r');
    app.on_key('l');
    app.on_key('d');

    // Should find 4 matches: "World", "world", "WORLD", and in command "echo WORLD"
    let initial_matches = app.output_search_matches.len();
    assert!(
        initial_matches > 1,
        "Should find multiple matches in case-insensitive mode, found: {}",
        initial_matches
    );

    // Toggle to case-sensitive mode with Tab
    app.on_key_code(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(app.output_search_mode, SearchMode::CaseSensitive);

    // Now should only find exact case matches
    let case_sensitive_matches = app.output_search_matches.len();
    assert!(
        case_sensitive_matches < initial_matches,
        "Case-sensitive should find fewer matches"
    );

    // Toggle to regex mode with Tab
    app.on_key_code(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(app.output_search_mode, SearchMode::Regex);

    // Toggle back to case-insensitive
    app.on_key_code(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(app.output_search_mode, SearchMode::CaseInsensitive);

    // Should find the initial matches again
    assert_eq!(app.output_search_matches.len(), initial_matches);
}

#[tokio::test]
async fn test_output_search_status_display() {
    let mut app = create_test_app().await;

    // Test initial state
    assert!(app.get_output_search_status().is_empty());

    // Start output search
    app.on_key_code(KeyCode::Char('f'), KeyModifiers::CONTROL);

    // Check initial status shows case-insensitive mode
    let status = app.get_output_search_status();
    assert!(status.contains("[aa]")); // Case-insensitive indicator
    assert!(status.contains("Tab to toggle mode"));

    // Toggle to case-sensitive
    app.on_key_code(KeyCode::Tab, KeyModifiers::NONE);

    // Check status shows case-sensitive mode
    let status = app.get_output_search_status();
    assert!(status.contains("[Aa]")); // Case-sensitive indicator

    // Type a search query
    app.on_key('t');
    app.on_key('e');
    app.on_key('s');
    app.on_key('t');

    // Check status includes the query
    let status = app.get_output_search_status();
    assert!(status.contains("'test'"));
    assert!(status.contains("[Aa]")); // Still case-sensitive
}

#[tokio::test]
async fn test_output_search_regex_mode() {
    let mut app = create_test_app().await;

    // Add test data with patterns that work well with regex
    let entry1 = CommandEntry {
        command: "echo test123".to_string(),
        output: "test123\ntest456\nNumber: 789\nEmail: user@example.com".to_string(),
        success: true,
    };
    let entry2 = CommandEntry {
        command: "cat file.txt".to_string(),
        output: "line1: hello\nline2: world123\nline3: abc123def".to_string(),
        success: true,
    };

    app.command_history.push(entry1);
    app.command_history.push(entry2);

    // Start output search
    app.on_key_code(KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert_eq!(app.output_search_mode, SearchMode::CaseInsensitive);

    // Toggle to regex mode
    app.on_key_code(KeyCode::Tab, KeyModifiers::NONE); // -> CaseSensitive
    app.on_key_code(KeyCode::Tab, KeyModifiers::NONE); // -> Regex
    assert_eq!(app.output_search_mode, SearchMode::Regex);

    // Test regex pattern: match numbers
    app.on_key('\\');
    app.on_key('d');
    app.on_key('+');

    // Should find: 123, 456, 789, 123, 123
    let regex_matches = app.output_search_matches.len();
    assert!(
        regex_matches >= 3,
        "Should find multiple number patterns with regex, found: {}",
        regex_matches
    );

    // Clear and test email pattern
    app.output_search_query.clear();
    app.on_key('[');
    app.on_key('a');
    app.on_key('-');
    app.on_key('z');
    app.on_key('A');
    app.on_key('-');
    app.on_key('Z');
    app.on_key('0');
    app.on_key('-');
    app.on_key('9');
    app.on_key(']');
    app.on_key('+');
    app.on_key('@');
    app.on_key('[');
    app.on_key('a');
    app.on_key('-');
    app.on_key('z');
    app.on_key('A');
    app.on_key('-');
    app.on_key('Z');
    app.on_key(']');
    app.on_key('+');
    app.on_key('\\');
    app.on_key('.');
    app.on_key('[');
    app.on_key('a');
    app.on_key('-');
    app.on_key('z');
    app.on_key('A');
    app.on_key('-');
    app.on_key('Z');
    app.on_key(']');
    app.on_key('+');

    // Should find the email address
    let email_matches = app.output_search_matches.len();
    assert!(email_matches >= 1, "Should find email pattern with regex");

    // Check status shows regex mode
    let status = app.get_output_search_status();
    assert!(status.contains("[.*]")); // Regex indicator
}
