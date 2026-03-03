use crate::ApiResponse;
use crate::models::{ExtractionRule, ExtractionSource, ExtractedVariable};

/// Apply extraction rules to a response, returning one ExtractedVariable per rule.
pub fn apply_extraction_rules(
    rules: &[ExtractionRule],
    response: &ApiResponse,
) -> Vec<ExtractedVariable> {
    rules.iter().map(|rule| extract_one(rule, response)).collect()
}

fn extract_one(rule: &ExtractionRule, response: &ApiResponse) -> ExtractedVariable {
    let variable_name = rule.target_variable.clone();
    match &rule.source {
        ExtractionSource::Status => ExtractedVariable {
            variable_name,
            value: Some(response.status.to_string()),
            error: None,
        },
        ExtractionSource::ResponseBody => ExtractedVariable {
            variable_name,
            value: Some(response.body.clone()),
            error: None,
        },
        ExtractionSource::Header { name } => {
            // Case-insensitive header lookup
            let lower = name.to_lowercase();
            let found = response
                .headers
                .iter()
                .find(|(k, _)| k.to_lowercase() == lower)
                .map(|(_, v)| v.clone());
            match found {
                Some(val) => ExtractedVariable {
                    variable_name,
                    value: Some(val),
                    error: None,
                },
                None => ExtractedVariable {
                    variable_name,
                    value: None,
                    error: Some(format!("Header '{name}' not found in response")),
                },
            }
        }
        ExtractionSource::JsonPath { path } => extract_json_path(&variable_name, path, &response.body),
    }
}

