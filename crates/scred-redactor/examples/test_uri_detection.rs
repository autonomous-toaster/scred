use scred_detector::detect_all;

fn main() {
    println!("=== URI Pattern Detection ===\n");

    let test_cases = vec![
        ("mongodb://user:MyPassword123@localhost:27017/db", "MongoDB"),
        ("redis://user:secretpass@redis.example.com:6379/0", "Redis"),
        (
            "https://hooks.slack.com/services/T123/B456/abcdef123456",
            "Slack Webhook",
        ),
        (
            "postgres://admin:secret@db.example.com:5432/mydb",
            "PostgreSQL",
        ),
    ];

    for (input, name) in test_cases {
        let input_bytes = input.as_bytes();
        let matches = detect_all(input_bytes);

        println!("{}", name);
        println!("Input:  {}", input);
        println!("Matches: {}", matches.count());

        for m in &matches.matches {
            let secret_text =
                std::str::from_utf8(&input_bytes[m.start..m.end]).unwrap_or("<invalid>");
            println!(
                "  [{}, {}] type={:3} '{}'",
                m.start, m.end, m.pattern_type, secret_text
            );
        }
        println!();
    }
}
