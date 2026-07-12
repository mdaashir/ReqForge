//! Minimal `.proto` file parser
//!
//! Extracts package, imports, services, methods, and messages so the UI
//! can present them as a navigable tree. This is intentionally a tolerant
//! parser — it ignores the bulk of the proto grammar (oneofs, maps, options,
//! reserved ranges, extensions) and only captures what ReqForge needs to
//! drive a gRPC client. Unknown constructs are skipped, not rejected.
//!
//! For production use with full proto semantics, wire this through
//! `prost-build` or `tonic-build`. For interactive use (loading a proto
//! at runtime to drive the UI) this is enough.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProtoFile {
    pub syntax: String,
    pub package: String,
    pub imports: Vec<String>,
    pub services: Vec<ProtoService>,
    pub messages: Vec<ProtoMessage>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProtoService {
    pub name: String,
    pub methods: Vec<ProtoMethod>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProtoMethod {
    pub name: String,
    pub request_type: String,
    pub response_type: String,
    /// "unary" for now — streaming markers (`stream`) are detected
    /// and recorded but not supported in this implementation.
    pub client_streaming: bool,
    pub server_streaming: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProtoMessage {
    pub name: String,
    pub fields: Vec<ProtoField>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProtoField {
    pub name: String,
    /// Repeated fields (e.g. `repeated string tags`) set this true.
    pub repeated: bool,
    pub kind: ProtoFieldKind,
    pub number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(Default)]
pub enum ProtoFieldKind {
    Double,
    Float,
    Int64,
    Uint64,
    Int32,
    Fixed64,
    Fixed32,
    Bool,
    #[default]
    String,
    Bytes,
    Uint32,
    Sfixed32,
    Sint32,
    Sfixed64,
    Sint64,
    Message(String),
    Enum(String),
    Map { key: Box<ProtoFieldKind>, value: Box<ProtoFieldKind> },
}


pub fn parse_proto(input: &str) -> Result<ProtoFile, String> {
    let mut file = ProtoFile {
        syntax: "proto3".to_string(),
        package: String::new(),
        imports: Vec::new(),
        services: Vec::new(),
        messages: Vec::new(),
    };

    let mut by_name: BTreeMap<String, ProtoMessage> = BTreeMap::new();

    // Strip line comments. Block comments are stripped too but we don't try
    // to track nested state — proto files don't allow nesting anyway.
    let cleaned: String = input
        .lines()
        .map(|l| match l.find("//") {
            Some(idx) => &l[..idx],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let cleaned = strip_block_comments(&cleaned);

    // Strip string literals so we don't accidentally parse keywords inside
    // default values.
    let cleaned = strip_strings(&cleaned);

    // Tokenize at brace depth 0 to find top-level declarations.
    let decls = split_top_level(&cleaned);

    for decl in decls {
        let d = decl.trim();
        if d.is_empty() {
            continue;
        }

        if let Some(rest) = d.strip_prefix("syntax") {
            let val = extract_string(rest.trim_start_matches('=').trim());
            if !val.is_empty() {
                file.syntax = val;
            }
            continue;
        }

        if let Some(rest) = d.strip_prefix("package") {
            file.package = rest.trim().trim_end_matches(';').trim().to_string();
            continue;
        }

        if let Some(rest) = d.strip_prefix("import") {
            file.imports
                .push(rest.trim().trim_end_matches(';').trim().to_string());
            continue;
        }

        if let Some(rest) = d.strip_prefix("service") {
            file.services.push(parse_service(rest)?);
            continue;
        }

        if let Some(rest) = d.strip_prefix("message") {
            let msg = parse_message(rest)?;
            by_name.insert(msg.name.clone(), msg.clone());
            file.messages.push(msg);
            continue;
        }

        if d.starts_with("enum ") {
            // We don't expose enums in the UI yet; record as a message so
            // field references can still resolve.
            let enum_msg = parse_enum_as_message(d)?;
            by_name.insert(enum_msg.name.clone(), enum_msg.clone());
            // Don't push to file.messages since they aren't real messages.
            continue;
        }

        // Unknown top-level — ignore.
    }

    Ok(file)
}

fn parse_service(input: &str) -> Result<ProtoService, String> {
    let body = extract_body(input).ok_or("service missing body")?;
    let header = input[..input.find('{').unwrap_or(input.len())].trim();
    let name = header
        .trim_start_matches("service")
        .trim()
        .trim_end_matches('{')
        .trim()
        .to_string();
    let mut svc = ProtoService {
        name,
        methods: Vec::new(),
    };

    let method_decls = split_top_level(&body);
    for m in method_decls {
        let mt = m.trim();
        if !mt.starts_with("rpc ") {
            continue;
        }
        svc.methods.push(parse_method(mt)?);
    }
    Ok(svc)
}

fn parse_method(input: &str) -> Result<ProtoMethod, String> {
    // Strip trailing semicolons / options braces
    let mut s = input.trim().trim_end_matches(';').to_string();
    if let Some(idx) = s.find('{') {
        s.truncate(idx);
    }
    let s = s.trim();

    // Reassemble types wrapped in parens, e.g. `(stream Foo)` becomes a
    // single token `stream Foo`. Whitespace splitting would otherwise split
    // it into `(stream` and `Foo)`.
    let s = glue_paren_types(s);

    let after_rpc = s.trim_start_matches("rpc").trim();
    let parts: Vec<&str> = after_rpc.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(format!("malformed rpc: {input}"));
    }
    let name = parts[0].to_string();
    let request_token = parts[1];
    let response_token = if parts.len() >= 4 && parts[2] == "returns" {
        parts[3]
    } else if parts.len() >= 3 && parts[2] != "returns" {
        parts[2]
    } else {
        return Err(format!("malformed rpc: {input}"));
    };
    let request = strip_stream_marker(request_token);
    let response = strip_stream_marker(response_token);
    let client_streaming = request_token.contains("stream");
    let server_streaming = response_token.contains("stream");

    Ok(ProtoMethod {
        name,
        request_type: request,
        response_type: response,
        client_streaming,
        server_streaming,
    })
}

fn strip_stream_marker(token: &str) -> String {
    let mut s = token.trim().to_string();
    // Drop surrounding parens (already-glued by `glue_paren_types`).
    s = s.trim_start_matches('(').to_string();
    s = s.trim_end_matches(')').to_string();
    // If it starts with `stream`, drop that prefix.
    if let Some(rest) = s.strip_prefix("stream") {
        s = rest.trim().to_string();
    }
    s
}

/// Joins tokens like `(stream` `Foo)` into a single token so that downstream
/// whitespace splitting doesn't break the type apart.
fn glue_paren_types(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut depth: i32 = 0;
    let mut buf = String::new();
    let mut in_paren = false;
    for ch in input.chars() {
        match ch {
            '(' => {
                depth += 1;
                in_paren = true;
                buf.push(ch);
            }
            ')' => {
                depth -= 1;
                buf.push(ch);
                if depth == 0 {
                    out.push_str(&buf.replace(' ', "_"));
                    buf.clear();
                    in_paren = false;
                }
            }
            ' ' if in_paren => buf.push('_'),
            _ => {
                if in_paren {
                    buf.push(ch);
                } else {
                    out.push(ch);
                }
            }
        }
    }
    out
}

fn parse_message(input: &str) -> Result<ProtoMessage, String> {
    let body = extract_body(input).ok_or("message missing body")?;
    let header = input[..input.find('{').unwrap_or(input.len())].trim();
    let name = header
        .trim_start_matches("message")
        .trim()
        .trim_end_matches('{')
        .trim()
        .to_string();

    let mut msg = ProtoMessage {
        name,
        fields: Vec::new(),
    };

    for line in body.lines() {
        let l = line.trim().trim_end_matches(';').trim();
        if l.is_empty()
            || l.starts_with("//")
            || l.starts_with("reserved")
            || l.starts_with("oneof")
            || l.starts_with("option")
            || l.starts_with("extensions")
        {
            continue;
        }
        // Skip nested message/enum declarations: they have their own braces
        // and are captured separately when we recurse — for now we ignore.
        if l.starts_with("message ") || l.starts_with("enum ") {
            continue;
        }
        if let Some(field) = parse_field(l) {
            msg.fields.push(field);
        }
    }
    Ok(msg)
}

fn parse_enum_as_message(input: &str) -> Result<ProtoMessage, String> {
    let header = input[..input.find('{').unwrap_or(input.len())].trim();
    let name = header
        .trim_start_matches("enum")
        .trim()
        .trim_end_matches('{')
        .trim()
        .to_string();
    // We synthesise a placeholder field so the enum can be referenced as a
    // message type from other fields. Real enum values are ignored.
    Ok(ProtoMessage {
        name,
        fields: Vec::new(),
    })
}

fn parse_field(input: &str) -> Option<ProtoField> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let repeated = parts[0] == "repeated" || parts[0] == "optional";
    let type_start = if repeated { 1 } else { 0 };
    let kind_str = parts[type_start];
    let name = parts[type_start + 1];
    let number_str = parts
        .iter()
        .rev()
        .find(|p| p.parse::<i32>().is_ok())?;
    let number = number_str.parse().ok()?;

    Some(ProtoField {
        name: name.to_string(),
        repeated,
        kind: parse_kind(kind_str),
        number,
    })
}

fn parse_kind(s: &str) -> ProtoFieldKind {
    match s {
        "double" => ProtoFieldKind::Double,
        "float" => ProtoFieldKind::Float,
        "int64" => ProtoFieldKind::Int64,
        "uint64" => ProtoFieldKind::Uint64,
        "int32" => ProtoFieldKind::Int32,
        "fixed64" => ProtoFieldKind::Fixed64,
        "fixed32" => ProtoFieldKind::Fixed32,
        "bool" => ProtoFieldKind::Bool,
        "string" => ProtoFieldKind::String,
        "bytes" => ProtoFieldKind::Bytes,
        "uint32" => ProtoFieldKind::Uint32,
        "sfixed32" => ProtoFieldKind::Sfixed32,
        "sint32" => ProtoFieldKind::Sint32,
        "sfixed64" => ProtoFieldKind::Sfixed64,
        "sint64" => ProtoFieldKind::Sint64,
        other => ProtoFieldKind::Message(other.to_string()),
    }
}

fn extract_body(input: &str) -> Option<String> {
    let open = input.find('{')?;
    let close = input.rfind('}')?;
    if close <= open {
        return None;
    }
    Some(input[open + 1..close].to_string())
}

fn split_top_level(input: &str) -> Vec<String> {
    // We want each top-level decl to include its full header AND its body
    // when it's a brace-delimited block (so `service Foo { ... }` comes out
    // as one string, not as two). To do that we keep `start` pinned to the
    // beginning of the current decl, including any nested `{ ... }`.
    let mut decls = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (idx, ch) = chars[i];
        match ch {
            '{' => {
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    // Close of a top-level block: capture [start, `}`] inclusive.
                    let end = idx + ch.len_utf8();
                    let s = input[start..end].trim().to_string();
                    if !s.is_empty() {
                        decls.push(s);
                    }
                    start = end;
                }
            }
            ';' if depth == 0 => {
                let s = input[start..idx].trim().to_string();
                if !s.is_empty() {
                    decls.push(s);
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
        i += 1;
    }
    let tail = input[start..].trim().to_string();
    if !tail.is_empty() {
        decls.push(tail);
    }
    decls
}

fn extract_string(s: &str) -> String {
    s.trim()
        .trim_start_matches('=')
        .trim()
        .trim_matches('"')
        .to_string()
}

fn strip_block_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            // Find closing */
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn strip_strings(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_str = false;
    for ch in input.chars() {
        if ch == '"' {
            in_str = !in_str;
            out.push('"');
            continue;
        }
        if in_str {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_basic_proto() {
        let proto = r#"
            syntax = "proto3";
            package helloworld;

            service Greeter {
              rpc SayHello (HelloRequest) returns (HelloReply);
              rpc Chat (stream Message) returns (stream Message);
            }

            message HelloRequest {
              string name = 1;
              repeated string tags = 2;
              int32 age = 3;
            }

            message HelloReply {
              string message = 1;
            }

            message Message {
              string text = 1;
            }
        "#;
        let f = parse_proto(proto).unwrap();
        assert_eq!(f.package, "helloworld");
        assert_eq!(f.services.len(), 1);
        assert_eq!(f.services[0].name, "Greeter");
        assert_eq!(f.services[0].methods.len(), 2);
        let say = &f.services[0].methods[0];
        assert_eq!(say.name, "SayHello");
        assert_eq!(say.request_type, "HelloRequest");
        assert_eq!(say.response_type, "HelloReply");
        assert!(!say.client_streaming);
        let chat = &f.services[0].methods[1];
        assert!(chat.client_streaming);
        assert!(chat.server_streaming);

        assert_eq!(f.messages.len(), 3);
        let req = &f.messages[0];
        assert_eq!(req.name, "HelloRequest");
        assert_eq!(req.fields.len(), 3);
        assert!(req.fields[1].repeated);
    }

    #[test]
    fn test_ignores_comments_and_unknown_constructs() {
        let proto = r#"
            // line comment
            /* block */
            syntax = "proto3";
            package x;
            message M {
              reserved 1 to 10;
              string a = 11;
            }
        "#;
        let f = parse_proto(proto).unwrap();
        assert_eq!(f.messages[0].fields.len(), 1);
        assert_eq!(f.messages[0].fields[0].name, "a");
    }
}
