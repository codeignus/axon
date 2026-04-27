#[axon_export]
fn describe_parse(source: &str) -> String {
    let mut stack: Vec<char> = Vec::new();
    for ch in source.chars() {
        match ch {
            '(' | '[' | '{' => stack.push(ch),
            ')' => {
                if stack.pop() != Some('(') {
                    return format!("error: parser: mismatched ')'");
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return format!("error: parser: mismatched ']'");
                }
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return format!("error: parser: mismatched '}}'");
                }
            }
            _ => {}
        }
    }
    if stack.is_empty() {
        "ok".to_string()
    } else {
        "error: parser: unclosed delimiter".to_string()
    }
}
