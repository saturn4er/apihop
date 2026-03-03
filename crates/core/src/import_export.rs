//! Import/Export module for apihop.
//!
//! Supports:
//! - Import: Postman Collection v2.1, OpenAPI 3.x, cURL commands
//! - Export: apihop native JSON, Postman Collection v2.1, cURL command

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::*;
use crate::models::RequestType;
use crate::HttpMethod;

// ============================================================
// apihop Native Format
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApihopExport {
    pub version: String,
    pub collection: ExportCollection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCollection {
    pub name: String,
    pub description: Option<String>,
    pub auth: AuthConfig,
    pub folders: Vec<ExportFolder>,
    pub requests: Vec<ExportRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFolder {
    pub name: String,
    pub folders: Vec<ExportFolder>,
    pub requests: Vec<ExportRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub name: String,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    #[serde(default)]
    pub params: Vec<KeyValueParam>,
    #[serde(default)]
    pub auth: AuthConfig,
}

// ============================================================
// Postman Collection v2.1 Types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanCollection {
    pub info: PostmanInfo,
    #[serde(default)]
    pub item: Vec<PostmanItem>,
    #[serde(default)]
    pub auth: Option<PostmanAuth>,
    #[serde(default)]
    pub variable: Vec<PostmanVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanInfo {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanItem {
    pub name: String,
    #[serde(default)]
    pub item: Vec<PostmanItem>,
    #[serde(default)]
    pub request: Option<PostmanRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanRequest {
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub url: PostmanUrl,
    #[serde(default)]
    pub header: Vec<PostmanKeyValue>,
    #[serde(default)]
    pub body: Option<PostmanBody>,
    #[serde(default)]
    pub auth: Option<PostmanAuth>,
}

fn default_method() -> String {
    "GET".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PostmanUrl {
    Simple(String),
    Detailed(PostmanUrlDetail),
}

impl Default for PostmanUrl {
    fn default() -> Self {
        PostmanUrl::Simple(String::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanUrlDetail {
    #[serde(default)]
    pub raw: String,
    #[serde(default)]
    pub query: Vec<PostmanKeyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanKeyValue {
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanBody {
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub raw: Option<String>,
    #[serde(default)]
    pub urlencoded: Vec<PostmanKeyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanAuth {
    #[serde(rename = "type")]
    pub auth_type: String,
    #[serde(default)]
    pub basic: Vec<PostmanKeyValue>,
    #[serde(default)]
    pub bearer: Vec<PostmanKeyValue>,
    #[serde(default)]
    pub apikey: Vec<PostmanKeyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanVariable {
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
}

// ============================================================
// Import Results
// ============================================================

/// Response type for cURL import, shared between server and desktop.
#[derive(Debug, Clone, Serialize)]
pub struct CurlImportResponse {
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub params: Vec<KeyValueParam>,
    pub auth: AuthConfig,
}

impl From<ImportRequest> for CurlImportResponse {
    fn from(req: ImportRequest) -> Self {
        Self {
            name: req.name,
            method: req.method,
            url: req.url,
            headers: req.headers,
            body: req.body,
            params: req.params,
            auth: req.auth,
        }
    }
}

/// Intermediate representation from any import source.
pub struct ImportResult {
    pub collection_name: String,
    pub description: Option<String>,
    pub auth: AuthConfig,
    pub folders: Vec<ImportFolder>,
    pub requests: Vec<ImportRequest>,
    pub variables: Vec<(String, String)>,
}

pub struct ImportFolder {
    pub name: String,
    pub folders: Vec<ImportFolder>,
    pub requests: Vec<ImportRequest>,
}

pub struct ImportRequest {
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub params: Vec<KeyValueParam>,
    pub auth: AuthConfig,
}

// ============================================================
// Postman Import
// ============================================================

pub fn import_postman(json_str: &str) -> Result<ImportResult, String> {
    let collection: PostmanCollection =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid Postman JSON: {e}"))?;

    let auth = collection
        .auth
        .as_ref()
        .map(convert_postman_auth)
        .unwrap_or_default();

    let variables: Vec<(String, String)> = collection
        .variable
        .iter()
        .filter_map(|v| {
            let val = v.value.as_deref().unwrap_or("");
            if v.key.is_empty() {
                None
            } else {
                Some((v.key.clone(), val.to_string()))
            }
        })
        .collect();

    let (folders, requests) = convert_postman_items(&collection.item);

    Ok(ImportResult {
        collection_name: collection.info.name,
        description: collection.info.description,
        auth,
        folders,
        requests,
        variables,
    })
}

fn convert_postman_items(items: &[PostmanItem]) -> (Vec<ImportFolder>, Vec<ImportRequest>) {
    let mut folders = Vec::new();
    let mut requests = Vec::new();

    for item in items {
        if item.request.is_some() {
            // It's a request
            if let Some(req) = convert_postman_request(item) {
                requests.push(req);
            }
        } else if !item.item.is_empty() {
            // It's a folder
            let (sub_folders, sub_requests) = convert_postman_items(&item.item);
            folders.push(ImportFolder {
                name: item.name.clone(),
                folders: sub_folders,
                requests: sub_requests,
            });
        }
    }

    (folders, requests)
}

fn convert_postman_request(item: &PostmanItem) -> Option<ImportRequest> {
    let req = item.request.as_ref()?;
    let method = parse_http_method(&req.method);

    let (url, params) = match &req.url {
        PostmanUrl::Simple(s) => (s.clone(), Vec::new()),
        PostmanUrl::Detailed(d) => {
            let params: Vec<KeyValueParam> = d
                .query
                .iter()
                .map(|q| KeyValueParam {
                    key: q.key.clone(),
                    value: q.value.clone().unwrap_or_default(),
                    enabled: !q.disabled.unwrap_or(false),
                })
                .collect();
            (d.raw.clone(), params)
        }
    };

    let headers: HashMap<String, String> = req
        .header
        .iter()
        .filter(|h| !h.disabled.unwrap_or(false))
        .map(|h| (h.key.clone(), h.value.clone().unwrap_or_default()))
        .collect();

    let body = req.body.as_ref().and_then(|b| {
        match b.mode.as_deref() {
            Some("raw") => b.raw.clone(),
            Some("urlencoded") => {
                let params: Vec<String> = b
                    .urlencoded
                    .iter()
                    .filter(|kv| !kv.disabled.unwrap_or(false))
                    .map(|kv| {
                        format!(
                            "{}={}",
                            urlencoding::encode(&kv.key),
                            urlencoding::encode(kv.value.as_deref().unwrap_or(""))
                        )
                    })
                    .collect();
                if params.is_empty() { None } else { Some(params.join("&")) }
            }
            _ => None,
        }
    });

    let auth = req
        .auth
        .as_ref()
        .map(convert_postman_auth)
        .unwrap_or_default();

    Some(ImportRequest {
        name: item.name.clone(),
        method,
        url,
        headers,
        body,
        params,
        auth,
    })
}

fn convert_postman_auth(auth: &PostmanAuth) -> AuthConfig {
    fn find_val(kvs: &[PostmanKeyValue], key: &str) -> String {
        kvs.iter()
            .find(|kv| kv.key == key)
            .and_then(|kv| kv.value.clone())
            .unwrap_or_default()
    }

    match auth.auth_type.as_str() {
        "basic" => AuthConfig::Basic {
            username: find_val(&auth.basic, "username"),
            password: find_val(&auth.basic, "password"),
        },
        "bearer" => AuthConfig::Bearer {
            token: find_val(&auth.bearer, "token"),
        },
        "apikey" => AuthConfig::ApiKey {
            key: find_val(&auth.apikey, "key"),
            value: find_val(&auth.apikey, "value"),
            add_to: if find_val(&auth.apikey, "in") == "query" {
                ApiKeyLocation::QueryParam
            } else {
                ApiKeyLocation::Header
            },
        },
        _ => AuthConfig::None,
    }
}

// ============================================================
// OpenAPI 3.x Import
// ============================================================

pub fn import_openapi(content: &str) -> Result<ImportResult, String> {
    // Try JSON first, then YAML
    let doc: serde_json::Value = serde_json::from_str(content)
        .or_else(|_| {
            serde_yaml::from_str::<serde_json::Value>(content)
                .map_err(|e| serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())))
        })
        .map_err(|e| format!("Invalid OpenAPI spec: {e}"))?;

    let title = doc
        .pointer("/info/title")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported API")
        .to_string();
    let description = doc
        .pointer("/info/description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract base URL from servers
    let base_url = doc
        .pointer("/servers/0/url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let paths = doc
        .get("paths")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    // Group operations by tag
    let mut tag_map: HashMap<String, Vec<ImportRequest>> = HashMap::new();
    let mut untagged: Vec<ImportRequest> = Vec::new();

    for (path, methods) in &paths {
        let methods_obj = match methods.as_object() {
            Some(o) => o,
            None => continue,
        };

        for (method, operation) in methods_obj {
            let http_method = match method.as_str() {
                "get" | "post" | "put" | "patch" | "delete" | "head" | "options" => {
                    parse_http_method(&method.to_uppercase())
                }
                _ => continue, // skip parameters, summary, etc.
            };

            let op_id = operation
                .get("operationId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let summary = operation
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let name = if !summary.is_empty() {
                summary.to_string()
            } else if !op_id.is_empty() {
                op_id.to_string()
            } else {
                format!("{} {}", method.to_uppercase(), path)
            };

            let url = format!("{}{}", base_url, path);

            // Extract query parameters
            let params: Vec<KeyValueParam> = operation
                .get("parameters")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter(|p| p.get("in").and_then(|v| v.as_str()) == Some("query"))
                        .map(|p| KeyValueParam {
                            key: p
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            value: String::new(),
                            enabled: true,
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Extract headers from parameters
            let headers: HashMap<String, String> = operation
                .get("parameters")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter(|p| p.get("in").and_then(|v| v.as_str()) == Some("header"))
                        .map(|p| {
                            (
                                p.get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                String::new(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            let request = ImportRequest {
                name,
                method: http_method,
                url,
                headers,
                body: None,
                params,
                auth: AuthConfig::None,
            };

            let tags = operation
                .get("tags")
                .and_then(|v| v.as_array());

            if let Some(tags) = tags {
                if let Some(tag) = tags.first().and_then(|t| t.as_str()) {
                    tag_map.entry(tag.to_string()).or_default().push(request);
                    continue;
                }
            }
            untagged.push(request);
        }
    }

    let mut folders: Vec<ImportFolder> = tag_map
        .into_iter()
        .map(|(tag, reqs)| ImportFolder {
            name: tag,
            folders: Vec::new(),
            requests: reqs,
        })
        .collect();
    folders.sort_by(|a, b| a.name.cmp(&b.name));

    // Base URL as a variable
    let mut variables = Vec::new();
    if !base_url.is_empty() {
        variables.push(("base_url".to_string(), base_url));
    }

    Ok(ImportResult {
        collection_name: title,
        description,
        auth: AuthConfig::None,
        folders,
        requests: untagged,
        variables,
    })
}

// ============================================================
// cURL Import
// ============================================================

pub fn import_curl(curl_cmd: &str) -> Result<ImportRequest, String> {
    let trimmed = curl_cmd.trim();
    if !trimmed.starts_with("curl ") && !trimmed.starts_with("curl\t") {
        return Err("Not a curl command".into());
    }

    let tokens = tokenize_curl(trimmed)?;
    let mut method = None;
    let mut url = String::new();
    let mut headers: HashMap<String, String> = HashMap::new();
    let mut body: Option<String> = None;
    let mut user: Option<String> = None;

    let mut i = 1; // skip "curl"
    while i < tokens.len() {
        let tok = &tokens[i];
        match tok.as_str() {
            "-X" | "--request" => {
                i += 1;
                if i < tokens.len() {
                    method = Some(tokens[i].clone());
                }
            }
            "-H" | "--header" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((k, v)) = tokens[i].split_once(':') {
                        headers.insert(k.trim().to_string(), v.trim().to_string());
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                i += 1;
                if i < tokens.len() {
                    body = Some(tokens[i].clone());
                }
            }
            "-u" | "--user" => {
                i += 1;
                if i < tokens.len() {
                    user = Some(tokens[i].clone());
                }
            }
            "--compressed" | "-s" | "--silent" | "-S" | "--show-error" | "-k"
            | "--insecure" | "-L" | "--location" | "-v" | "--verbose" => {
                // flags without values, skip
            }
            _ => {
                // Assume it's the URL if it looks like one
                let s = tok.trim_matches('\'').trim_matches('"');
                if (s.starts_with("http://") || s.starts_with("https://") || s.starts_with("{{"))
                    && url.is_empty()
                {
                    url = s.to_string();
                }
            }
        }
        i += 1;
    }

    if url.is_empty() {
        return Err("No URL found in curl command".into());
    }

    // Determine method
    let method_str = method.unwrap_or_else(|| {
        if body.is_some() {
            "POST".into()
        } else {
            "GET".into()
        }
    });

    // Auth from -u flag
    let auth = if let Some(user_str) = user {
        if let Some((username, password)) = user_str.split_once(':') {
            AuthConfig::Basic {
                username: username.to_string(),
                password: password.to_string(),
            }
        } else {
            AuthConfig::Basic {
                username: user_str,
                password: String::new(),
            }
        }
    } else {
        AuthConfig::None
    };

    // Extract query params from URL
    let (base_url, params) = if let Some(q_idx) = url.find('?') {
        let base = url[..q_idx].to_string();
        let qs = &url[q_idx + 1..];
        let params: Vec<KeyValueParam> = qs
            .split('&')
            .filter(|s| !s.is_empty())
            .map(|pair| {
                let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                KeyValueParam {
                    key: urlencoding::decode(k).unwrap_or_default().into_owned(),
                    value: urlencoding::decode(v).unwrap_or_default().into_owned(),
                    enabled: true,
                }
            })
            .collect();
        (base, params)
    } else {
        (url.clone(), Vec::new())
    };

    Ok(ImportRequest {
        name: format!("{} {}", method_str, base_url),
        method: parse_http_method(&method_str),
        url: base_url,
        headers,
        body,
        params,
        auth,
    })
}

/// Simple shell-like tokenizer for curl commands.
fn tokenize_curl(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for ch in input.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        if ch == '\\' && !in_single_quote {
            // Line continuation
            escape_next = true;
            continue;
        }

        if ch == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }

        if ch == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }

        if ch.is_whitespace() && !in_single_quote && !in_double_quote {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }

        current.push(ch);
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

// ============================================================
// Export Functions
// ============================================================

/// Export a collection + its folders/requests to apihop native JSON format.
pub fn export_apihop(
    collection: &Collection,
    folders: &[(Folder, Vec<SavedRequest>, Vec<(Folder, Vec<SavedRequest>)>)],
    root_requests: &[SavedRequest],
) -> serde_json::Value {
    let export = ApihopExport {
        version: "1.0".into(),
        collection: ExportCollection {
            name: collection.name.clone(),
            description: collection.description.clone(),
            auth: collection.auth.clone(),
            folders: folders
                .iter()
                .map(|(f, reqs, sub_folders)| export_folder(f, reqs, sub_folders))
                .collect(),
            requests: root_requests.iter().map(export_request).collect(),
        },
    };
    serde_json::to_value(&export).unwrap_or_default()
}

fn export_folder(
    folder: &Folder,
    requests: &[SavedRequest],
    sub_folders: &[(Folder, Vec<SavedRequest>)],
) -> ExportFolder {
    ExportFolder {
        name: folder.name.clone(),
        folders: sub_folders
            .iter()
            .map(|(f, reqs)| ExportFolder {
                name: f.name.clone(),
                folders: Vec::new(),
                requests: reqs.iter().map(export_request).collect(),
            })
            .collect(),
        requests: requests.iter().map(export_request).collect(),
    }
}

fn export_request(req: &SavedRequest) -> ExportRequest {
    ExportRequest {
        name: req.name.clone(),
        method: req.method.to_string(),
        url: req.url.clone(),
        headers: req.headers.clone(),
        body: req.body.clone(),
        params: req.params.clone(),
        auth: req.auth.clone(),
    }
}

/// Export a collection as Postman Collection v2.1 JSON.
pub fn export_postman(
    collection: &Collection,
    folders: &[(Folder, Vec<SavedRequest>, Vec<(Folder, Vec<SavedRequest>)>)],
    root_requests: &[SavedRequest],
) -> serde_json::Value {
    let mut items: Vec<PostmanItem> = Vec::new();

    for (folder, reqs, sub_folders) in folders {
        let mut folder_items: Vec<PostmanItem> = Vec::new();
        for req in reqs {
            folder_items.push(saved_request_to_postman_item(req));
        }
        for (sf, sf_reqs) in sub_folders {
            let sub_items: Vec<PostmanItem> =
                sf_reqs.iter().map(saved_request_to_postman_item).collect();
            folder_items.push(PostmanItem {
                name: sf.name.clone(),
                item: sub_items,
                request: None,
            });
        }
        items.push(PostmanItem {
            name: folder.name.clone(),
            item: folder_items,
            request: None,
        });
    }

    for req in root_requests {
        items.push(saved_request_to_postman_item(req));
    }

    let postman = PostmanCollection {
        info: PostmanInfo {
            name: collection.name.clone(),
            description: collection.description.clone(),
            schema: Some(
                "https://schema.getpostman.com/json/collection/v2.1.0/collection.json".into(),
            ),
        },
        item: items,
        auth: match &collection.auth {
            AuthConfig::None => None,
            auth => Some(auth_to_postman(auth)),
        },
        variable: Vec::new(),
    };

    serde_json::to_value(&postman).unwrap_or_default()
}

fn saved_request_to_postman_item(req: &SavedRequest) -> PostmanItem {
    let method_str = req.method.to_string();

    let headers: Vec<PostmanKeyValue> = req
        .headers
        .iter()
        .map(|(k, v)| PostmanKeyValue {
            key: k.clone(),
            value: Some(v.clone()),
            disabled: None,
        })
        .collect();

    let query: Vec<PostmanKeyValue> = req
        .params
        .iter()
        .map(|p| PostmanKeyValue {
            key: p.key.clone(),
            value: Some(p.value.clone()),
            disabled: Some(!p.enabled),
        })
        .collect();

    let body = req.body.as_ref().map(|b| PostmanBody {
        mode: Some("raw".into()),
        raw: Some(b.clone()),
        urlencoded: Vec::new(),
    });

    let auth = match &req.auth {
        AuthConfig::None => None,
        a => Some(auth_to_postman(a)),
    };

    PostmanItem {
        name: req.name.clone(),
        item: Vec::new(),
        request: Some(PostmanRequest {
            method: method_str,
            url: PostmanUrl::Detailed(PostmanUrlDetail {
                raw: req.url.clone(),
                query,
            }),
            header: headers,
            body,
            auth,
        }),
    }
}

fn auth_to_postman(auth: &AuthConfig) -> PostmanAuth {
    match auth {
        AuthConfig::Basic { username, password } => PostmanAuth {
            auth_type: "basic".into(),
            basic: vec![
                PostmanKeyValue {
                    key: "username".into(),
                    value: Some(username.clone()),
                    disabled: None,
                },
                PostmanKeyValue {
                    key: "password".into(),
                    value: Some(password.clone()),
                    disabled: None,
                },
            ],
            bearer: Vec::new(),
            apikey: Vec::new(),
        },
        AuthConfig::Bearer { token } => PostmanAuth {
            auth_type: "bearer".into(),
            basic: Vec::new(),
            bearer: vec![PostmanKeyValue {
                key: "token".into(),
                value: Some(token.clone()),
                disabled: None,
            }],
            apikey: Vec::new(),
        },
        AuthConfig::ApiKey { key, value, add_to } => PostmanAuth {
            auth_type: "apikey".into(),
            basic: Vec::new(),
            bearer: Vec::new(),
            apikey: vec![
                PostmanKeyValue {
                    key: "key".into(),
                    value: Some(key.clone()),
                    disabled: None,
                },
                PostmanKeyValue {
                    key: "value".into(),
                    value: Some(value.clone()),
                    disabled: None,
                },
                PostmanKeyValue {
                    key: "in".into(),
                    value: Some(match add_to {
                        ApiKeyLocation::Header => "header".into(),
                        ApiKeyLocation::QueryParam => "query".into(),
                    }),
                    disabled: None,
                },
            ],
        },
        _ => PostmanAuth {
            auth_type: "noauth".into(),
            basic: Vec::new(),
            bearer: Vec::new(),
            apikey: Vec::new(),
        },
    }
}

/// Export a single request as a cURL command.
pub fn export_curl(req: &SavedRequest) -> String {
    let mut parts = vec![format!("curl -X {}", req.method)];

    // URL with params
    let mut url = req.url.clone();
    let enabled_params: Vec<_> = req.params.iter().filter(|p| p.enabled && !p.key.is_empty()).collect();
    if !enabled_params.is_empty() {
        let qs: Vec<String> = enabled_params
            .iter()
            .map(|p| {
                format!(
                    "{}={}",
                    urlencoding::encode(&p.key),
                    urlencoding::encode(&p.value)
                )
            })
            .collect();
        url = format!("{}?{}", url, qs.join("&"));
    }
    parts.push(format!("'{}'", url.replace('\'', "'\\''")));

    // Headers
    for (k, v) in &req.headers {
        parts.push(format!("-H '{}: {}'", k, v.replace('\'', "'\\''")));
    }

    // Auth
    match &req.auth {
        AuthConfig::Basic { username, password } => {
            parts.push(format!("-u '{}:{}'", username, password));
        }
        AuthConfig::Bearer { token } => {
            parts.push(format!("-H 'Authorization: Bearer {}'", token));
        }
        AuthConfig::ApiKey { key, value, add_to } => match add_to {
            ApiKeyLocation::Header => {
                parts.push(format!("-H '{}: {}'", key, value));
            }
            ApiKeyLocation::QueryParam => {
                // Already handled in URL if params were there
            }
        },
        _ => {}
    }

    // Body
    if let Some(body) = &req.body {
        parts.push(format!("-d '{}'", body.replace('\'', "'\\''")));
    }

    parts.join(" \\\n  ")
}

/// Re-import an apihop native JSON export.
pub fn import_apihop(json_str: &str) -> Result<ImportResult, String> {
    let export: ApihopExport =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid apihop JSON: {e}"))?;

    let col = export.collection;
    let (folders, requests) = convert_export_items(&col.folders, &col.requests);

    Ok(ImportResult {
        collection_name: col.name,
        description: col.description,
        auth: col.auth,
        folders,
        requests,
        variables: Vec::new(),
    })
}

fn convert_export_items(
    export_folders: &[ExportFolder],
    export_requests: &[ExportRequest],
) -> (Vec<ImportFolder>, Vec<ImportRequest>) {
    let folders = export_folders
        .iter()
        .map(|f| {
            let (sub_folders, sub_requests) = convert_export_items(&f.folders, &f.requests);
            ImportFolder {
                name: f.name.clone(),
                folders: sub_folders,
                requests: sub_requests,
            }
        })
        .collect();

    let requests = export_requests
        .iter()
        .map(|r| ImportRequest {
            name: r.name.clone(),
            method: parse_http_method(&r.method),
            url: r.url.clone(),
            headers: r.headers.clone(),
            body: r.body.clone(),
            params: r.params.clone(),
            auth: r.auth.clone(),
        })
        .collect();

    (folders, requests)
}

// ============================================================
// Helpers
// ============================================================

fn parse_http_method(s: &str) -> HttpMethod {
    s.parse().unwrap_or(HttpMethod::Get)
}

/// Helper to persist an ImportResult to storage.
pub async fn persist_import(
    result: ImportResult,
    storage: &dyn crate::storage::StorageBackend,
    user_id: Option<&str>,
) -> Result<Collection, crate::storage::StorageError> {
    let collection = storage
        .create_collection(
            &result.collection_name,
            result.description.as_deref(),
            Some(&result.auth),
            None,
            None,
            user_id,
            None,
        )
        .await?;

    // Create root-level requests
    for req in &result.requests {
        let saved = SavedRequest {
            id: String::new(),
            collection_id: collection.id.clone(),
            folder_id: None,
            name: req.name.clone(),
            method: req.method.clone(),
            url: req.url.clone(),
            headers: req.headers.clone(),
            body: req.body.clone(),
            params: req.params.clone(),
            auth: req.auth.clone(),
            pre_request_script: None,
            test_script: None,
            request_type: RequestType::Http,
            graphql_query: None,
            graphql_variables: None,
            graphql_operation_name: None,
            extraction_rules: None,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        };
        storage.create_request(&saved, user_id).await?;
    }

    // Create folders recursively
    persist_folders(&collection.id, None, &result.folders, storage, user_id).await?;

    Ok(collection)
}

#[async_recursion::async_recursion]
async fn persist_folders(
    collection_id: &str,
    parent_id: Option<&str>,
    folders: &[ImportFolder],
    storage: &dyn crate::storage::StorageBackend,
    user_id: Option<&str>,
) -> Result<(), crate::storage::StorageError> {
    for folder in folders {
        let created = storage
            .create_folder(collection_id, parent_id, &folder.name, user_id)
            .await?;

        for req in &folder.requests {
            let saved = SavedRequest {
                id: String::new(),
                collection_id: collection_id.to_string(),
                folder_id: Some(created.id.clone()),
                name: req.name.clone(),
                method: req.method.clone(),
                url: req.url.clone(),
                headers: req.headers.clone(),
                body: req.body.clone(),
                params: req.params.clone(),
                auth: req.auth.clone(),
                pre_request_script: None,
                test_script: None,
                request_type: RequestType::Http,
                graphql_query: None,
                graphql_variables: None,
                graphql_operation_name: None,
                extraction_rules: None,
                sort_order: 0,
                created_at: String::new(),
                updated_at: String::new(),
            };
            storage.create_request(&saved, user_id).await?;
        }

        persist_folders(collection_id, Some(&created.id), &folder.folders, storage, user_id).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_method() {
        assert_eq!(parse_http_method("GET").to_string(), "GET");
        assert_eq!(parse_http_method("post").to_string(), "POST");
        assert_eq!(parse_http_method("Delete").to_string(), "DELETE");
        // Unknown defaults to GET
        assert_eq!(parse_http_method("INVALID").to_string(), "GET");
    }

    #[test]
    fn test_import_curl_basic() {
        let req = import_curl("curl https://example.com/api").unwrap();
        assert_eq!(req.url, "https://example.com/api");
        assert_eq!(req.method.to_string(), "GET");
    }

    #[test]
    fn test_import_curl_with_method_and_body() {
        let req = import_curl("curl -X POST -d '{\"key\":\"val\"}' https://example.com/api").unwrap();
        assert_eq!(req.method.to_string(), "POST");
        assert_eq!(req.body.as_deref(), Some("{\"key\":\"val\"}"));
    }

    #[test]
    fn test_import_curl_with_auth() {
        let req = import_curl("curl -u user:pass https://example.com/api").unwrap();
        match &req.auth {
            AuthConfig::Basic { username, password } => {
                assert_eq!(username, "user");
                assert_eq!(password, "pass");
            }
            _ => panic!("Expected Basic auth"),
        }
    }

    #[test]
    fn test_import_curl_not_curl() {
        assert!(import_curl("wget https://example.com").is_err());
    }

    #[test]
    fn test_import_curl_with_query_params() {
        let req = import_curl("curl 'https://example.com/api?foo=bar&baz=qux'").unwrap();
        assert_eq!(req.url, "https://example.com/api");
        assert_eq!(req.params.len(), 2);
        assert_eq!(req.params[0].key, "foo");
        assert_eq!(req.params[0].value, "bar");
    }
}
