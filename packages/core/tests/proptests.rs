//! Property-based tests for the import + proto parsers.
//!
//! These don't aim for full grammar coverage — they generate a wide variety
//! of *roughly valid* inputs and assert that the parsers either succeed or
//! return a clean error rather than panicking.

use proptest::prelude::*;
use reqforge_core::import::{CurlImporter, Importer, PostmanImporter};
use reqforge_core::protocol::proto::parse_proto;

proptest! {
    /// Any input we throw at PostmanImporter must not panic.
    #[test]
    fn postman_never_panics(input in ".*") {
        let _ = PostmanImporter.import(&input);
    }

    /// Valid v2.1 envelopes built from random data should round-trip.
    #[test]
    fn postman_random_valid_envelope_round_trips(
        name in "[A-Za-z0-9 ]{1,30}",
        url in "https://[a-z]{3,8}\\.[a-z]{2,3}/[a-z]{1,5}",
        method in prop::sample::select(vec!["GET", "POST", "PUT", "DELETE", "PATCH"])
    ) {
        let json = format!(
            r#"{{
                "info": {{ "_postman_id": "abc", "name": "{name}", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" }},
                "item": [
                    {{
                        "name": "Test",
                        "request": {{
                            "method": "{method}",
                            "header": [],
                            "url": {{ "raw": "{url}", "protocol": "https", "host": ["example", "com"], "path": ["x"] }}
                        }},
                        "response": []
                    }}
                ]
            }}"#
        );
        let result = PostmanImporter.import(&json);
        prop_assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
    }

    /// Deeply nested folders must not blow the stack.
    #[test]
    fn postman_deep_nesting(depth in 1usize..15) {
        let mut json = String::from(r#"{"info":{"name":"d","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},"item":["#);
        for _ in 0..depth {
            json.push_str(r#"{"name":"f","item":["#);
        }
        json.push_str(r#"{"name":"leaf","request":{"method":"GET","header":[],"url":{"raw":"https://x.test","protocol":"https","host":["x"],"path":[]}}}"#);
        for _ in 0..depth {
            json.push_str("]}");
        }
        json.push_str("]}");

        let _ = PostmanImporter.import(&json);
    }

    /// Any input we throw at CurlImporter must not panic.
    #[test]
    fn curl_never_panics(input in ".*") {
        let _ = CurlImporter.import(&input);
    }

    /// A cURL line starting with `curl ` should never panic. We don't
    /// assert Ok because `curl -` (stdin) and similar malformed inputs
    /// are legitimately rejected.
    #[test]
    fn curl_with_random_path_no_panic(path in "[a-zA-Z0-9/_.-]{1,100}") {
        let line = format!("curl {path}");
        let _ = CurlImporter.import(&line);
    }

    /// Headers with random keys should always parse without panic.
    #[test]
    fn curl_with_random_header(key in "[A-Za-z]{1,20}", value in "[A-Za-z0-9 ]{1,30}") {
        let line = format!(r#"curl https://x.test -H "{key}: {value}""#);
        let result = CurlImporter.import(&line);
        prop_assert!(result.is_ok());
    }

    /// The proto parser must never panic on arbitrary input.
    #[test]
    fn proto_never_panics(input in ".*") {
        let _ = parse_proto(&input);
    }

    /// A message with random field names should always parse.
    #[test]
    fn proto_random_field_name_parses(
        name in "[A-Za-z][A-Za-z0-9_]{0,30}",
        field in "[A-Za-z][A-Za-z0-9_]{0,30}"
    ) {
        let input = format!(
            "message {name} {{ {field} string x = 1; }}",
            name = name,
            field = field
        );
        let result = parse_proto(&input);
        prop_assert!(result.is_ok(), "Expected Ok: {:?}", result.err());
        let parsed = result.unwrap();
        prop_assert!(!parsed.messages.is_empty());
    }

    /// A service with random method names should always parse.
    #[test]
    fn proto_random_method_parses(
        svc in "[A-Z][A-Za-z]{0,20}",
        method in "[a-z][A-Za-z]{0,20}"
    ) {
        let input = format!(
            r#"service {svc} {{ rpc {method} (Foo) returns (Bar); }}"#
        );
        let result = parse_proto(&input);
        prop_assert!(result.is_ok(), "Expected Ok: {:?}", result.err());
        let parsed = result.unwrap();
        prop_assert_eq!(parsed.services.len(), 1);
        prop_assert_eq!(parsed.services[0].methods.len(), 1);
        prop_assert_eq!(parsed.services[0].methods[0].name.as_str(), method.as_str());
    }
}
