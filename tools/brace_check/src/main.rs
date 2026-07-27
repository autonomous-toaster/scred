use std::fs;
use std::path::Path;

/// Parse a Rust file and find brace mismatches by tracking depth
/// with awareness of string literals, char literals, comments, lifetime labels
fn main() {
    let path = std::env::args().nth(1).expect("Usage: brace_check <file.rs>");
    let content = fs::read_to_string(&path).expect("Failed to read file");
    
    let lines: Vec<&str> = content.lines().collect();
    let mut depth = 0i32;
    let mut depth_by_line: Vec<(usize, i32)> = Vec::new();
    
    for (line_num, line) in lines.iter().enumerate() {
        let line_num_1 = line_num + 1;
        let mut i = 0;
        let chars: Vec<char> = line.chars().collect();
        
        while i < chars.len() {
            let ch = chars[i];
            
            // Skip string literals
            if ch == '"' {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' { i += 1; }
                    i += 1;
                }
                i += 1;
                continue;
            }
            
            // Skip char literals and lifetime labels
            if ch == '\'' {
                i += 1;
                if i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    // Lifetime label: 'a, 'static, etc.
                    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    continue;
                } else {
                    // Char literal: 'x', '\n', etc.
                    while i < chars.len() && chars[i] != '\'' {
                        if chars[i] == '\\' { i += 1; }
                        i += 1;
                    }
                    i += 1;
                    continue;
                }
            }
            
            // Skip line comments
            if ch == '/' && i + 1 < chars.len() && chars[i+1] == '/' {
                break;
            }
            
            // Skip block comments
            if ch == '/' && i + 1 < chars.len() && chars[i+1] == '*' {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i+1] == '/') {
                    i += 1;
                }
                i += 2;
                continue;
            }
            
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth < 0 {
                    println!("NEGATIVE DEPTH at line {}: depth={}", line_num_1, depth);
                    // Show context
                    let start = if i > 50 { i - 50 } else { 0 };
                    let end = std::cmp::min(i + 50, chars.len());
                    let ctx: String = chars[start..end].iter().collect();
                    println!("  Context: ...{}...", ctx);
                }
            }
            
            i += 1;
        }
        
        depth_by_line.push((line_num_1, depth));
    }
    
    println!("Final depth: {}", depth);
    
    // Find the function handle_h2_client_transcoding and check its depth
    let mut in_func = false;
    let mut func_start_depth = 0;
    let mut func_start_line = 0;
    
    for (line_num, line) in lines.iter().enumerate() {
        let line_num_1 = line_num + 1;
        if line.contains("async fn handle_h2_client_transcoding") {
            in_func = true;
            func_start_line = line_num_1;
            // Find the opening brace
            for (ln, d) in &depth_by_line {
                if *ln == line_num_1 {
                    func_start_depth = *d;
                    break;
                }
            }
            println!("Function starts at line {} with depth {}", func_start_line, func_start_depth);
        }
        
        if in_func && line.trim() == "}" {
            let d = depth_by_line[line_num].1;
            println!("  Line {}: depth={} }}", line_num_1, d);
            if d == func_start_depth - 1 {
                println!("  -> Function body closed at line {}", line_num_1);
                break;
            }
        }
    }
}
