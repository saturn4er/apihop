pub mod console;
pub mod engine;
pub mod pm_api;

use std::collections::HashMap;

use crate::models::{ConsoleEntry, TestResult};
use crate::{ApiRequest, ApiResponse};

pub struct PreRequestResult {
    pub console: Vec<ConsoleEntry>,
    pub error: Option<String>,
    pub variable_updates: HashMap<String, String>,
    pub environment_updates: HashMap<String, String>,
    pub request_mutations: Option<RequestMutations>,
}

pub struct RequestMutations {
    pub url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
}

pub struct TestScriptResult {
    pub console: Vec<ConsoleEntry>,
    pub error: Option<String>,
    pub test_results: Vec<TestResult>,
    pub variable_updates: HashMap<String, String>,
    pub environment_updates: HashMap<String, String>,
}

pub async fn execute_pre_request_scripts(
    collection_script: Option<&str>,
    request_script: Option<&str>,
    request: &mut ApiRequest,
    variables: &mut HashMap<String, String>,
) -> PreRequestResult {
    let combined = combine_scripts(collection_script, request_script);
    if combined.is_empty() {
        return PreRequestResult {
            console: vec![],
            error: None,
            variable_updates: HashMap::new(),
            environment_updates: HashMap::new(),
            request_mutations: None,
        };
    }

    let request_json = serde_json::to_string(request).unwrap_or_default();
    let vars_json = serde_json::to_string(variables).unwrap_or_default();

    let preamble = format!(
        "{}\n{}\n{}",
        console::CONSOLE_SETUP_JS,
        pm_api::pre_request_pm_js(&request_json, &vars_json),
        &combined,
    );
    let postlude = pm_api::PRE_REQUEST_COLLECT_JS;
    let full_script = format!("{preamble}\n{postlude}");

    // Run JS execution on a blocking thread to avoid starving the async runtime
    let result = tokio::task::spawn_blocking(move || engine::execute_js(&full_script))
        .await
        .unwrap_or_else(|e| Err(format!("Script task panicked: {e}")));

    match result {
        Ok(result_json) => parse_pre_request_result(&result_json, request, variables),
        Err(err) => PreRequestResult {
            console: vec![],
            error: Some(err),
            variable_updates: HashMap::new(),
            environment_updates: HashMap::new(),
            request_mutations: None,
        },
    }
}

pub async fn execute_test_scripts(
    collection_script: Option<&str>,
    request_script: Option<&str>,
    response: &ApiResponse,
    variables: &HashMap<String, String>,
) -> TestScriptResult {
    let combined = combine_scripts(collection_script, request_script);
    if combined.is_empty() {
        return TestScriptResult {
            console: vec![],
            error: None,
            test_results: vec![],
            variable_updates: HashMap::new(),
            environment_updates: HashMap::new(),
        };
    }

    let response_json = serde_json::to_string(response).unwrap_or_default();
    let vars_json = serde_json::to_string(variables).unwrap_or_default();

    let preamble = format!(
        "{}\n{}\n{}",
        console::CONSOLE_SETUP_JS,
        pm_api::test_pm_js(&response_json, &vars_json),
        &combined,
    );
    let postlude = pm_api::TEST_COLLECT_JS;
    let full_script = format!("{preamble}\n{postlude}");

    // Run JS execution on a blocking thread to avoid starving the async runtime
    let result = tokio::task::spawn_blocking(move || engine::execute_js(&full_script))
        .await
        .unwrap_or_else(|e| Err(format!("Script task panicked: {e}")));

    match result {
        Ok(result_json) => parse_test_result(&result_json),
        Err(err) => TestScriptResult {
            console: vec![],
            error: Some(err),
            test_results: vec![],
            variable_updates: HashMap::new(),
            environment_updates: HashMap::new(),
        },
    }
}

fn combine_scripts(collection_script: Option<&str>, request_script: Option<&str>) -> String {
    let mut parts = Vec::new();
    if let Some(s) = collection_script {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_string());
        }
    }
    if let Some(s) = request_script {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_string());
        }
    }
    parts.join("\n")
}

fn parse_pre_request_result(
    json: &str,
    request: &mut ApiRequest,
    variables: &mut HashMap<String, String>,
) -> PreRequestResult {
    let parsed: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            return PreRequestResult {
                console: vec![],
                error: Some(format!("Failed to parse script result: {e}")),
                variable_updates: HashMap::new(),
                environment_updates: HashMap::new(),
                request_mutations: None,
            };
        }
    };

    let console_entries = parse_console_entries(&parsed["console"]);
    let variable_updates = parse_string_map(&parsed["variables"]);
    let environment_updates = parse_string_map(&parsed["environment"]);

    // Apply request mutations
    let mut mutations = None;
    if let Some(req) = parsed.get("request") {
        let mut m = RequestMutations {
            url: None,
            headers: None,
            body: None,
        };
        if let Some(url) = req.get("url").and_then(|v| v.as_str()) {
            request.url = url.to_string();
            m.url = Some(url.to_string());
        }
        if let Some(headers_obj) = req.get("headers").and_then(|v| v.as_object()) {
            let mut new_headers = HashMap::new();
            for (k, v) in headers_obj {
                if let Some(vs) = v.as_str() {
                    new_headers.insert(k.clone(), vs.to_string());
                }
            }
            request.headers = new_headers.clone();
            m.headers = Some(new_headers);
        }
        if let Some(body) = req.get("body") {
            if body.is_null() {
                request.body = None;
                m.body = None;
            } else if let Some(b) = body.as_str() {
                request.body = Some(b.to_string());
                m.body = Some(b.to_string());
            }
        }
        mutations = Some(m);
    }

    // Apply variable updates to the working variable map
    for (k, v) in &variable_updates {
        variables.insert(k.clone(), v.clone());
    }

    PreRequestResult {
        console: console_entries,
        error: parsed
            .get("error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        variable_updates,
        environment_updates,
        request_mutations: mutations,
    }
}

fn parse_test_result(json: &str) -> TestScriptResult {
    let parsed: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            return TestScriptResult {
                console: vec![],
                error: Some(format!("Failed to parse script result: {e}")),
                test_results: vec![],
                variable_updates: HashMap::new(),
                environment_updates: HashMap::new(),
            };
        }
    };

    let console_entries = parse_console_entries(&parsed["console"]);
    let variable_updates = parse_string_map(&parsed["variables"]);
    let environment_updates = parse_string_map(&parsed["environment"]);

    let test_results = parsed
        .get("tests")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|t| TestResult {
                    name: t
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    passed: t
                        .get("passed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    error: t
                        .get("error")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
                .collect()
        })
        .unwrap_or_default();

    TestScriptResult {
        console: console_entries,
        error: parsed
            .get("error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        test_results,
        variable_updates,
        environment_updates,
    }
}

fn parse_console_entries(val: &serde_json::Value) -> Vec<ConsoleEntry> {
    val.as_array()
        .map(|arr| {
            arr.iter()
                .map(|e| ConsoleEntry {
                    level: e
                        .get("level")
                        .and_then(|v| v.as_str())
                        .unwrap_or("log")
                        .to_string(),
                    message: e
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_string_map(val: &serde_json::Value) -> HashMap<String, String> {
    val.as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}
