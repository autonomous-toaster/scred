use scred_pattern_detector::Detector;

fn main() {
    let mut detector = Detector::new().unwrap();
    
    let test_cases = [
        "2024-03-20 10:15:23 [INFO] Server started on port 8080",
        "Authorization: Bearer token123",
        "[DEBUG] Timestamp: 2024-03-20T10:15:23Z",
        "user email: test@example.com",
        "port number: 3306",
    ];
    
    for test in &test_cases {
        let result = detector.process(test.as_bytes(), true).unwrap();
        println!("\nTest: {}", test);
        println!("Matches: {}", result.events.len());
        for event in &result.events {
            println!("  - '{}' at pos {}", event.pattern_name(), event.position);
        }
    }
}
