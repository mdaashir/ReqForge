//! Micro-benchmarks for the request execution and JSON serialisation pipeline.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use reqforge_core::request::{Body, BodyMode, HttpMethod, KeyValue, Request};
use reqforge_core::testing::{Assertion, AssertionType, TestRunner};

fn bench_request_construction(c: &mut Criterion) {
    c.bench_function("request_construction", |b| {
        b.iter(|| {
            let req = Request::new(
                HttpMethod::Post,
                "https://jsonplaceholder.typicode.com/posts",
            );
            let body = Body {
                mode: BodyMode::Json,
                content_type: Some("application/json".into()),
                content: r#"{"title":"foo","body":"bar","userId":1}"#.into(),
            };
            black_box(Request {
                headers: vec![
                    KeyValue {
                        key: "Content-Type".into(),
                        value: "application/json".into(),
                        enabled: true,
                        description: None,
                    },
                    KeyValue {
                        key: "Accept".into(),
                        value: "application/json".into(),
                        enabled: true,
                        description: None,
                    },
                ],
                body,
                ..req
            })
        })
    });
}

fn bench_json_serialization(c: &mut Criterion) {
    let req = Request {
        id: "test".into(),
        name: "Test".into(),
        method: HttpMethod::Post,
        url: "https://api.example.com/data".into(),
        headers: vec![
            KeyValue {
                key: "Content-Type".into(),
                value: "application/json".into(),
                enabled: true,
                description: None,
            },
            KeyValue {
                key: "Authorization".into(),
                value: "Bearer eyJhbGciOiJIUzI1NiJ9.test.test".into(),
                enabled: true,
                description: None,
            },
        ],
        params: vec![KeyValue {
            key: "limit".into(),
            value: "100".into(),
            enabled: true,
            description: None,
        }],
        body: Body::default(),
        auth: None,
        settings: Default::default(),
        pre_request_script: None,
        post_response_script: None,
        test_script: None,
        description: None,
    };

    c.bench_function("json_serialize_request", |b| {
        b.iter(|| serde_json::to_string(black_box(&req)))
    });

    c.bench_function("json_deserialize_request", |b| {
        let json = serde_json::to_string(&req).unwrap();
        b.iter(|| serde_json::from_str::<Request>(black_box(&json)))
    });
}

fn bench_test_runner(c: &mut Criterion) {
    let response = reqforge_core::request::Response {
        status: 200,
        status_text: "OK".into(),
        headers: std::collections::HashMap::from([(
            "content-type".into(),
            "application/json".into(),
        )]),
        body: reqforge_core::request::ResponseBody {
            content: b"{\"id\":1,\"name\":\"test\"}".to_vec(),
            content_type: Some("application/json".into()),
            is_text: true,
        },
        cookies: Vec::new(),
        timing: Default::default(),
        size: Default::default(),
        url: "https://api.example.com/data".into(),
        protocol: "HTTP/1.1".into(),
    };

    c.bench_function("test_runner_5_assertions", |b| {
        b.iter(|| {
            let mut runner = TestRunner::new("bench");
            runner = runner
                .add(Assertion {
                    name: "status is 200".into(),
                    assertion: AssertionType::StatusCode { expected: 200 },
                })
                .add(Assertion {
                    name: "response time < 500ms".into(),
                    assertion: AssertionType::ResponseTime { max_ms: 500 },
                })
                .add(Assertion {
                    name: "body contains id".into(),
                    assertion: AssertionType::BodyContains {
                        substring: "id".into(),
                    },
                })
                .add(Assertion {
                    name: "header content-type".into(),
                    assertion: AssertionType::HeaderEquals {
                        header: "content-type".into(),
                        expected: "application/json".into(),
                    },
                })
                .add(Assertion {
                    name: "json path name".into(),
                    assertion: AssertionType::JsonPath {
                        path: "$.name".into(),
                        expected: "test".into(),
                    },
                });
            black_box(runner.run(black_box(&response)).unwrap())
        })
    });
}

criterion_group!(
    benches,
    bench_request_construction,
    bench_json_serialization,
    bench_test_runner,
);
criterion_main!(benches);
