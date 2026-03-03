//! GraphQL schema introspection support.
//!
//! Executes the standard introspection query against a GraphQL endpoint and
//! parses the result into typed schema structs for use by the frontend.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{ApiError, HTTP_CLIENT};

/// The standard GraphQL introspection query.
const INTROSPECTION_QUERY: &str = r#"
  query IntrospectionQuery {
    __schema {
      queryType { name }
      mutationType { name }
      subscriptionType { name }
      types {
        kind
        name
        description
        fields(includeDeprecated: true) {
          name
          description
          args {
            name
            description
            type { ...TypeRef }
            defaultValue
          }
          type { ...TypeRef }
          isDeprecated
          deprecationReason
        }
        inputFields {
          name
          description
          type { ...TypeRef }
          defaultValue
        }
        interfaces { ...TypeRef }
        enumValues(includeDeprecated: true) {
          name
          description
          isDeprecated
          deprecationReason
        }
        possibleTypes { ...TypeRef }
      }
      directives {
        name
        description
        locations
        args {
          name
          description
          type { ...TypeRef }
          defaultValue
        }
      }
    }
  }

  fragment TypeRef on __Type {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
              ofType {
                kind
                name
              }
            }
          }
        }
      }
    }
  }
"#;

// ── Schema types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLSchema {
    pub query_type: Option<String>,
    pub mutation_type: Option<String>,
    pub subscription_type: Option<String>,
    pub types: Vec<GraphQLType>,
    pub directives: Vec<GraphQLDirective>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLType {
    pub kind: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub fields: Option<Vec<GraphQLField>>,
    pub input_fields: Option<Vec<GraphQLInputValue>>,
    pub interfaces: Option<Vec<GraphQLTypeRef>>,
    pub enum_values: Option<Vec<GraphQLEnumValue>>,
    pub possible_types: Option<Vec<GraphQLTypeRef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLField {
    pub name: String,
    pub description: Option<String>,
    pub args: Vec<GraphQLInputValue>,
    #[serde(rename = "type")]
    pub field_type: GraphQLTypeRef,
    pub is_deprecated: bool,
    pub deprecation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLInputValue {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub value_type: GraphQLTypeRef,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLTypeRef {
    pub kind: String,
    pub name: Option<String>,
    pub of_type: Option<Box<GraphQLTypeRef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLEnumValue {
    pub name: String,
    pub description: Option<String>,
    pub is_deprecated: bool,
    pub deprecation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLDirective {
    pub name: String,
    pub description: Option<String>,
    pub locations: Vec<String>,
    pub args: Vec<GraphQLInputValue>,
}

// ── Introspection response types (for JSON parsing) ──────────────────

#[derive(Deserialize)]
struct IntrospectionResponse {
    data: Option<IntrospectionData>,
    errors: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
struct IntrospectionData {
    #[serde(rename = "__schema")]
    schema: RawSchema,
}

#[derive(Deserialize)]
struct RawSchema {
    #[serde(rename = "queryType")]
    query_type: Option<NameOnly>,
    #[serde(rename = "mutationType")]
    mutation_type: Option<NameOnly>,
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<NameOnly>,
    types: Vec<RawType>,
    directives: Vec<RawDirective>,
}

#[derive(Deserialize)]
struct NameOnly {
    name: Option<String>,
}

#[derive(Deserialize)]
struct RawType {
    kind: String,
    name: Option<String>,
    description: Option<String>,
    fields: Option<Vec<RawField>>,
    #[serde(rename = "inputFields")]
    input_fields: Option<Vec<RawInputValue>>,
    interfaces: Option<Vec<RawTypeRef>>,
    #[serde(rename = "enumValues")]
    enum_values: Option<Vec<RawEnumValue>>,
    #[serde(rename = "possibleTypes")]
    possible_types: Option<Vec<RawTypeRef>>,
}

#[derive(Deserialize)]
struct RawField {
    name: String,
    description: Option<String>,
    args: Vec<RawInputValue>,
    #[serde(rename = "type")]
    field_type: RawTypeRef,
    #[serde(rename = "isDeprecated")]
    is_deprecated: bool,
    #[serde(rename = "deprecationReason")]
    deprecation_reason: Option<String>,
}

#[derive(Deserialize)]
struct RawInputValue {
    name: String,
    description: Option<String>,
    #[serde(rename = "type")]
    value_type: RawTypeRef,
    #[serde(rename = "defaultValue")]
    default_value: Option<String>,
}

#[derive(Deserialize)]
struct RawTypeRef {
    kind: String,
    name: Option<String>,
    #[serde(rename = "ofType")]
    of_type: Option<Box<RawTypeRef>>,
}

#[derive(Deserialize)]
struct RawEnumValue {
    name: String,
    description: Option<String>,
    #[serde(rename = "isDeprecated")]
    is_deprecated: bool,
    #[serde(rename = "deprecationReason")]
    deprecation_reason: Option<String>,
}

#[derive(Deserialize)]
struct RawDirective {
    name: String,
    description: Option<String>,
    locations: Vec<String>,
    args: Vec<RawInputValue>,
}

// ── Conversion helpers ───────────────────────────────────────────────

fn convert_type_ref(raw: &RawTypeRef) -> GraphQLTypeRef {
    GraphQLTypeRef {
        kind: raw.kind.clone(),
        name: raw.name.clone(),
        of_type: raw.of_type.as_ref().map(|t| Box::new(convert_type_ref(t))),
    }
}

fn convert_input_value(raw: &RawInputValue) -> GraphQLInputValue {
    GraphQLInputValue {
        name: raw.name.clone(),
        description: raw.description.clone(),
        value_type: convert_type_ref(&raw.value_type),
        default_value: raw.default_value.clone(),
    }
}

fn convert_field(raw: &RawField) -> GraphQLField {
    GraphQLField {
        name: raw.name.clone(),
        description: raw.description.clone(),
        args: raw.args.iter().map(convert_input_value).collect(),
        field_type: convert_type_ref(&raw.field_type),
        is_deprecated: raw.is_deprecated,
        deprecation_reason: raw.deprecation_reason.clone(),
    }
}

fn convert_schema(raw: RawSchema) -> GraphQLSchema {
    GraphQLSchema {
        query_type: raw.query_type.and_then(|t| t.name),
        mutation_type: raw.mutation_type.and_then(|t| t.name),
        subscription_type: raw.subscription_type.and_then(|t| t.name),
        types: raw
            .types
            .iter()
            .map(|t| GraphQLType {
                kind: t.kind.clone(),
                name: t.name.clone(),
                description: t.description.clone(),
                fields: t.fields.as_ref().map(|fs| fs.iter().map(convert_field).collect()),
                input_fields: t
                    .input_fields
                    .as_ref()
                    .map(|fs| fs.iter().map(convert_input_value).collect()),
                interfaces: t
                    .interfaces
                    .as_ref()
                    .map(|ts| ts.iter().map(convert_type_ref).collect()),
                enum_values: t.enum_values.as_ref().map(|evs| {
                    evs.iter()
                        .map(|ev| GraphQLEnumValue {
                            name: ev.name.clone(),
                            description: ev.description.clone(),
                            is_deprecated: ev.is_deprecated,
                            deprecation_reason: ev.deprecation_reason.clone(),
                        })
                        .collect()
                }),
                possible_types: t
                    .possible_types
                    .as_ref()
                    .map(|ts| ts.iter().map(convert_type_ref).collect()),
            })
            .collect(),
        directives: raw
            .directives
            .iter()
            .map(|d| GraphQLDirective {
                name: d.name.clone(),
                description: d.description.clone(),
                locations: d.locations.clone(),
                args: d.args.iter().map(convert_input_value).collect(),
            })
            .collect(),
    }
}

// ── Public API ───────────────────────────────────────────────────────

/// Execute a GraphQL introspection query against the given endpoint.
///
/// `headers` should include any auth headers needed to access the endpoint.
pub async fn introspect(
    endpoint_url: &str,
    headers: &HashMap<String, String>,
) -> Result<GraphQLSchema, ApiError> {
    let client = &*HTTP_CLIENT;

    let body = serde_json::json!({
        "query": INTROSPECTION_QUERY,
    });

    let mut builder = client.post(endpoint_url).json(&body);
    for (k, v) in headers {
        builder = builder.header(k, v);
    }

    let response = builder.send().await?;
    let status = response.status();
    let text = response.text().await?;

    if !status.is_success() {
        return Err(ApiError::Auth(format!(
            "Introspection request failed with status {status}: {text}"
        )));
    }

    let parsed: IntrospectionResponse = serde_json::from_str(&text).map_err(|e| {
        ApiError::Auth(format!("Failed to parse introspection response: {e}"))
    })?;

    if let Some(errors) = &parsed.errors {
        if !errors.is_empty() && parsed.data.is_none() {
            return Err(ApiError::Auth(format!(
                "Introspection returned errors: {}",
                serde_json::to_string(errors).unwrap_or_default()
            )));
        }
    }

    match parsed.data {
        Some(data) => Ok(convert_schema(data.schema)),
        None => Err(ApiError::Auth(
            "Introspection response missing data field".into(),
        )),
    }
}
