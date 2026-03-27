#[test]
fn test_detector_works() {
    use scred_pattern_detector::Detector;
    
    let mut detector = Detector::new().expect("Detector creation failed");
    let input = b"AKIAIOSFODNN7EXAMPLE";
    
    let result = detector.process(input, true).expect("Process failed");
    
    println!("Events found: {}", result.events.len());
    for event in &result.events {
        println!("  Pattern: {}, Position: {}, Length: {}", 
            event.pattern_name(), event.position, event.length);
    }
    
    assert!(result.events.len() > 0, "Should detect AWS AKIA pattern");
}