fn extract_json_path(variable_name: &str, path: &str, body: &str) -> ExtractedVariable {
    let json_value: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return ExtractedVariable {
                variable_name: variable_name.to_string(),
                value: None,
                error: Some(format!("Response body is not valid JSON: {e}")),
            };
        }
    };

    let json_path = match serde_json_path::JsonPath::parse(path) {
        Ok(p) => p,
        Err(e) => {
            return ExtractedVariable {
                variable_name: variable_name.to_string(),
                value: None,
                error: Some(format!("Invalid JSONPath '{path}': {e}")),
            };
        }
    };

    let node_list = json_path.query(&json_value);
    match node_list.first() {
        Some(val) => {
            let string_val = match val {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            ExtractedVariable {
                variable_name: variable_name.to_string(),
                value: Some(string_val),
                error: None,
            }
        }
        None => ExtractedVariable {
            variable_name: variable_name.to_string(),
            value: None,
            error: Some(format!("JSONPath '{path}' matched no values")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_response(status: u16, headers: Vec<(&str, &str)>, body: &str) -> ApiResponse {
        ApiResponse {
            status,
            headers: headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>(),
            body: body.to_string(),
            duration_ms: 100,
            content_type: None,
            size_bytes: Some(body.len() as u64),
        }
    }

    #[test]
    fn test_extract_status() {
        let resp = make_response(201, vec![], "");
        let rules = vec![ExtractionRule {
            source: ExtractionSource::Status,
            target_variable: "status_code".to_string(),
        }];
        let results = apply_extraction_rules(&rules, &resp);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value.as_deref(), Some("201"));
        assert!(results[0].error.is_none());
    }

    #[test]
    fn test_extract_response_body() {
        let resp = make_response(200, vec![], "hello world");
        let rules = vec![ExtractionRule {
            source: ExtractionSource::ResponseBody,
            target_variable: "full_body".to_string(),
        }];
        let results = apply_extraction_rules(&rules, &resp);
        assert_eq!(results[0].value.as_deref(), Some("hello world"));
    }

    #[test]
    fn test_extract_header() {
        let resp = make_response(200, vec![("Content-Type", "application/json")], "");
        let rules = vec![ExtractionRule {
            source: ExtractionSource::Header {
                name: "content-type".to_string(),
            },
            target_variable: "ct".to_string(),
        }];
        let results = apply_extraction_rules(&rules, &resp);
        assert_eq!(results[0].value.as_deref(), Some("application/json"));
    }

    #[test]
    fn test_extract_header_missing() {
        let resp = make_response(200, vec![], "");
        let rules = vec![ExtractionRule {
            source: ExtractionSource::Header {
                name: "X-Missing".to_string(),
            },
            target_variable: "missing".to_string(),
        }];
        let results = apply_extraction_rules(&rules, &resp);
        assert!(results[0].value.is_none());
        assert!(results[0].error.is_some());
    }

    #[test]
    fn test_extract_json_path_string() {
        let body = r#"{"user": {"name": "Alice", "id": 42}}"#;
        let resp = make_response(200, vec![], body);
        let rules = vec![ExtractionRule {
            source: ExtractionSource::JsonPath {
                path: "$.user.name".to_string(),
            },
            target_variable: "username".to_string(),
        }];
        let results = apply_extraction_rules(&rules, &resp);
        assert_eq!(results[0].value.as_deref(), Some("Alice"));
    }

    #[test]
    fn test_extract_json_path_number() {
        let body = r#"{"user": {"id": 42}}"#;
        let resp = make_response(200, vec![], body);
        let rules = vec![ExtractionRule {
            source: ExtractionSource::JsonPath {
                path: "$.user.id".to_string(),
            },
            target_variable: "user_id".to_string(),
        }];
        let results = apply_extraction_rules(&rules, &resp);
        assert_eq!(results[0].value.as_deref(), Some("42"));
    }

    #[test]
    fn test_extract_json_path_no_match() {
        let body = r#"{"data": "value"}"#;
        let resp = make_response(200, vec![], body);
        let rules = vec![ExtractionRule {
            source: ExtractionSource::JsonPath {
                path: "$.missing.field".to_string(),
            },
            target_variable: "x".to_string(),
        }];
        let results = apply_extraction_rules(&rules, &resp);
        assert!(results[0].value.is_none());
        assert!(results[0].error.as_ref().unwrap().contains("matched no values"));
    }

    #[test]
    fn test_extract_json_path_invalid_json() {
        let resp = make_response(200, vec![], "not json");
        let rules = vec![ExtractionRule {
            source: ExtractionSource::JsonPath {
                path: "$.foo".to_string(),
            },
            target_variable: "x".to_string(),
        }];
        let results = apply_extraction_rules(&rules, &resp);
        assert!(results[0].error.as_ref().unwrap().contains("not valid JSON"));
    }

    #[test]
    fn test_extract_json_path_invalid_path() {
        let body = r#"{"data": 1}"#;
        let resp = make_response(200, vec![], body);
        let rules = vec![ExtractionRule {
            source: ExtractionSource::JsonPath {
                path: "$[invalid".to_string(),
            },
            target_variable: "x".to_string(),
        }];
        let results = apply_extraction_rules(&rules, &resp);
        assert!(results[0].error.is_some());
    }

    #[test]
    fn test_multiple_rules() {
        let body = r#"{"token": "abc123"}"#;
        let resp = make_response(200, vec![("X-Request-Id", "req-1")], body);
        let rules = vec![
            ExtractionRule {
                source: ExtractionSource::Status,
                target_variable: "status".to_string(),
            },
            ExtractionRule {
                source: ExtractionSource::JsonPath {
                    path: "$.token".to_string(),
                },
                target_variable: "auth_token".to_string(),
            },
            ExtractionRule {
                source: ExtractionSource::Header {
                    name: "X-Request-Id".to_string(),
                },
                target_variable: "req_id".to_string(),
            },
        ];
        let results = apply_extraction_rules(&rules, &resp);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].value.as_deref(), Some("200"));
        assert_eq!(results[1].value.as_deref(), Some("abc123"));
        assert_eq!(results[2].value.as_deref(), Some("req-1"));
    }
}
