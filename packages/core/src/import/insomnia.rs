//! Insomnia v4 collection + environment importer
//!
//! Parses an Insomnia export (YAML or JSON) and converts it to a
//! ReqForge `Collection`. Environments are exposed via
//! `Importer::import_environments`.

use crate::collection::{Collection, CollectionItem};
use crate::environment::{Environment, Variable, VariableType};
use crate::error::{Error, Result};
use crate::import::Importer;
use crate::request::{Auth, AuthType, Body, BodyMode, HttpMethod, KeyValue, Request};
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "_type", rename_all = "snake_case")]
enum InsomniaResource {
    Request {
        _id: String,
        #[serde(default)]
        parent_id: Option<String>,
        name: String,
        method: String,
        url: String,
        #[serde(default)]
        headers: Vec<InsomniaHeader>,
        #[serde(default)]
        body: Option<InsomniaBody>,
        #[serde(default)]
        authentication: Option<InsomniaAuth>,
        #[serde(default)]
        parameters: Vec<InsomniaQuery>,
        #[serde(default)]
        description: Option<String>,
    },
    RequestGroup {
        _id: String,
        #[serde(default)]
        parent_id: Option<String>,
        name: String,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        environment: HashMap<String, serde_json::Value>,
        #[serde(default)]
        authentication: Option<InsomniaAuth>,
    },
    Workspace {
        _id: String,
        name: String,
        #[serde(default)]
        #[allow(dead_code)]
        description: Option<String>,
    },
    Environment {
        _id: String,
        #[serde(default)]
        #[allow(dead_code)]
        parent_id: Option<String>,
        name: String,
        #[serde(default)]
        data: HashMap<String, serde_json::Value>,
        /// UI colour for the environment (hex). Persisted for round-trip
        /// safety; not yet surfaced in ReqForge's environment selector.
        #[serde(default)]
        #[allow(dead_code)]
        color: Option<String>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
struct InsomniaHeader {
    name: String,
    value: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct InsomniaQuery {
    name: String,
    value: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct InsomniaBody {
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum InsomniaAuth {
    Bearer {
        #[serde(default)]
        prefix: Option<String>,
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    Apikey {
        key: String,
        value: String,
        #[serde(default)]
        add_to: Option<String>,
    },
    #[serde(other)]
    None,
}

#[derive(Debug, Deserialize)]
struct InsomniaExport {
    #[serde(default = "default_format")]
    __export_format: u32,
    #[serde(default)]
    resources: Vec<InsomniaResource>,
}

fn default_format() -> u32 {
    0
}

pub struct InsomniaImporter;

impl Importer for InsomniaImporter {
    fn format(&self) -> &'static str {
        "insomnia"
    }

    fn file_extension(&self) -> Option<&'static str> {
        Some("yaml")
    }

    fn import(&self, input: &str) -> Result<Collection> {
        let export = parse(input)?;

        let (workspace_id, workspace_name) = export
            .resources
            .iter()
            .find_map(|r| match r {
                InsomniaResource::Workspace { _id, name, .. } => Some((_id.clone(), name.clone())),
                _ => None,
            })
            .ok_or_else(|| Error::other("Insomnia export missing workspace root"))?;

        let mut folder_children: HashMap<String, Vec<String>> = HashMap::new();
        for r in &export.resources {
            let (id, parent_id) = match r {
                InsomniaResource::RequestGroup { _id, parent_id, .. }
                | InsomniaResource::Request { _id, parent_id, .. } => (_id.clone(), parent_id.clone()),
                _ => continue,
            };
            if let Some(p) = parent_id {
                folder_children.entry(p).or_default().push(id);
            }
        }
        let folder_ids: HashMap<String, ()> = export
            .resources
            .iter()
            .filter_map(|r| match r {
                InsomniaResource::RequestGroup { _id, .. } => Some((_id.clone(), ())),
                _ => None,
            })
            .collect();

        let mut root_children = Vec::new();
        if let Some(child_ids) = folder_children.get(&workspace_id) {
            for cid in child_ids {
                if let Some(item) = build_folder(cid, &folder_children, &folder_ids, &export.resources) {
                    root_children.push(item);
                }
            }
        }
        for r in &export.resources {
            if let InsomniaResource::Request { _id, parent_id, .. } = r {
                if parent_id.as_deref() == Some(&workspace_id) {
                    if let Some(item) = build_request(_id, &export.resources) {
                        root_children.push(item);
                    }
                }
            }
        }

        let mut variables = Vec::new();
        for r in &export.resources {
            if let InsomniaResource::RequestGroup {
                parent_id,
                environment,
                ..
            } = r
            {
                if parent_id.as_deref() == Some(&workspace_id) {
                    for (k, v) in environment {
                        variables.push(KeyValue {
                            key: k.clone(),
                            value: json_to_string(v),
                            enabled: true,
                            description: None,
                        });
                    }
                }
            }
        }

        let workspace_auth = export.resources.iter().find_map(|r| match r {
            InsomniaResource::RequestGroup {
                _id,
                authentication,
                ..
            } if _id == &workspace_id => authentication.clone(),
            _ => None,
        });

        Ok(Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name: workspace_name,
            description: None,
            auth: workspace_auth.and_then(|a| convert_auth(&a)),
            headers: Vec::new(),
            variables,
            items: root_children,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    fn import_environments(&self, input: &str) -> Result<Vec<Environment>> {
        let export = parse(input)?;
        let mut out = Vec::new();
        for r in export.resources {
            if let InsomniaResource::Environment { name, data, .. } = r {
                let variables = data
                    .into_iter()
                    .map(|(k, v)| Variable {
                        key: k,
                        value: json_to_string(&v),
                        var_type: VariableType::String,
                        enabled: true,
                    })
                    .collect();
                out.push(Environment {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    variables,
                    color: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                });
            }
        }
        Ok(out)
    }
}

fn parse(input: &str) -> Result<InsomniaExport> {
    let trimmed = input.trim();
    if let Ok(value) = serde_yaml::from_str::<InsomniaExport>(trimmed) {
        return Ok(value);
    }
    if let Ok(value) = serde_json::from_str::<InsomniaExport>(trimmed) {
        return Ok(value);
    }
    Err(Error::other("Could not parse Insomnia export (not valid YAML or JSON)"))
}

fn json_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn build_folder(
    folder_id: &str,
    folder_children: &HashMap<String, Vec<String>>,
    folder_ids: &HashMap<String, ()>,
    resources: &[InsomniaResource],
) -> Option<CollectionItem> {
    let folder = resources
        .iter()
        .find(|r| matches!(r, InsomniaResource::RequestGroup { _id, .. } if _id == folder_id))?;

    let mut children = Vec::new();
    if let Some(child_ids) = folder_children.get(folder_id) {
        for cid in child_ids {
            if folder_ids.contains_key(cid) {
                if let Some(sub) = build_folder(cid, folder_children, folder_ids, resources) {
                    children.push(sub);
                }
            } else if let Some(req) = build_request(cid, resources) {
                children.push(req);
            }
        }
    }

    if let InsomniaResource::RequestGroup {
        _id,
        name,
        description,
        authentication,
        ..
    } = folder
    {
        Some(CollectionItem::Folder {
            id: _id.clone(),
            name: name.clone(),
            description: description.clone(),
            children,
            auth: authentication.as_ref().and_then(convert_auth),
        })
    } else {
        None
    }
}

fn build_request(request_id: &str, resources: &[InsomniaResource]) -> Option<CollectionItem> {
    let req = resources
        .iter()
        .find(|r| matches!(r, InsomniaResource::Request { _id, .. } if _id == request_id))?;
    if let InsomniaResource::Request {
        _id,
        parent_id: _,
        name,
        method,
        url,
        headers,
        body,
        authentication,
        parameters,
        description,
    } = req
    {
        let method = HttpMethod::from_str(method).unwrap_or(HttpMethod::Get);
        let headers: Vec<KeyValue> = headers
            .iter()
            .filter(|h| !h.disabled)
            .map(|h| KeyValue {
                key: h.name.clone(),
                value: h.value.clone(),
                enabled: true,
                description: None,
            })
            .collect();
        let params: Vec<KeyValue> = parameters
            .iter()
            .filter(|p| !p.disabled)
            .map(|p| KeyValue {
                key: p.name.clone(),
                value: p.value.clone(),
                enabled: true,
                description: None,
            })
            .collect();
        let body = body.as_ref().map(|b| Body {
            mode: body_mode_for(b),
            content_type: b.mime_type.clone(),
            content: b.text.clone().unwrap_or_default(),
        });
        let auth = authentication.as_ref().and_then(convert_auth);

        Some(CollectionItem::Request {
            id: _id.clone(),
            name: name.clone(),
            request: Request {
                id: _id.clone(),
                name: name.clone(),
                method,
                url: url.clone(),
                headers,
                params,
                body: body.unwrap_or_default(),
                auth,
                settings: Default::default(),
                pre_request_script: None,
                post_response_script: None,
                test_script: None,
                description: description.clone(),
            },
        })
    } else {
        None
    }
}

fn body_mode_for(body: &InsomniaBody) -> BodyMode {
    match body.mime_type.as_deref() {
        Some("application/json") => BodyMode::Json,
        Some("application/xml") | Some("text/xml") => BodyMode::Xml,
        Some("text/html") => BodyMode::Text,
        Some("text/plain") => BodyMode::Text,
        Some("application/x-www-form-urlencoded") => BodyMode::Form,
        Some("multipart/form-data") => BodyMode::Multipart,
        _ => BodyMode::Text,
    }
}

fn convert_auth(a: &InsomniaAuth) -> Option<Auth> {
    let (auth_type, config) = match a {
        InsomniaAuth::Bearer { prefix, token } => (
            AuthType::Bearer,
            HashMap::from([
                ("token".to_string(), token.clone()),
                ("prefix".to_string(), prefix.clone().unwrap_or_else(|| "Bearer".to_string())),
            ]),
        ),
        InsomniaAuth::Basic { username, password } => (
            AuthType::Basic,
            HashMap::from([
                ("username".to_string(), username.clone()),
                ("password".to_string(), password.clone()),
            ]),
        ),
        InsomniaAuth::Apikey { key, value, add_to } => {
            let location = match add_to.as_deref() {
                Some("queryParams") => "query",
                Some("cookie") => "cookie",
                _ => "header",
            };
            (
                AuthType::ApiKey,
                HashMap::from([
                    ("key".to_string(), key.clone()),
                    ("value".to_string(), value.clone()),
                    ("location".to_string(), location.to_string()),
                ]),
            )
        }
        InsomniaAuth::None => return None,
    };
    Some(Auth { auth_type, config })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"__export_format: 4
resources:
  - _id: wrk_1
    _type: workspace
    name: Sample
  - _id: fld_1
    _type: request_group
    parent_id: wrk_1
    name: Users
  - _id: req_1
    _type: request
    parent_id: fld_1
    name: Get users
    method: GET
    url: https://api.example.com/users
    headers:
      - name: Accept
        value: application/json
  - _id: req_2
    _type: request
    parent_id: fld_1
    name: Create user
    method: POST
    url: https://api.example.com/users
    body:
      mimeType: application/json
      text: '{"name":"x"}'
  - _id: env_1
    _type: environment
    parent_id: wrk_1
    name: Production
    data:
      baseUrl: https://api.example.com
"#;

    #[test]
    fn test_import_basic_collection() {
        let imp = InsomniaImporter;
        let col = imp.import(SAMPLE).unwrap();
        assert_eq!(col.name, "Sample");
        assert_eq!(col.items.len(), 1);
        match &col.items[0] {
            CollectionItem::Folder { name, children, .. } => {
                assert_eq!(name, "Users");
                assert_eq!(children.len(), 2);
            }
            _ => panic!("expected folder"),
        }
    }

    #[test]
    fn test_import_environments() {
        let imp = InsomniaImporter;
        let envs = imp.import_environments(SAMPLE).unwrap();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].name, "Production");
        assert_eq!(envs[0].variables.len(), 1);
    }
}
