use std::collections::HashMap;
use std::sync::atomic::Ordering;

use crate::models::{
    self, ScriptExecutionResult, SendRequestPayload, SendRequestResponse,
};
use crate::storage::StorageBackend;
use crate::{
    ApiError, ApiRequest, ApiResponse, HttpMethod, PRUNE_COUNTER, apply_auth, interpolate,
    interpolate_auth, mask_auth_header, scripting, send_request,
};

/// Intermediate state carried through the pipeline stages.
struct PipelineContext {
    payload: SendRequestPayload,
    var_map: HashMap<String, String>,
    final_url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
    unresolved_vars: Vec<String>,
    api_request: ApiRequest,
    collection_pre_script: Option<String>,
    collection_test_script: Option<String>,
    pre_result: scripting::PreRequestResult,
    test_result: scripting::TestScriptResult,
    extracted_variables: Vec<models::ExtractedVariable>,
}

/// Encapsulates the full send-request pipeline: variable interpolation,
/// auth, pre/post scripts, HTTP call, and history recording.
pub struct RequestPipeline<'a> {
    storage: &'a dyn StorageBackend,
    encryption_key: &'a [u8; 32],
    user_id: Option<String>,
}

impl<'a> RequestPipeline<'a> {
    pub fn new(storage: &'a dyn StorageBackend, encryption_key: &'a [u8; 32]) -> Self {
        Self {
            storage,
            encryption_key,
            user_id: None,
        }
    }

    pub fn with_user_id(mut self, user_id: Option<String>) -> Self {
        self.user_id = user_id;
        self
    }

    pub async fn execute(
        &self,
        payload: SendRequestPayload,
    ) -> Result<SendRequestResponse, ApiError> {
        let mut ctx = self.build_context(payload).await?;
        self.interpolate(&mut ctx);
        self.apply_auth(&mut ctx).await?;
        self.load_collection_scripts(&mut ctx).await;
        self.run_pre_scripts(&mut ctx).await;
        let response = self.send(&ctx).await?;
        self.run_test_scripts(&mut ctx, &response).await;
        self.apply_extractions(&mut ctx, &response).await;
        self.persist_env_updates(&ctx).await;
        let history = self.record_history(&ctx, &response).await?;
        Ok(self.build_response(ctx, response, history))
    }

    async fn build_context(
        &self,
        payload: SendRequestPayload,
    ) -> Result<PipelineContext, ApiError> {
        let var_map =
            crate::load_variables(self.storage, payload.environment_id.as_deref(), self.user_id.as_deref()).await?;
        let _ = self.encryption_key; // reserved for future secret variable decryption

        Ok(PipelineContext {
            payload,
            var_map,
            final_url: String::new(),
            headers: HashMap::new(),
            body: None,
            unresolved_vars: Vec::new(),
            api_request: ApiRequest {
                method: HttpMethod::Get,
                url: String::new(),
                headers: HashMap::new(),
                body: None,
            },
            collection_pre_script: None,
            collection_test_script: None,
            pre_result: scripting::PreRequestResult {
                console: vec![],
                error: None,
                variable_updates: HashMap::new(),
                environment_updates: HashMap::new(),
                request_mutations: None,
            },
            test_result: scripting::TestScriptResult {
                console: vec![],
                error: None,
                test_results: vec![],
                variable_updates: HashMap::new(),
                environment_updates: HashMap::new(),
            },
            extracted_variables: Vec::new(),
        })
    }

    fn interpolate(&self, ctx: &mut PipelineContext) {
        let var_map = &ctx.var_map;
        let payload = &ctx.payload;

        let (base_url, mut all_unresolved) = interpolate(&payload.url, var_map);

        let mut req_headers = HashMap::new();
        for (k, v) in &payload.headers {
            let (ik, u1) = interpolate(k, var_map);
            all_unresolved.extend(u1);
            let (iv, u2) = interpolate(v, var_map);
            all_unresolved.extend(u2);
            req_headers.insert(ik, iv);
        }

        // For GraphQL requests, construct JSON body from query+variables+operationName
        let body = if matches!(payload.request_type, models::RequestType::Graphql) {
            req_headers
                .entry("Content-Type".to_string())
                .or_insert_with(|| "application/json".to_string());
            let query = payload.graphql_query.as_deref().unwrap_or("");
            let (interp_query, u) = interpolate(query, var_map);
            all_unresolved.extend(u);

            let variables: serde_json::Value = payload
                .graphql_variables
                .as_deref()
                .and_then(|v| {
                    let (interp_v, u) = interpolate(v, var_map);
                    all_unresolved.extend(u);
                    serde_json::from_str(&interp_v).ok()
                })
                .unwrap_or(serde_json::Value::Null);

            let op_name = payload.graphql_operation_name.as_deref().map(|n| {
                let (interp_n, u) = interpolate(n, var_map);
                all_unresolved.extend(u);
                interp_n
            });

            let mut gql_body = serde_json::json!({ "query": interp_query });
            if !variables.is_null() {
                gql_body["variables"] = variables;
            }
            if let Some(name) = op_name {
                if !name.is_empty() {
                    gql_body["operationName"] = serde_json::Value::String(name);
                }
            }
            Some(serde_json::to_string(&gql_body).unwrap_or_default())
        } else {
            payload.body.as_ref().map(|b| {
                let (ib, u) = interpolate(b, var_map);
                all_unresolved.extend(u);
                ib
            })
        };

        // Build final URL from base + enabled params
        let enabled_params: Vec<_> = payload
            .params
            .iter()
            .filter(|p| p.enabled && !p.key.is_empty())
            .collect();
        let mut final_url = base_url;
        if !enabled_params.is_empty() {
            let parts: Vec<String> = enabled_params
                .iter()
                .map(|p| {
                    let (ik, u1) = interpolate(&p.key, var_map);
                    let (iv, u2) = interpolate(&p.value, var_map);
                    all_unresolved.extend(u1);
                    all_unresolved.extend(u2);
                    format!(
                        "{}={}",
                        urlencoding::encode(&ik),
                        urlencoding::encode(&iv)
                    )
                })
                .collect();
            final_url = format!("{}?{}", final_url, parts.join("&"));
        }

        ctx.final_url = final_url;
        ctx.headers = req_headers;
        ctx.body = body;
        ctx.unresolved_vars = all_unresolved;
    }

