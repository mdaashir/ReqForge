//! Header Juggler — ReqForge sample plugin.
//!
//! Implements the ReqForge plugin ABI (alloc + handle).
//! Build: `cargo build --target wasm32-unknown-unknown --release`
//! Output: `target/wasm32-unknown-unknown/release/header_juggler.wasm`

use std::slice;

static mut BUFFER: [u8; 65536] = [0u8; 65536];

#[no_mangle]
pub extern "C" fn alloc(_size: i32) -> i32 {
    unsafe { BUFFER.as_mut_ptr() as i32 }
}

#[no_mangle]
pub extern "C" fn handle(input_len: i32, input_ptr: i32) -> i32 {
    let input = unsafe {
        let input_slice = slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        std::str::from_utf8_unchecked(input_slice)
    };

    let response = process_message(input);

    let bytes = response.as_bytes();
    let len = bytes.len().min(65500);
    unsafe {
        std::ptr::write_unaligned(BUFFER.as_mut_ptr() as *mut u32, len as u32);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), BUFFER.as_mut_ptr().add(4), len);
    }
    0
}

fn process_message(input: &str) -> String {
    let kind = extract_json_string(input, "kind").unwrap_or_default();
    match kind.as_str() {
        "init" | "Init" => r#"{"kind":"ok"}"#.to_string(),
        "on_request" | "OnRequest" => {
            // Add an X-Header-Juggler header to every outgoing request.
            r#"{"kind":"replace","headers":{"X-Header-Juggler":"processed"}}"#.to_string()
        }
        "on_response" | "OnResponse" => r#"{"kind":"ok"}"#.to_string(),
        _ => r#"{"kind":"ok"}"#.to_string(),
    }
}

fn extract_json_string(input: &str, key: &str) -> Option<String> {
    // Simple JSON string value extraction — not a full parser.
    // Works for our known message shapes. For a real plugin use serde_json.
    let search = &format!("\"{}\"", key);
    if let Some(start) = input.find(search) {
        let after_key = &input[start + search.len()..];
        let colon = after_key.find(':')?;
        let after_colon = after_key[colon + 1..].trim();
        let trimmed = after_colon
            .trim_start_matches('"')
            .split('"')
            .next()?;
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kind() {
        let msg = r#"{"kind":"on_request","request_id":"abc","method":"GET"}"#;
        let kind = extract_json_string(msg, "kind");
        assert_eq!(kind.as_deref(), Some("on_request"));
    }

    #[test]
    fn test_process_on_request() {
        let msg = r#"{"kind":"on_request","request_id":"abc","method":"GET","url":"https://x.test"}"#;
        let response = process_message(&msg);
        assert!(response.contains("X-Header-Juggler"));
        assert!(response.contains("processed"));
    }

    #[test]
    fn test_process_unknown_kind() {
        let msg = r#"{"kind":"bogus"}"#;
        let response = process_message(&msg);
        assert_eq!(response, r#"{"kind":"ok"}"#);
    }
}
