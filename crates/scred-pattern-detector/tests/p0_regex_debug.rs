use regex::Regex;

#[test]
fn debug_sha256() {
    let input = "$5$rounds=8000$abc$AAAA.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8JJKKLL";
    let re = Regex::new(r"\$5\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{43}").unwrap();
    println!("\nSHA-256 input: {}", input);
    println!("Match result: {}", re.is_match(input));
    if let Some(m) = re.find(input) {
        println!("Matched: {}", &input[m.start()..m.end()]);
    } else {
        println!("NO MATCH!");
        // Debug the parts
        let parts: Vec<&str> = input.split('$').collect();
        println!("Parts: {:?}", parts);
        if parts.len() > 3 {
            println!("Hash part: {}, len={}", parts[4], parts[4].len());
        }
    }
    assert!(re.is_match(input));
}

#[test]
fn debug_header() {
    let input = "X-Auth-Token: sk_live_abcdef123456789012345678901234";
    let re = Regex::new(r"[Xx]-(?:[Aa]uth|[Aa]ccess|[Aa][Pp][Ii]-)?[Tt]oken\s*:\s*[a-zA-Z0-9_./+\-]{20,}").unwrap();
    println!("\nHeader input: {}", input);
    println!("Match result: {}", re.is_match(input));
    if let Some(m) = re.find(input) {
        println!("Matched: {}", &input[m.start()..m.end()]);
    } else {
        println!("NO MATCH!");
    }
    assert!(re.is_match(input));
}

#[test]
fn debug_jdbc() {
    let input = "jdbc:oracle:thin:scott:tiger@//oracle.example.com:1521/orcl";
    let re = Regex::new(r"[a-zA-Z][\w+:\.\-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.\-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    println!("\nJDBC input: {}", input);
    println!("Match result: {}", re.is_match(input));
    if let Some(m) = re.find(input) {
        println!("Matched: {}", &input[m.start()..m.end()]);
    } else {
        println!("NO MATCH!");
    }
    assert!(re.is_match(input));
}