    async fn apply_auth(&self, ctx: &mut PipelineContext) -> Result<(), ApiError> {
        let (resolved_auth, auth_unresolved) =
            interpolate_auth(&ctx.payload.auth, &ctx.var_map);
        let extra_params = apply_auth(&resolved_auth, &mut ctx.headers).await?;
        if !extra_params.is_empty() {
            let sep = if ctx.final_url.contains('?') {
                "&"
            } else {
                "?"
            };
            let qs: Vec<String> = extra_params
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}={}",
                        urlencoding::encode(k),
                        urlencoding::encode(v)
                    )
                })
                .collect();
            ctx.final_url = format!("{}{}{}", ctx.final_url, sep, qs.join("&"));
        }
        ctx.unresolved_vars.extend(auth_unresolved);

        // Build the ApiRequest now that URL, headers, body, and auth are finalized
        // GraphQL requests are always POST
        let method = if matches!(ctx.payload.request_type, models::RequestType::Graphql) {
            HttpMethod::Post
        } else {
            ctx.payload.method.clone()
        };
        ctx.api_request = ApiRequest {
            method,
            url: ctx.final_url.clone(),
            headers: ctx.headers.clone(),
            body: ctx.body.clone(),
        };

        Ok(())
    }

    async fn load_collection_scripts(&self, ctx: &mut PipelineContext) {
        if let Some(ref coll_id) = ctx.payload.collection_id {
            if let Ok(coll) = self.storage.get_collection(coll_id, self.user_id.as_deref()).await {
                ctx.collection_pre_script = coll.pre_request_script;
                ctx.collection_test_script = coll.test_script;
            }
        }
    }

    async fn run_pre_scripts(&self, ctx: &mut PipelineContext) {
        ctx.pre_result = scripting::execute_pre_request_scripts(
            ctx.collection_pre_script.as_deref(),
            ctx.payload.pre_request_script.as_deref(),
            &mut ctx.api_request,
            &mut ctx.var_map,
        )
        .await;
        // Update final_url after script mutations
        ctx.final_url = ctx.api_request.url.clone();
    }

    async fn send(&self, ctx: &PipelineContext) -> Result<ApiResponse, ApiError> {
        send_request(ctx.api_request.clone()).await
    }

    async fn run_test_scripts(&self, ctx: &mut PipelineContext, response: &ApiResponse) {
        ctx.test_result = scripting::execute_test_scripts(
            ctx.collection_test_script.as_deref(),
            ctx.payload.test_script.as_deref(),
            response,
            &ctx.var_map,
        )
        .await;
    }

    async fn apply_extractions(&self, ctx: &mut PipelineContext, response: &ApiResponse) {
        if ctx.payload.extraction_rules.is_empty() {
            return;
        }
        let extracted =
            crate::extraction::apply_extraction_rules(&ctx.payload.extraction_rules, response);
        // Persist successfully extracted values as environment variables
        for var in &extracted {
            if let Some(ref value) = var.value {
                let variable = models::Variable {
                    id: String::new(),
                    environment_id: ctx.payload.environment_id.clone(),
                    key: var.variable_name.clone(),
                    value: value.clone(),
                    is_secret: false,
                };
                let _ = self.storage.set_variable(&variable, self.user_id.as_deref()).await;
            }
        }
        ctx.extracted_variables = extracted;
    }

    async fn persist_env_updates(&self, ctx: &PipelineContext) {
        for (key, value) in ctx
            .pre_result
            .environment_updates
            .iter()
            .chain(ctx.test_result.environment_updates.iter())
        {
            let var = models::Variable {
                id: String::new(),
                environment_id: ctx.payload.environment_id.clone(),
                key: key.clone(),
                value: value.clone(),
                is_secret: false,
            };
            let _ = self.storage.set_variable(&var, self.user_id.as_deref()).await;
        }
    }

    async fn record_history(
        &self,
        ctx: &PipelineContext,
        resp: &ApiResponse,
    ) -> Result<models::HistoryEntry, ApiError> {
        let mut masked_headers = ctx.headers.clone();
        if let Some(auth_val) = masked_headers.get("Authorization") {
            masked_headers.insert("Authorization".into(), mask_auth_header(auth_val));
        }

        let history_entry = models::HistoryEntry {
            id: String::new(),
            method: ctx.payload.method.clone(),
            url: ctx.final_url.clone(),
            request_headers: serde_json::to_string(&masked_headers).unwrap_or_default(),
            request_body: ctx.body.clone(),
            response_status: resp.status,
            response_headers: serde_json::to_string(&resp.headers).unwrap_or_default(),
            response_body: resp.body.clone(),
            duration_ms: resp.duration_ms,
            timestamp: String::new(),
        };
        let saved_entry = self
            .storage
            .create_history_entry(&history_entry, self.user_id.as_deref())
            .await?;

        // Auto-prune: keep max 1000 entries, delete entries older than 30 days (every 100th request)
        if PRUNE_COUNTER.fetch_add(1, Ordering::Relaxed) % 100 == 0 {
            let _ = self.storage.prune_history(1000, 30).await;
        }

        Ok(saved_entry)
    }

    fn build_response(
        &self,
        mut ctx: PipelineContext,
        response: ApiResponse,
        history_entry: models::HistoryEntry,
    ) -> SendRequestResponse {
        let has_script_data = !ctx.pre_result.console.is_empty()
            || ctx.pre_result.error.is_some()
            || !ctx.test_result.test_results.is_empty()
            || !ctx.test_result.console.is_empty()
            || ctx.test_result.error.is_some();

        let script_result = if has_script_data {
            Some(ScriptExecutionResult {
                pre_request_console: std::mem::take(&mut ctx.pre_result.console),
                pre_request_error: ctx.pre_result.error.take(),
                test_results: std::mem::take(&mut ctx.test_result.test_results),
                test_console: std::mem::take(&mut ctx.test_result.console),
                test_error: ctx.test_result.error.take(),
            })
        } else {
            None
        };

        ctx.unresolved_vars.sort();
        ctx.unresolved_vars.dedup();

        SendRequestResponse {
            response,
            unresolved_variables: ctx.unresolved_vars,
            history_id: history_entry.id,
            script_result,
            extracted_variables: ctx.extracted_variables,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use crate::storage::{StorageBackend, StorageError};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

    // ── Tracking structs ──────────────────────────────────────────────

    #[derive(Debug, Clone)]
    struct SetVariableCall {
        environment_id: Option<String>,
        key: String,
        value: String,
    }

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct HistoryCall {
        method: HttpMethod,
        url: String,
        request_headers: String,
        request_body: Option<String>,
        response_status: u16,
    }

    #[derive(Debug, Clone)]
    struct PruneCall {
        max_entries: u32,
        max_age_days: u32,
    }

    #[derive(Debug, Default)]
    struct MockStorageInner {
        // Configurable return values
        variables: Vec<Variable>,
        collection: Option<Collection>,
        // Call tracking
        set_variable_calls: Vec<SetVariableCall>,
        history_calls: Vec<HistoryCall>,
        prune_calls: Vec<PruneCall>,
        get_collection_calls: Vec<String>,
        list_variables_calls: Vec<Option<String>>,
    }

    #[derive(Clone)]
    struct MockStorage {
        inner: Arc<Mutex<MockStorageInner>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                inner: Arc::new(Mutex::new(MockStorageInner::default())),
            }
        }

        fn with_variables(self, vars: Vec<Variable>) -> Self {
            self.inner.lock().unwrap().variables = vars;
            self
        }

        fn with_collection(self, coll: Collection) -> Self {
            self.inner.lock().unwrap().collection = Some(coll);
            self
        }

        fn set_variable_calls(&self) -> Vec<SetVariableCall> {
            self.inner.lock().unwrap().set_variable_calls.clone()
        }

        fn history_calls(&self) -> Vec<HistoryCall> {
            self.inner.lock().unwrap().history_calls.clone()
        }

        fn prune_calls(&self) -> Vec<PruneCall> {
            self.inner.lock().unwrap().prune_calls.clone()
        }

        fn get_collection_calls(&self) -> Vec<String> {
            self.inner.lock().unwrap().get_collection_calls.clone()
        }
    }

    impl StorageBackend for MockStorage {
        fn list_collections(&self, _user_id: Option<&str>) -> BoxFuture<'_, Result<Vec<Collection>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }

        fn get_collection(&self, id: &str, _user_id: Option<&str>) -> BoxFuture<'_, Result<Collection, StorageError>> {
            let id = id.to_string();
            Box::pin(async move {
                let mut inner = self.inner.lock().unwrap();
                inner.get_collection_calls.push(id.clone());
                inner
                    .collection
                    .clone()
                    .ok_or_else(|| StorageError::NotFound(format!("Collection {id}")))
            })
        }

        fn create_collection(
            &self,
            _name: &str,
            _description: Option<&str>,
            _auth: Option<&AuthConfig>,
            _pre_request_script: Option<&str>,
            _test_script: Option<&str>,
            _user_id: Option<&str>,
            _workspace_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Collection, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn update_collection(
            &self,
            _id: &str,
            _name: &str,
            _description: Option<&str>,
            _auth: Option<&AuthConfig>,
            _pre_request_script: Option<&str>,
            _test_script: Option<&str>,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Collection, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn delete_collection(&self, _id: &str, _user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }

        fn list_folders(
            &self,
            _collection_id: &str,
            _parent_folder_id: Option<&str>,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Vec<Folder>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }

        fn create_folder(
            &self,
            _collection_id: &str,
            _parent_folder_id: Option<&str>,
            _name: &str,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Folder, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn update_folder(
            &self,
            _id: &str,
            _name: &str,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Folder, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn delete_folder(&self, _id: &str, _user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }

        fn list_requests(
            &self,
            _collection_id: &str,
            _folder_id: Option<&str>,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Vec<SavedRequest>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }

        fn get_request(&self, _id: &str, _user_id: Option<&str>) -> BoxFuture<'_, Result<SavedRequest, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn create_request(
            &self,
            _req: &SavedRequest,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<SavedRequest, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn update_request(
            &self,
            _req: &SavedRequest,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<SavedRequest, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn delete_request(&self, _id: &str, _user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }

        fn list_history(
            &self,
            _limit: u32,
            _offset: u32,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Vec<HistoryEntry>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }

        fn create_history_entry(
            &self,
            entry: &HistoryEntry,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<HistoryEntry, StorageError>> {
            let mut saved = entry.clone();
            saved.id = "hist-001".to_string();
            saved.timestamp = "2026-01-01T00:00:00Z".to_string();
            let call = HistoryCall {
                method: entry.method.clone(),
                url: entry.url.clone(),
                request_headers: entry.request_headers.clone(),
                request_body: entry.request_body.clone(),
                response_status: entry.response_status,
            };
            self.inner.lock().unwrap().history_calls.push(call);
            Box::pin(async move { Ok(saved) })
        }

        fn delete_history_entry(&self, _id: &str, _user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }

        fn clear_history(&self, _user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }

        fn prune_history(
            &self,
            max_entries: u32,
            max_age_days: u32,
        ) -> BoxFuture<'_, Result<u64, StorageError>> {
            self.inner.lock().unwrap().prune_calls.push(PruneCall {
                max_entries,
                max_age_days,
            });
            Box::pin(async { Ok(0) })
        }

        fn list_environments(&self, _user_id: Option<&str>) -> BoxFuture<'_, Result<Vec<Environment>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }

        fn get_environment(
            &self,
            _id: &str,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Environment, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn create_environment(
            &self,
            _name: &str,
            _user_id: Option<&str>,
            _workspace_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Environment, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn update_environment(
            &self,
            _id: &str,
            _name: &str,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Environment, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn delete_environment(&self, _id: &str, _user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }

        fn list_variables(
            &self,
            environment_id: Option<&str>,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Vec<Variable>, StorageError>> {
            let env_id = environment_id.map(|s| s.to_string());
            Box::pin(async move {
                let mut inner = self.inner.lock().unwrap();
                inner.list_variables_calls.push(env_id.clone());
                let filtered: Vec<Variable> = inner
                    .variables
                    .iter()
                    .filter(|v| v.environment_id == env_id)
                    .cloned()
                    .collect();
                Ok(filtered)
            })
        }

        fn set_variable(
            &self,
            variable: &Variable,
            _user_id: Option<&str>,
        ) -> BoxFuture<'_, Result<Variable, StorageError>> {
            let call = SetVariableCall {
                environment_id: variable.environment_id.clone(),
                key: variable.key.clone(),
                value: variable.value.clone(),
            };
            self.inner.lock().unwrap().set_variable_calls.push(call);
            let mut saved = variable.clone();
            saved.id = "var-saved".to_string();
            Box::pin(async move { Ok(saved) })
        }

        fn delete_variable(&self, _id: &str, _user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }

        fn create_ws_session(
            &self,
            _session: &WsSession,
        ) -> BoxFuture<'_, Result<WsSession, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }

        fn list_ws_sessions(
            &self,
            _limit: u32,
            _offset: u32,
        ) -> BoxFuture<'_, Result<Vec<WsSession>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }

        fn delete_ws_session(&self, _id: &str) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }

        fn cleanup_orphaned_ws_sessions(&self) -> BoxFuture<'_, Result<u64, StorageError>> {
            Box::pin(async { Ok(0) })
        }

        fn create_user(&self, _email: &str, _password_hash: &str, _display_name: Option<&str>) -> BoxFuture<'_, Result<crate::models::User, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn get_user_by_email(&self, _email: &str) -> BoxFuture<'_, Result<crate::models::UserWithHash, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn get_user_by_id(&self, _id: &str) -> BoxFuture<'_, Result<crate::models::User, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn list_all_users(&self) -> BoxFuture<'_, Result<Vec<crate::models::User>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn store_refresh_token(&self, _user_id: &str, _token_hash: &str, _expires_at: &str) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }
        fn get_refresh_token(&self, _token_hash: &str) -> BoxFuture<'_, Result<crate::models::RefreshToken, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn delete_refresh_token(&self, _token_hash: &str) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }
        fn delete_user_refresh_tokens(&self, _user_id: &str) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }
        fn list_connections(&self) -> BoxFuture<'_, Result<Vec<crate::models::ServerConnection>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn get_connection(&self, _id: &str) -> BoxFuture<'_, Result<crate::models::ServerConnection, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn create_connection(&self, _connection: &crate::models::ServerConnection) -> BoxFuture<'_, Result<crate::models::ServerConnection, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn update_connection_tokens(&self, _id: &str, _access_token: Option<&str>, _refresh_token: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }
        fn update_connection_status(&self, _id: &str, _status: &str) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }
        fn delete_connection(&self, _id: &str) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }
        fn get_connection_tokens(&self, _id: &str) -> BoxFuture<'_, Result<(Option<String>, Option<String>), StorageError>> {
            Box::pin(async { Ok((None, None)) })
        }

        // Workspaces
        fn create_workspace(&self, _name: &str, _description: Option<&str>, _owner_id: &str, _is_personal: bool) -> BoxFuture<'_, Result<Workspace, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn get_workspace(&self, _id: &str) -> BoxFuture<'_, Result<Workspace, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn update_workspace(&self, _id: &str, _name: &str, _description: Option<&str>) -> BoxFuture<'_, Result<Workspace, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn delete_workspace(&self, _id: &str) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }
        fn list_user_workspaces(&self, _user_id: &str) -> BoxFuture<'_, Result<Vec<Workspace>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn get_personal_workspace(&self, _user_id: &str) -> BoxFuture<'_, Result<Workspace, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn add_workspace_member(&self, _workspace_id: &str, _user_id: &str, _role: &str) -> BoxFuture<'_, Result<WorkspaceMember, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn remove_workspace_member(&self, _workspace_id: &str, _user_id: &str) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }
        fn update_workspace_member_role(&self, _workspace_id: &str, _user_id: &str, _role: &str) -> BoxFuture<'_, Result<WorkspaceMember, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn list_workspace_members(&self, _workspace_id: &str) -> BoxFuture<'_, Result<Vec<WorkspaceMember>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn get_workspace_member(&self, _workspace_id: &str, _user_id: &str) -> BoxFuture<'_, Result<WorkspaceMember, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn create_workspace_invite(&self, _workspace_id: &str, _email: &str, _role: &str, _token: &str, _expires_at: &str) -> BoxFuture<'_, Result<WorkspaceInvite, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn get_workspace_invite_by_token(&self, _token: &str) -> BoxFuture<'_, Result<WorkspaceInvite, StorageError>> {
            Box::pin(async { Err(StorageError::NotFound("not implemented".into())) })
        }
        fn delete_workspace_invite(&self, _id: &str) -> BoxFuture<'_, Result<(), StorageError>> {
            Box::pin(async { Ok(()) })
        }
        fn list_workspace_invites(&self, _workspace_id: &str) -> BoxFuture<'_, Result<Vec<WorkspaceInvite>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn list_workspace_collections(&self, _workspace_id: &str) -> BoxFuture<'_, Result<Vec<Collection>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn list_workspace_environments(&self, _workspace_id: &str) -> BoxFuture<'_, Result<Vec<Environment>, StorageError>> {
            Box::pin(async { Ok(vec![]) })
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────

    fn make_encryption_key() -> [u8; 32] {
        [42u8; 32]
    }

    fn make_payload(url: &str) -> SendRequestPayload {
        SendRequestPayload {
            method: HttpMethod::Get,
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
            params: vec![],
            auth: AuthConfig::None,
            environment_id: None,
            pre_request_script: None,
            test_script: None,
            collection_id: None,
            request_type: RequestType::Http,
            graphql_query: None,
            graphql_variables: None,
            graphql_operation_name: None,
            extraction_rules: vec![],
        }
    }

    fn make_global_var(key: &str, value: &str) -> Variable {
        Variable {
            id: format!("var-{key}"),
            environment_id: None,
            key: key.to_string(),
            value: value.to_string(),
            is_secret: false,
        }
    }

    fn make_env_var(env_id: &str, key: &str, value: &str) -> Variable {
        Variable {
            id: format!("var-{env_id}-{key}"),
            environment_id: Some(env_id.to_string()),
            key: key.to_string(),
            value: value.to_string(),
            is_secret: false,
        }
    }

    fn make_collection(id: &str) -> Collection {
        Collection {
            id: id.to_string(),
            name: "Test Collection".to_string(),
            description: None,
            auth: AuthConfig::None,
            pre_request_script: None,
            test_script: None,
            workspace_id: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_variable_interpolation_in_url() {
        let mock_server = wiremock::MockServer::start().await;
        let host = mock_server.address().to_string();

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/api/users"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new().with_variables(vec![
            make_global_var("base_url", &format!("http://{host}")),
            make_global_var("path", "api/users"),
        ]);
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let payload = make_payload("{{base_url}}/{{path}}");
        let result = pipeline.execute(payload).await.unwrap();

        assert_eq!(result.response.status, 200);
        assert!(result.unresolved_variables.is_empty());

        let history = storage.history_calls();
        assert_eq!(history.len(), 1);
        assert!(history[0].url.contains(&host));
        assert!(history[0].url.contains("/api/users"));
    }

    #[tokio::test]
    async fn test_variable_interpolation_in_headers() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::header("X-Custom", "my-value"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new().with_variables(vec![
            make_global_var("header_val", "my-value"),
        ]);
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.headers.insert("X-Custom".into(), "{{header_val}}".into());

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);
    }

    #[tokio::test]
    async fn test_variable_interpolation_in_body() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::body_string("hello world"))
            .respond_with(wiremock::ResponseTemplate::new(201).set_body_string("created"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new().with_variables(vec![
            make_global_var("greeting", "hello world"),
        ]);
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.method = HttpMethod::Post;
        payload.body = Some("{{greeting}}".to_string());

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 201);
    }

    #[tokio::test]
    async fn test_unresolved_variables_reported() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&format!("{}/{{{{missing_path}}}}", mock_server.uri()));
        payload.headers.insert("X-Key".into(), "{{missing_key}}".into());

        let result = pipeline.execute(payload).await.unwrap();
        assert!(result.unresolved_variables.contains(&"missing_path".to_string()));
        assert!(result.unresolved_variables.contains(&"missing_key".to_string()));
    }

    #[tokio::test]
    async fn test_env_variables_override_globals() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::header("X-Env", "env-value"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new().with_variables(vec![
            make_global_var("val", "global-value"),
            make_env_var("env-1", "val", "env-value"),
        ]);
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.headers.insert("X-Env".into(), "{{val}}".into());
        payload.environment_id = Some("env-1".to_string());

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);
    }

    #[tokio::test]
    async fn test_auth_bearer() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::header("Authorization", "Bearer my-token-123"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("authed"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.auth = AuthConfig::Bearer {
            token: "my-token-123".to_string(),
        };

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);
    }

    #[tokio::test]
    async fn test_auth_basic() {
        use base64::Engine;
        let mock_server = wiremock::MockServer::start().await;
        let expected_creds = base64::engine::general_purpose::STANDARD.encode("admin:secret");

        wiremock::Mock::given(wiremock::matchers::header(
            "Authorization",
            format!("Basic {expected_creds}").as_str(),
        ))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("basic ok"))
        .mount(&mock_server)
        .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.auth = AuthConfig::Basic {
            username: "admin".to_string(),
            password: "secret".to_string(),
        };

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);
    }

    #[tokio::test]
    async fn test_auth_api_key_header() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::header("X-Api-Key", "abc123"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("key ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.auth = AuthConfig::ApiKey {
            key: "X-Api-Key".to_string(),
            value: "abc123".to_string(),
            add_to: ApiKeyLocation::Header,
        };

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);
    }

    #[tokio::test]
    async fn test_auth_api_key_query_param() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::query_param("api_key", "xyz789"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("qp ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.auth = AuthConfig::ApiKey {
            key: "api_key".to_string(),
            value: "xyz789".to_string(),
            add_to: ApiKeyLocation::QueryParam,
        };

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);
    }

    #[tokio::test]
    async fn test_auth_with_variable_interpolation() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::header("Authorization", "Bearer secret-token"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new().with_variables(vec![
            make_global_var("my_token", "secret-token"),
        ]);
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.auth = AuthConfig::Bearer {
            token: "{{my_token}}".to_string(),
        };

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);
    }

    #[tokio::test]
    async fn test_history_recorded() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_string(r#"{"data":"test"}"#),
            )
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&format!("{}/test", mock_server.uri()));
        payload.method = HttpMethod::Post;
        payload.body = Some("request body".to_string());

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.history_id, "hist-001");

        let history = storage.history_calls();
        assert_eq!(history.len(), 1);
        assert!(history[0].url.contains("/test"));
        assert_eq!(history[0].request_body.as_deref(), Some("request body"));
        assert_eq!(history[0].response_status, 200);
    }

    #[tokio::test]
    async fn test_history_masks_auth_header() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.auth = AuthConfig::Bearer {
            token: "super-secret-token-value".to_string(),
        };

        pipeline.execute(payload).await.unwrap();

        let history = storage.history_calls();
        assert_eq!(history.len(), 1);

        let headers: HashMap<String, String> =
            serde_json::from_str(&history[0].request_headers).unwrap();
        let auth_val = headers.get("Authorization").unwrap();
        // mask_auth_header keeps first 4 chars + "****"
        assert!(auth_val.contains("****"));
        assert!(!auth_val.contains("super-secret-token-value"));
    }

    #[tokio::test]
    async fn test_query_params_appended() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::query_param("page", "1"))
            .and(wiremock::matchers::query_param("limit", "10"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.params = vec![
            KeyValueParam {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
            },
            KeyValueParam {
                key: "limit".into(),
                value: "10".into(),
                enabled: true,
            },
            KeyValueParam {
                key: "ignored".into(),
                value: "val".into(),
                enabled: false,
            },
        ];

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);
    }

    #[tokio::test]
    async fn test_disabled_and_empty_params_skipped() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.params = vec![
            KeyValueParam {
                key: "".into(),
                value: "empty-key".into(),
                enabled: true,
            },
            KeyValueParam {
                key: "disabled".into(),
                value: "val".into(),
                enabled: false,
            },
        ];

        let result = pipeline.execute(payload).await.unwrap();
        let history = storage.history_calls();
        // URL should not have query params since both are filtered out
        assert!(!history[0].url.contains('?'));
        assert_eq!(result.response.status, 200);
    }

    #[tokio::test]
    async fn test_collection_scripts_loaded() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let mut coll = make_collection("coll-1");
        coll.pre_request_script = Some("// pre script".to_string());
        coll.test_script = Some("// test script".to_string());

        let storage = MockStorage::new().with_collection(coll);
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.collection_id = Some("coll-1".to_string());

        let _result = pipeline.execute(payload).await.unwrap();

        let coll_calls = storage.get_collection_calls();
        assert_eq!(coll_calls.len(), 1);
        assert_eq!(coll_calls[0], "coll-1");
    }

    #[tokio::test]
    async fn test_no_collection_scripts_without_collection_id() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new().with_collection(make_collection("coll-1"));
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let payload = make_payload(&mock_server.uri());
        // No collection_id set
        let _result = pipeline.execute(payload).await.unwrap();

        let coll_calls = storage.get_collection_calls();
        assert!(coll_calls.is_empty());
    }

    #[tokio::test]
    async fn test_graphql_request_uses_post_and_json_body() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::header("Content-Type", "application/json"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_string(r#"{"data":{"user":{"name":"test"}}}"#),
            )
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.request_type = RequestType::Graphql;
        payload.graphql_query = Some("{ user { name } }".to_string());
        payload.graphql_variables = Some(r#"{"id": 1}"#.to_string());
        payload.graphql_operation_name = Some("GetUser".to_string());

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);

        let history = storage.history_calls();
        assert_eq!(history.len(), 1);
        let body: serde_json::Value =
            serde_json::from_str(history[0].request_body.as_deref().unwrap()).unwrap();
        assert_eq!(body["query"], "{ user { name } }");
        assert_eq!(body["variables"]["id"], 1);
        assert_eq!(body["operationName"], "GetUser");
    }

    #[tokio::test]
    async fn test_graphql_variables_interpolated() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(r#"{"data":{}}"#))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new().with_variables(vec![
            make_global_var("user_id", "42"),
        ]);
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.request_type = RequestType::Graphql;
        payload.graphql_query = Some("query { user(id: $id) { name } }".to_string());
        payload.graphql_variables = Some(r#"{"id": {{user_id}}}"#.to_string());

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);

        let history = storage.history_calls();
        let body: serde_json::Value =
            serde_json::from_str(history[0].request_body.as_deref().unwrap()).unwrap();
        assert_eq!(body["variables"]["id"], 42);
    }

    #[tokio::test]
    async fn test_no_script_result_when_no_scripts() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let payload = make_payload(&mock_server.uri());
        let result = pipeline.execute(payload).await.unwrap();

        assert!(result.script_result.is_none());
    }

    #[tokio::test]
    async fn test_pre_request_script_environment_updates_persisted() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.environment_id = Some("env-1".to_string());
        // This script sets an environment variable
        payload.pre_request_script = Some(
            r#"pm.environment.set("result_key", "result_value");"#.to_string(),
        );

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);

        let set_calls = storage.set_variable_calls();
        let env_update = set_calls
            .iter()
            .find(|c| c.key == "result_key");
        assert!(env_update.is_some(), "set_variable should be called for env update");
        let call = env_update.unwrap();
        assert_eq!(call.value, "result_value");
        assert_eq!(call.environment_id.as_deref(), Some("env-1"));
    }

    #[tokio::test]
    async fn test_test_script_environment_updates_persisted() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.environment_id = Some("env-2".to_string());
        payload.test_script = Some(
            r#"pm.environment.set("test_key", "test_value");"#.to_string(),
        );

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);

        let set_calls = storage.set_variable_calls();
        let env_update = set_calls.iter().find(|c| c.key == "test_key");
        assert!(env_update.is_some(), "set_variable should be called for test env update");
        let call = env_update.unwrap();
        assert_eq!(call.value, "test_value");
        assert_eq!(call.environment_id.as_deref(), Some("env-2"));
    }

    #[tokio::test]
    async fn test_test_script_results_returned() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.test_script = Some(
            r#"
            pm.test("status is 200", function() {
                pm.expect(pm.response.status).to.equal(200);
            });
            "#
            .to_string(),
        );

        let result = pipeline.execute(payload).await.unwrap();
        let script_result = result.script_result.expect("should have script results");
        assert!(!script_result.test_results.is_empty());
        assert_eq!(script_result.test_results[0].name, "status is 200");
        assert!(script_result.test_results[0].passed);
    }

    #[tokio::test]
    async fn test_history_pruning_runs_periodically() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .expect(100)
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        // Send 100 requests. Since PRUNE_COUNTER is global and increments by 1
        // per request, at least one of these will land on a multiple of 100.
        for _ in 0..100 {
            let payload = make_payload(&mock_server.uri());
            pipeline.execute(payload).await.unwrap();
        }

        let prune_calls = storage.prune_calls();
        assert!(
            !prune_calls.is_empty(),
            "prune_history should be called at least once in 100 requests"
        );
        // Verify the arguments are correct
        assert_eq!(prune_calls[0].max_entries, 1000);
        assert_eq!(prune_calls[0].max_age_days, 30);
    }

    #[tokio::test]
    async fn test_params_with_variable_interpolation() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::query_param("env", "production"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new().with_variables(vec![
            make_global_var("env_name", "production"),
        ]);
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.params = vec![KeyValueParam {
            key: "env".into(),
            value: "{{env_name}}".into(),
            enabled: true,
        }];

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);
    }

    #[tokio::test]
    async fn test_multiple_http_methods() {
        let mock_server = wiremock::MockServer::start().await;

        for (method_str, method) in [
            ("PUT", HttpMethod::Put),
            ("PATCH", HttpMethod::Patch),
            ("DELETE", HttpMethod::Delete),
        ] {
            wiremock::Mock::given(wiremock::matchers::method(method_str))
                .respond_with(
                    wiremock::ResponseTemplate::new(200)
                        .set_body_string(format!("{method_str} ok")),
                )
                .mount(&mock_server)
                .await;

            let storage = MockStorage::new();
            let key = make_encryption_key();
            let pipeline = RequestPipeline::new(&storage, &key);

            let mut payload = make_payload(&mock_server.uri());
            payload.method = method;

            let result = pipeline.execute(payload).await.unwrap();
            assert_eq!(result.response.status, 200, "Failed for {method_str}");
        }
    }

    #[tokio::test]
    async fn test_response_body_and_status_captured() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(
                wiremock::ResponseTemplate::new(404)
                    .set_body_string("not found")
                    .append_header("X-Custom", "test-header"),
            )
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let payload = make_payload(&mock_server.uri());
        let result = pipeline.execute(payload).await.unwrap();

        assert_eq!(result.response.status, 404);
        assert_eq!(result.response.body, "not found");
        assert_eq!(
            result.response.headers.get("x-custom").map(|s| s.as_str()),
            Some("test-header")
        );
    }

    #[tokio::test]
    async fn test_pre_request_script_console_output() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.pre_request_script = Some(r#"console.log("hello from pre-script");"#.to_string());

        let result = pipeline.execute(payload).await.unwrap();
        let script_result = result.script_result.expect("should have script result");
        assert!(
            script_result
                .pre_request_console
                .iter()
                .any(|e| e.message.contains("hello from pre-script")),
            "pre-request console should contain log output"
        );
    }

    #[tokio::test]
    async fn test_api_key_query_param_appended_to_existing_params() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::query_param("page", "1"))
            .and(wiremock::matchers::query_param("token", "abc"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let storage = MockStorage::new();
        let key = make_encryption_key();
        let pipeline = RequestPipeline::new(&storage, &key);

        let mut payload = make_payload(&mock_server.uri());
        payload.params = vec![KeyValueParam {
            key: "page".into(),
            value: "1".into(),
            enabled: true,
        }];
        payload.auth = AuthConfig::ApiKey {
            key: "token".to_string(),
            value: "abc".to_string(),
            add_to: ApiKeyLocation::QueryParam,
        };

        let result = pipeline.execute(payload).await.unwrap();
        assert_eq!(result.response.status, 200);

        // Verify URL in history has both params
        let history = storage.history_calls();
        assert!(history[0].url.contains("page=1"));
        assert!(history[0].url.contains("token=abc"));
    }
}
