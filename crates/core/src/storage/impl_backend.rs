/// Generate `StorageBackend` implementation for a database backend.
///
/// All SQL is written with Postgres-style `$N` placeholders; the `common::sql()`
/// helper rewrites them to `?` for SQLite at runtime.
///
/// Dialect-specific differences (is_secret type, integer casts) are handled
/// via internal macro arms (`@read_secret`, `@bind_secret_exec`).
macro_rules! impl_storage_backend {
    ($backend:ty, $is_pg:expr) => {
        impl StorageBackend for $backend {

    // ── Collections ──────────────────────────────────────────────

    fn list_collections(&self, user_id: Option<&str>) -> super::BoxFuture<'_, Result<Vec<Collection>, StorageError>> {
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let rows = match &user_id {
                Some(uid) => {
                    let __sql = common::sql(
                        "SELECT id, name, description, auth, pre_request_script, test_script, workspace_id, created_at, updated_at FROM collections WHERE user_id = $1 ORDER BY name",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(uid).fetch_all(&self.pool).await?
                }
                None => {
                    let __sql = common::sql(
                        "SELECT id, name, description, auth, pre_request_script, test_script, workspace_id, created_at, updated_at FROM collections WHERE (user_id IS NULL) ORDER BY name",
                        $is_pg,
                    );
                    sqlx::query(&__sql).fetch_all(&self.pool).await?
                }
            };
            Ok(rows.iter().map(|r| common::row_to_collection(r, &self.encryption_key)).collect())
        })
    }

    fn get_collection(&self, id: &str, user_id: Option<&str>) -> super::BoxFuture<'_, Result<Collection, StorageError>> {
        let id = id.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let row = match &user_id {
                Some(uid) => {
                    let __sql = common::sql(
                        "SELECT id, name, description, auth, pre_request_script, test_script, workspace_id, created_at, updated_at FROM collections WHERE id = $1 AND user_id = $2",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&id).bind(uid).fetch_one(&self.pool).await?
                }
                None => {
                    let __sql = common::sql(
                        "SELECT id, name, description, auth, pre_request_script, test_script, workspace_id, created_at, updated_at FROM collections WHERE id = $1 AND (user_id IS NULL)",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&id).fetch_one(&self.pool).await?
                }
            };
            Ok(common::row_to_collection(&row, &self.encryption_key))
        })
    }

    fn create_collection(
        &self,
        name: &str,
        description: Option<&str>,
        auth: Option<&AuthConfig>,
        pre_request_script: Option<&str>,
        test_script: Option<&str>,
        user_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Collection, StorageError>> {
        let name = name.to_string();
        let description = description.map(|s| s.to_string());
        let auth_config = auth.cloned().unwrap_or_default();
        let auth_json = serialize_auth(&auth_config, &self.encryption_key);
        let pre_request_script = pre_request_script.map(|s| s.to_string());
        let test_script = test_script.map(|s| s.to_string());
        let user_id = user_id.map(|s| s.to_string());
        let workspace_id = workspace_id.map(|s| s.to_string());
        Box::pin(async move {
            let id = new_id();
            let now = now_iso();
            let __sql = common::sql(
                "INSERT INTO collections (id, name, description, auth, pre_request_script, test_script, user_id, workspace_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&id)
                .bind(&name)
                .bind(&description)
                .bind(&auth_json)
                .bind(&pre_request_script)
                .bind(&test_script)
                .bind(&user_id)
                .bind(&workspace_id)
                .bind(&now)
                .bind(&now)
                .execute(&self.pool)
                .await?;
            Ok(Collection {
                id,
                name,
                description,
                auth: auth_config,
                pre_request_script,
                test_script,
                workspace_id,
                created_at: now.clone(),
                updated_at: now,
            })
        })
    }

    fn update_collection(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        auth: Option<&AuthConfig>,
        pre_request_script: Option<&str>,
        test_script: Option<&str>,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Collection, StorageError>> {
        let id = id.to_string();
        let name = name.to_string();
        let description = description.map(|s| s.to_string());
        let auth_json = auth.map(|a| serialize_auth(a, &self.encryption_key));
        let pre_request_script = pre_request_script.map(|s| s.to_string());
        let test_script = test_script.map(|s| s.to_string());
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let now = now_iso();
            match (&auth_json, &user_id) {
                (Some(auth_json), Some(uid)) => {
                    let __sql = common::sql(
                        "UPDATE collections SET name = $1, description = $2, auth = $3, pre_request_script = $4, test_script = $5, updated_at = $6 WHERE id = $7 AND user_id = $8",
                        $is_pg,
                    );
                    let result = sqlx::query(&__sql)
                        .bind(&name)
                        .bind(&description)
                        .bind(auth_json)
                        .bind(&pre_request_script)
                        .bind(&test_script)
                        .bind(&now)
                        .bind(&id)
                        .bind(uid)
                        .execute(&self.pool)
                        .await?;
                    if result.rows_affected() == 0 {
                        return Err(StorageError::NotFound(format!("Collection {id} not found")));
                    }
                }
                (Some(auth_json), None) => {
                    let __sql = common::sql(
                        "UPDATE collections SET name = $1, description = $2, auth = $3, pre_request_script = $4, test_script = $5, updated_at = $6 WHERE id = $7 AND (user_id IS NULL)",
                        $is_pg,
                    );
                    let result = sqlx::query(&__sql)
                        .bind(&name)
                        .bind(&description)
                        .bind(auth_json)
                        .bind(&pre_request_script)
                        .bind(&test_script)
                        .bind(&now)
                        .bind(&id)
                        .execute(&self.pool)
                        .await?;
                    if result.rows_affected() == 0 {
                        return Err(StorageError::NotFound(format!("Collection {id} not found")));
                    }
                }
                (None, Some(uid)) => {
                    let __sql = common::sql(
                        "UPDATE collections SET name = $1, description = $2, pre_request_script = $3, test_script = $4, updated_at = $5 WHERE id = $6 AND user_id = $7",
                        $is_pg,
                    );
                    let result = sqlx::query(&__sql)
                        .bind(&name)
                        .bind(&description)
                        .bind(&pre_request_script)
                        .bind(&test_script)
                        .bind(&now)
                        .bind(&id)
                        .bind(uid)
                        .execute(&self.pool)
                        .await?;
                    if result.rows_affected() == 0 {
                        return Err(StorageError::NotFound(format!("Collection {id} not found")));
                    }
                }
                (None, None) => {
                    let __sql = common::sql(
                        "UPDATE collections SET name = $1, description = $2, pre_request_script = $3, test_script = $4, updated_at = $5 WHERE id = $6 AND (user_id IS NULL)",
                        $is_pg,
                    );
                    let result = sqlx::query(&__sql)
                        .bind(&name)
                        .bind(&description)
                        .bind(&pre_request_script)
                        .bind(&test_script)
                        .bind(&now)
                        .bind(&id)
                        .execute(&self.pool)
                        .await?;
                    if result.rows_affected() == 0 {
                        return Err(StorageError::NotFound(format!("Collection {id} not found")));
                    }
                }
            }
            self.get_collection(&id, user_id.as_deref()).await
        })
    }

    fn delete_collection(&self, id: &str, user_id: Option<&str>) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            match &user_id {
                Some(uid) => {
                    let __sql = common::sql("DELETE FROM collections WHERE id = $1 AND user_id = $2", $is_pg);
                    sqlx::query(&__sql).bind(&id).bind(uid).execute(&self.pool).await?;
                }
                None => {
                    let __sql = common::sql("DELETE FROM collections WHERE id = $1 AND (user_id IS NULL)", $is_pg);
                    sqlx::query(&__sql).bind(&id).execute(&self.pool).await?;
                }
            }
            Ok(())
        })
    }

    // ── Folders ──────────────────────────────────────────────────

    fn list_folders(
        &self,
        collection_id: &str,
        parent_folder_id: Option<&str>,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Vec<Folder>, StorageError>> {
        let collection_id = collection_id.to_string();
        let parent_folder_id = parent_folder_id.map(|s| s.to_string());
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let rows = match (&parent_folder_id, &user_id) {
                (Some(pid), Some(uid)) => {
                    let __sql = common::sql(
                        "SELECT id, collection_id, parent_folder_id, name, sort_order FROM folders WHERE collection_id = $1 AND parent_folder_id = $2 AND user_id = $3 ORDER BY sort_order",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&collection_id).bind(pid).bind(uid).fetch_all(&self.pool).await?
                }
                (Some(pid), None) => {
                    let __sql = common::sql(
                        "SELECT id, collection_id, parent_folder_id, name, sort_order FROM folders WHERE collection_id = $1 AND parent_folder_id = $2 AND (user_id IS NULL) ORDER BY sort_order",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&collection_id).bind(pid).fetch_all(&self.pool).await?
                }
                (None, Some(uid)) => {
                    let __sql = common::sql(
                        "SELECT id, collection_id, parent_folder_id, name, sort_order FROM folders WHERE collection_id = $1 AND parent_folder_id IS NULL AND user_id = $2 ORDER BY sort_order",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&collection_id).bind(uid).fetch_all(&self.pool).await?
                }
                (None, None) => {
                    let __sql = common::sql(
                        "SELECT id, collection_id, parent_folder_id, name, sort_order FROM folders WHERE collection_id = $1 AND parent_folder_id IS NULL AND (user_id IS NULL) ORDER BY sort_order",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&collection_id).fetch_all(&self.pool).await?
                }
            };
            Ok(rows.iter().map(common::row_to_folder).collect())
        })
    }

    fn create_folder(
        &self,
        collection_id: &str,
        parent_folder_id: Option<&str>,
        name: &str,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Folder, StorageError>> {
        let collection_id = collection_id.to_string();
        let parent_folder_id = parent_folder_id.map(|s| s.to_string());
        let name = name.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let id = new_id();
            let __sql = common::sql(
                "INSERT INTO folders (id, collection_id, parent_folder_id, name, sort_order, user_id) VALUES ($1, $2, $3, $4, 0, $5)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&id)
                .bind(&collection_id)
                .bind(&parent_folder_id)
                .bind(&name)
                .bind(&user_id)
                .execute(&self.pool)
                .await?;
            Ok(Folder {
                id,
                collection_id,
                parent_folder_id,
                name,
                sort_order: 0,
            })
        })
    }

    fn update_folder(
        &self,
        id: &str,
        name: &str,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Folder, StorageError>> {
        let id = id.to_string();
        let name = name.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            match &user_id {
                Some(uid) => {
                    let __sql = common::sql("UPDATE folders SET name = $1 WHERE id = $2 AND user_id = $3", $is_pg);
                    let result = sqlx::query(&__sql).bind(&name).bind(&id).bind(uid).execute(&self.pool).await?;
                    if result.rows_affected() == 0 {
                        return Err(StorageError::NotFound(format!("Folder {id} not found")));
                    }
                }
                None => {
                    let __sql = common::sql("UPDATE folders SET name = $1 WHERE id = $2 AND (user_id IS NULL)", $is_pg);
                    let result = sqlx::query(&__sql).bind(&name).bind(&id).execute(&self.pool).await?;
                    if result.rows_affected() == 0 {
                        return Err(StorageError::NotFound(format!("Folder {id} not found")));
                    }
                }
            }
            // Re-query to return updated folder
            let row = match &user_id {
                Some(uid) => {
                    let __sql2 = common::sql(
                        "SELECT id, collection_id, parent_folder_id, name, sort_order FROM folders WHERE id = $1 AND user_id = $2",
                        $is_pg,
                    );
                    sqlx::query(&__sql2).bind(&id).bind(uid).fetch_one(&self.pool).await?
                }
                None => {
                    let __sql2 = common::sql(
                        "SELECT id, collection_id, parent_folder_id, name, sort_order FROM folders WHERE id = $1 AND (user_id IS NULL)",
                        $is_pg,
                    );
                    sqlx::query(&__sql2).bind(&id).fetch_one(&self.pool).await?
                }
            };
            Ok(common::row_to_folder(&row))
        })
    }

    fn delete_folder(&self, id: &str, user_id: Option<&str>) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            match &user_id {
                Some(uid) => {
                    let __sql = common::sql("DELETE FROM folders WHERE id = $1 AND user_id = $2", $is_pg);
                    sqlx::query(&__sql).bind(&id).bind(uid).execute(&self.pool).await?;
                }
                None => {
                    let __sql = common::sql("DELETE FROM folders WHERE id = $1 AND (user_id IS NULL)", $is_pg);
                    sqlx::query(&__sql).bind(&id).execute(&self.pool).await?;
                }
            }
            Ok(())
        })
    }

    // ── Saved Requests ───────────────────────────────────────────

    fn list_requests(
        &self,
        collection_id: &str,
        folder_id: Option<&str>,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Vec<SavedRequest>, StorageError>> {
        let collection_id = collection_id.to_string();
        let folder_id = folder_id.map(|s| s.to_string());
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let rows = match (&folder_id, &user_id) {
                (Some(fid), Some(uid)) => {
                    let __sql = common::sql(
                        "SELECT id, collection_id, folder_id, name, method, url, headers, body, params, auth, pre_request_script, test_script, request_type, graphql_query, graphql_variables, graphql_operation_name, extraction_rules, sort_order, created_at, updated_at FROM saved_requests WHERE collection_id = $1 AND folder_id = $2 AND user_id = $3 ORDER BY sort_order",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&collection_id).bind(fid).bind(uid).fetch_all(&self.pool).await?
                }
                (Some(fid), None) => {
                    let __sql = common::sql(
                        "SELECT id, collection_id, folder_id, name, method, url, headers, body, params, auth, pre_request_script, test_script, request_type, graphql_query, graphql_variables, graphql_operation_name, extraction_rules, sort_order, created_at, updated_at FROM saved_requests WHERE collection_id = $1 AND folder_id = $2 AND (user_id IS NULL) ORDER BY sort_order",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&collection_id).bind(fid).fetch_all(&self.pool).await?
                }
                (None, Some(uid)) => {
                    let __sql = common::sql(
                        "SELECT id, collection_id, folder_id, name, method, url, headers, body, params, auth, pre_request_script, test_script, request_type, graphql_query, graphql_variables, graphql_operation_name, extraction_rules, sort_order, created_at, updated_at FROM saved_requests WHERE collection_id = $1 AND folder_id IS NULL AND user_id = $2 ORDER BY sort_order",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&collection_id).bind(uid).fetch_all(&self.pool).await?
                }
                (None, None) => {
                    let __sql = common::sql(
                        "SELECT id, collection_id, folder_id, name, method, url, headers, body, params, auth, pre_request_script, test_script, request_type, graphql_query, graphql_variables, graphql_operation_name, extraction_rules, sort_order, created_at, updated_at FROM saved_requests WHERE collection_id = $1 AND folder_id IS NULL AND (user_id IS NULL) ORDER BY sort_order",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&collection_id).fetch_all(&self.pool).await?
                }
            };
            Ok(rows.iter().map(|r| common::row_to_saved_request(r, &self.encryption_key)).collect())
        })
    }

    fn get_request(&self, id: &str, user_id: Option<&str>) -> super::BoxFuture<'_, Result<SavedRequest, StorageError>> {
        let id = id.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let r = match &user_id {
                Some(uid) => {
                    let __sql = common::sql(
                        "SELECT id, collection_id, folder_id, name, method, url, headers, body, params, auth, pre_request_script, test_script, request_type, graphql_query, graphql_variables, graphql_operation_name, extraction_rules, sort_order, created_at, updated_at FROM saved_requests WHERE id = $1 AND user_id = $2",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&id).bind(uid).fetch_one(&self.pool).await?
                }
                None => {
                    let __sql = common::sql(
                        "SELECT id, collection_id, folder_id, name, method, url, headers, body, params, auth, pre_request_script, test_script, request_type, graphql_query, graphql_variables, graphql_operation_name, extraction_rules, sort_order, created_at, updated_at FROM saved_requests WHERE id = $1 AND (user_id IS NULL)",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&id).fetch_one(&self.pool).await?
                }
            };
            Ok(common::row_to_saved_request(&r, &self.encryption_key))
        })
    }

    fn create_request(
        &self,
        req: &SavedRequest,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<SavedRequest, StorageError>> {
        let mut req = req.clone();
        let auth_json = serialize_auth(&req.auth, &self.encryption_key);
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            req.id = new_id();
            let now = now_iso();
            req.created_at = now.clone();
            req.updated_at = now;
            let headers_json = serde_json::to_string(&req.headers).unwrap_or_default();
            let params_json = serde_json::to_string(&req.params).unwrap_or_default();
            let __sql = common::sql(
                "INSERT INTO saved_requests (id, collection_id, folder_id, name, method, url, headers, body, params, auth, pre_request_script, test_script, request_type, graphql_query, graphql_variables, graphql_operation_name, extraction_rules, sort_order, created_at, updated_at, user_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&req.id)
                .bind(&req.collection_id)
                .bind(&req.folder_id)
                .bind(&req.name)
                .bind(req.method.to_string())
                .bind(&req.url)
                .bind(&headers_json)
                .bind(&req.body)
                .bind(&params_json)
                .bind(&auth_json)
                .bind(&req.pre_request_script)
                .bind(&req.test_script)
                .bind(req.request_type.to_string())
                .bind(&req.graphql_query)
                .bind(&req.graphql_variables)
                .bind(&req.graphql_operation_name)
                .bind(&req.extraction_rules)
                .bind(req.sort_order)
                .bind(&req.created_at)
                .bind(&req.updated_at)
                .bind(&user_id)
                .execute(&self.pool)
                .await?;
            Ok(req)
        })
    }

    fn update_request(
        &self,
        req: &SavedRequest,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<SavedRequest, StorageError>> {
        let mut req = req.clone();
        let auth_json = serialize_auth(&req.auth, &self.encryption_key);
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            req.updated_at = now_iso();
            let headers_json = serde_json::to_string(&req.headers).unwrap_or_default();
            let params_json = serde_json::to_string(&req.params).unwrap_or_default();
            let result = match &user_id {
                Some(uid) => {
                    let __sql = common::sql(
                        "UPDATE saved_requests SET collection_id = $1, folder_id = $2, name = $3, method = $4, url = $5, headers = $6, body = $7, params = $8, auth = $9, pre_request_script = $10, test_script = $11, request_type = $12, graphql_query = $13, graphql_variables = $14, graphql_operation_name = $15, extraction_rules = $16, sort_order = $17, updated_at = $18 WHERE id = $19 AND user_id = $20",
                        $is_pg,
                    );
                    sqlx::query(&__sql)
                        .bind(&req.collection_id)
                        .bind(&req.folder_id)
                        .bind(&req.name)
                        .bind(req.method.to_string())
                        .bind(&req.url)
                        .bind(&headers_json)
                        .bind(&req.body)
                        .bind(&params_json)
                        .bind(&auth_json)
                        .bind(&req.pre_request_script)
                        .bind(&req.test_script)
                        .bind(req.request_type.to_string())
                        .bind(&req.graphql_query)
                        .bind(&req.graphql_variables)
                        .bind(&req.graphql_operation_name)
                        .bind(&req.extraction_rules)
                        .bind(req.sort_order)
                        .bind(&req.updated_at)
                        .bind(&req.id)
                        .bind(uid)
                        .execute(&self.pool)
                        .await?
                }
                None => {
                    let __sql = common::sql(
                        "UPDATE saved_requests SET collection_id = $1, folder_id = $2, name = $3, method = $4, url = $5, headers = $6, body = $7, params = $8, auth = $9, pre_request_script = $10, test_script = $11, request_type = $12, graphql_query = $13, graphql_variables = $14, graphql_operation_name = $15, extraction_rules = $16, sort_order = $17, updated_at = $18 WHERE id = $19 AND (user_id IS NULL)",
                        $is_pg,
                    );
                    sqlx::query(&__sql)
                        .bind(&req.collection_id)
                        .bind(&req.folder_id)
                        .bind(&req.name)
                        .bind(req.method.to_string())
                        .bind(&req.url)
                        .bind(&headers_json)
                        .bind(&req.body)
                        .bind(&params_json)
                        .bind(&auth_json)
                        .bind(&req.pre_request_script)
                        .bind(&req.test_script)
                        .bind(req.request_type.to_string())
                        .bind(&req.graphql_query)
                        .bind(&req.graphql_variables)
                        .bind(&req.graphql_operation_name)
                        .bind(&req.extraction_rules)
                        .bind(req.sort_order)
                        .bind(&req.updated_at)
                        .bind(&req.id)
                        .execute(&self.pool)
                        .await?
                }
            };
            if result.rows_affected() == 0 {
                return Err(StorageError::NotFound(format!("Request {} not found", req.id)));
            }
            Ok(req)
        })
    }

    fn delete_request(&self, id: &str, user_id: Option<&str>) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            match &user_id {
                Some(uid) => {
                    let __sql = common::sql("DELETE FROM saved_requests WHERE id = $1 AND user_id = $2", $is_pg);
                    sqlx::query(&__sql).bind(&id).bind(uid).execute(&self.pool).await?;
                }
                None => {
                    let __sql = common::sql("DELETE FROM saved_requests WHERE id = $1 AND (user_id IS NULL)", $is_pg);
                    sqlx::query(&__sql).bind(&id).execute(&self.pool).await?;
                }
            }
            Ok(())
        })
    }

    // ── History ──────────────────────────────────────────────────

    fn list_history(
        &self,
        limit: u32,
        offset: u32,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Vec<HistoryEntry>, StorageError>> {
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let rows = match &user_id {
                Some(uid) => {
                    let __sql = common::sql(
                        "SELECT id, method, url, request_headers, request_body, response_status, response_headers, response_body, duration_ms, timestamp FROM history_entries WHERE user_id = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
                        $is_pg,
                    );
                    sqlx::query(&__sql)
                        .bind(uid)
                        .bind(limit as i64)
                        .bind(offset as i64)
                        .fetch_all(&self.pool)
                        .await?
                }
                None => {
                    let __sql = common::sql(
                        "SELECT id, method, url, request_headers, request_body, response_status, response_headers, response_body, duration_ms, timestamp FROM history_entries WHERE (user_id IS NULL) ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
                        $is_pg,
                    );
                    sqlx::query(&__sql)
                        .bind(limit as i64)
                        .bind(offset as i64)
                        .fetch_all(&self.pool)
                        .await?
                }
            };
            Ok(rows.iter().map(common::row_to_history_entry).collect())
        })
    }

    fn create_history_entry(
        &self,
        entry: &HistoryEntry,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<HistoryEntry, StorageError>> {
        let mut entry = entry.clone();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            entry.id = new_id();
            entry.timestamp = now_iso();
            let __sql = common::sql(
                "INSERT INTO history_entries (id, method, url, request_headers, request_body, response_status, response_headers, response_body, duration_ms, timestamp, user_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&entry.id)
                .bind(entry.method.to_string())
                .bind(&entry.url)
                .bind(&entry.request_headers)
                .bind(&entry.request_body)
                .bind(entry.response_status as i32)
                .bind(&entry.response_headers)
                .bind(&entry.response_body)
                .bind(entry.duration_ms as i64)
                .bind(&entry.timestamp)
                .bind(&user_id)
                .execute(&self.pool)
                .await?;
            Ok(entry)
        })
    }

    fn delete_history_entry(&self, id: &str, user_id: Option<&str>) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            match &user_id {
                Some(uid) => {
                    let __sql = common::sql("DELETE FROM history_entries WHERE id = $1 AND user_id = $2", $is_pg);
                    sqlx::query(&__sql).bind(&id).bind(uid).execute(&self.pool).await?;
                }
                None => {
                    let __sql = common::sql("DELETE FROM history_entries WHERE id = $1 AND (user_id IS NULL)", $is_pg);
                    sqlx::query(&__sql).bind(&id).execute(&self.pool).await?;
                }
            }
            Ok(())
        })
    }

    fn clear_history(&self, user_id: Option<&str>) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            match &user_id {
                Some(uid) => {
                    let __sql = common::sql("DELETE FROM history_entries WHERE user_id = $1", $is_pg);
                    sqlx::query(&__sql).bind(uid).execute(&self.pool).await?;
                }
                None => {
                    let __sql = common::sql("DELETE FROM history_entries WHERE (user_id IS NULL)", $is_pg);
                    sqlx::query(&__sql).execute(&self.pool).await?;
                }
            }
            Ok(())
        })
    }

    fn prune_history(
        &self,
        max_entries: u32,
        max_age_days: u32,
    ) -> super::BoxFuture<'_, Result<u64, StorageError>> {
        Box::pin(async move {
            let mut total_deleted = 0u64;

            let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
            let cutoff_str = cutoff.to_rfc3339();
            let __sql1 = common::sql("DELETE FROM history_entries WHERE timestamp < $1", $is_pg);
            let result = sqlx::query(&__sql1)
                .bind(&cutoff_str)
                .execute(&self.pool)
                .await?;
            total_deleted += result.rows_affected();

            let __sql2 = common::sql(
                "DELETE FROM history_entries WHERE id NOT IN (SELECT id FROM history_entries ORDER BY timestamp DESC LIMIT $1)",
                $is_pg,
            );
            let result = sqlx::query(&__sql2)
                .bind(max_entries as i64)
                .execute(&self.pool)
                .await?;
            total_deleted += result.rows_affected();

            Ok(total_deleted)
        })
    }

    // ── Environments ─────────────────────────────────────────────

    fn list_environments(&self, user_id: Option<&str>) -> super::BoxFuture<'_, Result<Vec<Environment>, StorageError>> {
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let rows = match &user_id {
                Some(uid) => {
                    let __sql = common::sql(
                        "SELECT id, name, workspace_id, created_at, updated_at FROM environments WHERE user_id = $1 ORDER BY name",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(uid).fetch_all(&self.pool).await?
                }
                None => {
                    let __sql = common::sql(
                        "SELECT id, name, workspace_id, created_at, updated_at FROM environments WHERE (user_id IS NULL) ORDER BY name",
                        $is_pg,
                    );
                    sqlx::query(&__sql).fetch_all(&self.pool).await?
                }
            };
            Ok(rows.iter().map(common::row_to_environment).collect())
        })
    }

    fn get_environment(&self, id: &str, user_id: Option<&str>) -> super::BoxFuture<'_, Result<Environment, StorageError>> {
        let id = id.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let row = match &user_id {
                Some(uid) => {
                    let __sql = common::sql(
                        "SELECT id, name, workspace_id, created_at, updated_at FROM environments WHERE id = $1 AND user_id = $2",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&id).bind(uid).fetch_one(&self.pool).await?
                }
                None => {
                    let __sql = common::sql(
                        "SELECT id, name, workspace_id, created_at, updated_at FROM environments WHERE id = $1 AND (user_id IS NULL)",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(&id).fetch_one(&self.pool).await?
                }
            };
            Ok(common::row_to_environment(&row))
        })
    }

    fn create_environment(
        &self,
        name: &str,
        user_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Environment, StorageError>> {
        let name = name.to_string();
        let user_id = user_id.map(|s| s.to_string());
        let workspace_id = workspace_id.map(|s| s.to_string());
        Box::pin(async move {
            let id = new_id();
            let now = now_iso();
            let __sql = common::sql(
                "INSERT INTO environments (id, name, user_id, workspace_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&id)
                .bind(&name)
                .bind(&user_id)
                .bind(&workspace_id)
                .bind(&now)
                .bind(&now)
                .execute(&self.pool)
                .await?;
            Ok(Environment {
                id,
                name,
                workspace_id,
                created_at: now.clone(),
                updated_at: now,
            })
        })
    }

    fn update_environment(
        &self,
        id: &str,
        name: &str,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Environment, StorageError>> {
        let id = id.to_string();
        let name = name.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let now = now_iso();
            match &user_id {
                Some(uid) => {
                    let __sql = common::sql(
                        "UPDATE environments SET name = $1, updated_at = $2 WHERE id = $3 AND user_id = $4",
                        $is_pg,
                    );
                    let result = sqlx::query(&__sql)
                        .bind(&name)
                        .bind(&now)
                        .bind(&id)
                        .bind(uid)
                        .execute(&self.pool)
                        .await?;
                    if result.rows_affected() == 0 {
                        return Err(StorageError::NotFound(format!("Environment {id} not found")));
                    }
                }
                None => {
                    let __sql = common::sql(
                        "UPDATE environments SET name = $1, updated_at = $2 WHERE id = $3 AND (user_id IS NULL)",
                        $is_pg,
                    );
                    let result = sqlx::query(&__sql)
                        .bind(&name)
                        .bind(&now)
                        .bind(&id)
                        .execute(&self.pool)
                        .await?;
                    if result.rows_affected() == 0 {
                        return Err(StorageError::NotFound(format!("Environment {id} not found")));
                    }
                }
            }
            self.get_environment(&id, user_id.as_deref()).await
        })
    }

    fn delete_environment(&self, id: &str, user_id: Option<&str>) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            match &user_id {
                Some(uid) => {
                    let __sql = common::sql("DELETE FROM environments WHERE id = $1 AND user_id = $2", $is_pg);
                    sqlx::query(&__sql).bind(&id).bind(uid).execute(&self.pool).await?;
                }
                None => {
                    let __sql = common::sql("DELETE FROM environments WHERE id = $1 AND (user_id IS NULL)", $is_pg);
                    sqlx::query(&__sql).bind(&id).execute(&self.pool).await?;
                }
            }
            Ok(())
        })
    }

    // ── Variables ────────────────────────────────────────────────

    fn list_variables(
        &self,
        environment_id: Option<&str>,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Vec<Variable>, StorageError>> {
        let environment_id = environment_id.map(|s| s.to_string());
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            let rows = match (&environment_id, &user_id) {
                (Some(eid), Some(uid)) => {
                    let __sql = common::sql(
                        "SELECT id, environment_id, key, value, is_secret FROM variables WHERE environment_id = $1 AND user_id = $2 ORDER BY key",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(eid).bind(uid).fetch_all(&self.pool).await?
                }
                (Some(eid), None) => {
                    let __sql = common::sql(
                        "SELECT id, environment_id, key, value, is_secret FROM variables WHERE environment_id = $1 AND (user_id IS NULL) ORDER BY key",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(eid).fetch_all(&self.pool).await?
                }
                (None, Some(uid)) => {
                    let __sql = common::sql(
                        "SELECT id, environment_id, key, value, is_secret FROM variables WHERE environment_id IS NULL AND user_id = $1 ORDER BY key",
                        $is_pg,
                    );
                    sqlx::query(&__sql).bind(uid).fetch_all(&self.pool).await?
                }
                (None, None) => {
                    let __sql = common::sql(
                        "SELECT id, environment_id, key, value, is_secret FROM variables WHERE environment_id IS NULL AND (user_id IS NULL) ORDER BY key",
                        $is_pg,
                    );
                    sqlx::query(&__sql).fetch_all(&self.pool).await?
                }
            };
            Ok(rows.iter().map(common::row_to_variable).collect())
        })
    }

    fn set_variable(
        &self,
        variable: &Variable,
        user_id: Option<&str>,
    ) -> super::BoxFuture<'_, Result<Variable, StorageError>> {
        let mut variable = variable.clone();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            if variable.id.is_empty() {
                variable.id = new_id();
            }
            let __sql = common::sql(
                "INSERT INTO variables (id, environment_id, key, value, is_secret, user_id) VALUES ($1, $2, $3, $4, $5, $6) \
                 ON CONFLICT(environment_id, key, user_id) DO UPDATE SET value = excluded.value, is_secret = excluded.is_secret",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&variable.id)
                .bind(&variable.environment_id)
                .bind(&variable.key)
                .bind(&variable.value)
                .bind(variable.is_secret)
                .bind(&user_id)
                .execute(&self.pool)
                .await?;
            Ok(variable)
        })
    }

    fn delete_variable(&self, id: &str, user_id: Option<&str>) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        let user_id = user_id.map(|s| s.to_string());
        Box::pin(async move {
            match &user_id {
                Some(uid) => {
                    let __sql = common::sql("DELETE FROM variables WHERE id = $1 AND user_id = $2", $is_pg);
                    sqlx::query(&__sql).bind(&id).bind(uid).execute(&self.pool).await?;
                }
                None => {
                    let __sql = common::sql("DELETE FROM variables WHERE id = $1 AND (user_id IS NULL)", $is_pg);
                    sqlx::query(&__sql).bind(&id).execute(&self.pool).await?;
                }
            }
            Ok(())
        })
    }

    // ── Users & Auth ─────────────────────────────────────────────

    fn create_user(
        &self,
        email: &str,
        password_hash: &str,
        display_name: Option<&str>,
    ) -> super::BoxFuture<'_, Result<crate::models::User, StorageError>> {
        let email = email.to_string();
        let password_hash = password_hash.to_string();
        let display_name = display_name.map(|s| s.to_string());
        Box::pin(async move {
            let id = new_id();
            let now = now_iso();
            let __sql = common::sql(
                "INSERT INTO users (id, email, password_hash, display_name, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&id)
                .bind(&email)
                .bind(&password_hash)
                .bind(&display_name)
                .bind(&now)
                .bind(&now)
                .execute(&self.pool)
                .await?;
            Ok(crate::models::User {
                id,
                email,
                display_name,
                created_at: now.clone(),
                updated_at: now,
            })
        })
    }

    fn get_user_by_email(
        &self,
        email: &str,
    ) -> super::BoxFuture<'_, Result<crate::models::UserWithHash, StorageError>> {
        let email = email.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, email, password_hash, display_name, created_at, updated_at FROM users WHERE email = $1",
                $is_pg,
            );
            let row = sqlx::query(&__sql)
                .bind(&email)
                .fetch_one(&self.pool)
                .await?;
            Ok(common::row_to_user_with_hash(&row))
        })
    }

    fn get_user_by_id(
        &self,
        id: &str,
    ) -> super::BoxFuture<'_, Result<crate::models::User, StorageError>> {
        let id = id.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, email, display_name, created_at, updated_at FROM users WHERE id = $1",
                $is_pg,
            );
            let row = sqlx::query(&__sql)
                .bind(&id)
                .fetch_one(&self.pool)
                .await?;
            Ok(common::row_to_user(&row))
        })
    }

    fn list_all_users(&self) -> super::BoxFuture<'_, Result<Vec<crate::models::User>, StorageError>> {
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, email, display_name, created_at, updated_at FROM users",
                $is_pg,
            );
            let rows = sqlx::query(&__sql)
                .fetch_all(&self.pool)
                .await?;
            Ok(rows.iter().map(common::row_to_user).collect())
        })
    }

    fn store_refresh_token(
        &self,
        user_id: &str,
        token_hash: &str,
        expires_at: &str,
    ) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let user_id = user_id.to_string();
        let token_hash = token_hash.to_string();
        let expires_at = expires_at.to_string();
        Box::pin(async move {
            let id = new_id();
            let now = now_iso();
            let __sql = common::sql(
                "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at) VALUES ($1, $2, $3, $4, $5)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&id)
                .bind(&user_id)
                .bind(&token_hash)
                .bind(&expires_at)
                .bind(&now)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }

    fn get_refresh_token(
        &self,
        token_hash: &str,
    ) -> super::BoxFuture<'_, Result<crate::models::RefreshToken, StorageError>> {
        let token_hash = token_hash.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, user_id, token_hash, expires_at, created_at FROM refresh_tokens WHERE token_hash = $1",
                $is_pg,
            );
            let row = sqlx::query(&__sql)
                .bind(&token_hash)
                .fetch_one(&self.pool)
                .await?;
            Ok(common::row_to_refresh_token(&row))
        })
    }

    fn delete_refresh_token(
        &self,
        token_hash: &str,
    ) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let token_hash = token_hash.to_string();
        Box::pin(async move {
            let __sql = common::sql("DELETE FROM refresh_tokens WHERE token_hash = $1", $is_pg);
            sqlx::query(&__sql)
                .bind(&token_hash)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }

    fn delete_user_refresh_tokens(
        &self,
        user_id: &str,
    ) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let user_id = user_id.to_string();
        Box::pin(async move {
            let __sql = common::sql("DELETE FROM refresh_tokens WHERE user_id = $1", $is_pg);
            sqlx::query(&__sql)
                .bind(&user_id)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }

    // ── Server Connections ───────────────────────────────────────

    fn list_connections(&self) -> super::BoxFuture<'_, Result<Vec<crate::models::ServerConnection>, StorageError>> {
        Box::pin(async {
            let rows = sqlx::query(
                "SELECT id, server_url, display_name, access_token, refresh_token, user_email, user_display_name, user_server_id, server_mode, status, created_at, last_used_at FROM server_connections ORDER BY display_name",
            )
            .fetch_all(&self.pool)
            .await?;
            Ok(rows.iter().map(|r| common::row_to_connection(r, &self.encryption_key)).collect())
        })
    }

    fn get_connection(&self, id: &str) -> super::BoxFuture<'_, Result<crate::models::ServerConnection, StorageError>> {
        let id = id.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, server_url, display_name, access_token, refresh_token, user_email, user_display_name, user_server_id, server_mode, status, created_at, last_used_at FROM server_connections WHERE id = $1",
                $is_pg,
            );
            let row = sqlx::query(&__sql)
                .bind(&id)
                .fetch_one(&self.pool)
                .await?;
            Ok(common::row_to_connection(&row, &self.encryption_key))
        })
    }

    fn create_connection(
        &self,
        connection: &crate::models::ServerConnection,
    ) -> super::BoxFuture<'_, Result<crate::models::ServerConnection, StorageError>> {
        let mut conn = connection.clone();
        let access_token_enc = conn.access_token.as_deref()
            .and_then(|t| crate::crypto::encrypt(t, &self.encryption_key).ok());
        let refresh_token_enc = conn.refresh_token.as_deref()
            .and_then(|t| crate::crypto::encrypt(t, &self.encryption_key).ok());
        Box::pin(async move {
            conn.id = new_id();
            let now = now_iso();
            conn.created_at = now.clone();
            let __sql = common::sql(
                "INSERT INTO server_connections (id, server_url, display_name, access_token, refresh_token, user_email, user_display_name, user_server_id, server_mode, status, created_at, last_used_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&conn.id)
                .bind(&conn.server_url)
                .bind(&conn.display_name)
                .bind(&access_token_enc)
                .bind(&refresh_token_enc)
                .bind(&conn.user_email)
                .bind(&conn.user_display_name)
                .bind(&conn.user_server_id)
                .bind(&conn.server_mode)
                .bind(&conn.status)
                .bind(&conn.created_at)
                .bind(&conn.last_used_at)
                .execute(&self.pool)
                .await?;
            Ok(conn)
        })
    }

    fn update_connection_tokens(
        &self,
        id: &str,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
    ) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        let access_token_enc = access_token
            .and_then(|t| crate::crypto::encrypt(t, &self.encryption_key).ok());
        let refresh_token_enc = refresh_token
            .and_then(|t| crate::crypto::encrypt(t, &self.encryption_key).ok());
        Box::pin(async move {
            let __sql = common::sql(
                "UPDATE server_connections SET access_token = $1, refresh_token = $2 WHERE id = $3",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&access_token_enc)
                .bind(&refresh_token_enc)
                .bind(&id)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }

    fn update_connection_status(
        &self,
        id: &str,
        status: &str,
    ) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        let status = status.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "UPDATE server_connections SET status = $1 WHERE id = $2",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&status)
                .bind(&id)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }

    fn delete_connection(&self, id: &str) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        Box::pin(async move {
            let __sql = common::sql("DELETE FROM server_connections WHERE id = $1", $is_pg);
            sqlx::query(&__sql)
                .bind(&id)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }

    fn get_connection_tokens(
        &self,
        id: &str,
    ) -> super::BoxFuture<'_, Result<(Option<String>, Option<String>), StorageError>> {
        let id = id.to_string();
        let encryption_key = self.encryption_key;
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT access_token, refresh_token FROM server_connections WHERE id = $1",
                $is_pg,
            );
            let row = sqlx::query(&__sql)
                .bind(&id)
                .fetch_one(&self.pool)
                .await?;
            let access_enc: Option<String> = sqlx::Row::get(&row, "access_token");
            let refresh_enc: Option<String> = sqlx::Row::get(&row, "refresh_token");
            let access = access_enc.as_deref().and_then(|e| crate::crypto::decrypt(e, &encryption_key).ok());
            let refresh = refresh_enc.as_deref().and_then(|e| crate::crypto::decrypt(e, &encryption_key).ok());
            Ok((access, refresh))
        })
    }

    // ── WebSocket Sessions ───────────────────────────────────────

    fn create_ws_session(
        &self,
        session: &WsSession,
    ) -> super::BoxFuture<'_, Result<WsSession, StorageError>> {
        let session = session.clone();
        Box::pin(async move {
            let __sql = common::sql(
                "INSERT INTO ws_sessions (id, url, connected_at, disconnected_at, duration_ms, message_count) VALUES ($1, $2, $3, $4, $5, $6)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&session.id)
                .bind(&session.url)
                .bind(&session.connected_at)
                .bind(&session.disconnected_at)
                .bind(session.duration_ms.map(|v| v as i64))
                .bind(session.message_count as i64)
                .execute(&self.pool)
                .await?;
            Ok(session)
        })
    }

    fn list_ws_sessions(
        &self,
        limit: u32,
        offset: u32,
    ) -> super::BoxFuture<'_, Result<Vec<WsSession>, StorageError>> {
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, url, connected_at, disconnected_at, duration_ms, message_count FROM ws_sessions ORDER BY connected_at DESC LIMIT $1 OFFSET $2",
                $is_pg,
            );
            let rows = sqlx::query(&__sql)
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(&self.pool)
                .await?;
            Ok(rows.iter().map(common::row_to_ws_session).collect())
        })
    }

    fn delete_ws_session(&self, id: &str) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        Box::pin(async move {
            let __sql = common::sql("DELETE FROM ws_sessions WHERE id = $1", $is_pg);
            sqlx::query(&__sql)
                .bind(&id)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }

    fn cleanup_orphaned_ws_sessions(&self) -> super::BoxFuture<'_, Result<u64, StorageError>> {
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE ws_sessions SET disconnected_at = connected_at, duration_ms = 0 WHERE disconnected_at IS NULL",
            )
            .execute(&self.pool)
            .await?;
            Ok(result.rows_affected())
        })
    }

    // ── Workspaces ────────────────────────────────────────────────

    fn create_workspace(&self, name: &str, description: Option<&str>, owner_id: &str, is_personal: bool) -> super::BoxFuture<'_, Result<Workspace, StorageError>> {
        let name = name.to_string();
        let description = description.map(|s| s.to_string());
        let owner_id = owner_id.to_string();
        Box::pin(async move {
            let id = new_id();
            let now = now_iso();
            let __sql = common::sql(
                "INSERT INTO workspaces (id, name, description, owner_id, is_personal, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&id)
                .bind(&name)
                .bind(&description)
                .bind(&owner_id)
                .bind(is_personal)
                .bind(&now)
                .bind(&now)
                .execute(&self.pool)
                .await?;
            Ok(Workspace {
                id,
                name,
                description,
                owner_id,
                is_personal,
                created_at: now.clone(),
                updated_at: now,
            })
        })
    }

    fn get_workspace(&self, id: &str) -> super::BoxFuture<'_, Result<Workspace, StorageError>> {
        let id = id.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, name, description, owner_id, is_personal, created_at, updated_at FROM workspaces WHERE id = $1",
                $is_pg,
            );
            let row = sqlx::query(&__sql).bind(&id).fetch_one(&self.pool).await?;
            Ok(common::row_to_workspace(&row))
        })
    }

    fn update_workspace(&self, id: &str, name: &str, description: Option<&str>) -> super::BoxFuture<'_, Result<Workspace, StorageError>> {
        let id = id.to_string();
        let name = name.to_string();
        let description = description.map(|s| s.to_string());
        Box::pin(async move {
            let now = now_iso();
            let __sql = common::sql(
                "UPDATE workspaces SET name = $1, description = $2, updated_at = $3 WHERE id = $4",
                $is_pg,
            );
            let result = sqlx::query(&__sql)
                .bind(&name)
                .bind(&description)
                .bind(&now)
                .bind(&id)
                .execute(&self.pool)
                .await?;
            if result.rows_affected() == 0 {
                return Err(StorageError::NotFound(format!("Workspace {id} not found")));
            }
            self.get_workspace(&id).await
        })
    }

    fn delete_workspace(&self, id: &str) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        Box::pin(async move {
            let __sql = common::sql("DELETE FROM workspaces WHERE id = $1", $is_pg);
            sqlx::query(&__sql).bind(&id).execute(&self.pool).await?;
            Ok(())
        })
    }

    fn list_user_workspaces(&self, user_id: &str) -> super::BoxFuture<'_, Result<Vec<Workspace>, StorageError>> {
        let user_id = user_id.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT w.id, w.name, w.description, w.owner_id, w.is_personal, w.created_at, w.updated_at FROM workspaces w INNER JOIN workspace_members wm ON w.id = wm.workspace_id WHERE wm.user_id = $1 ORDER BY w.name",
                $is_pg,
            );
            let rows = sqlx::query(&__sql).bind(&user_id).fetch_all(&self.pool).await?;
            Ok(rows.iter().map(common::row_to_workspace).collect())
        })
    }

    fn get_personal_workspace(&self, user_id: &str) -> super::BoxFuture<'_, Result<Workspace, StorageError>> {
        let user_id = user_id.to_string();
        Box::pin(async move {
            let __sql = if $is_pg {
                "SELECT id, name, description, owner_id, is_personal, created_at, updated_at FROM workspaces WHERE owner_id = $1 AND is_personal = true".to_string()
            } else {
                "SELECT id, name, description, owner_id, is_personal, created_at, updated_at FROM workspaces WHERE owner_id = ? AND is_personal = 1".to_string()
            };
            let row = sqlx::query(&__sql).bind(&user_id).fetch_one(&self.pool).await?;
            Ok(common::row_to_workspace(&row))
        })
    }

    // ── Workspace Members ───────────────────────────────────────

    fn add_workspace_member(&self, workspace_id: &str, user_id: &str, role: &str) -> super::BoxFuture<'_, Result<WorkspaceMember, StorageError>> {
        let workspace_id = workspace_id.to_string();
        let user_id = user_id.to_string();
        let role = role.to_string();
        Box::pin(async move {
            let id = new_id();
            let now = now_iso();
            let __sql = common::sql(
                "INSERT INTO workspace_members (id, workspace_id, user_id, role, created_at) VALUES ($1, $2, $3, $4, $5)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&id)
                .bind(&workspace_id)
                .bind(&user_id)
                .bind(&role)
                .bind(&now)
                .execute(&self.pool)
                .await?;
            Ok(WorkspaceMember {
                id,
                workspace_id,
                user_id,
                role: role.parse().unwrap_or(WorkspaceRole::Viewer),
                created_at: now,
            })
        })
    }

    fn remove_workspace_member(&self, workspace_id: &str, user_id: &str) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let workspace_id = workspace_id.to_string();
        let user_id = user_id.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "DELETE FROM workspace_members WHERE workspace_id = $1 AND user_id = $2",
                $is_pg,
            );
            sqlx::query(&__sql).bind(&workspace_id).bind(&user_id).execute(&self.pool).await?;
            Ok(())
        })
    }

    fn update_workspace_member_role(&self, workspace_id: &str, user_id: &str, role: &str) -> super::BoxFuture<'_, Result<WorkspaceMember, StorageError>> {
        let workspace_id = workspace_id.to_string();
        let user_id = user_id.to_string();
        let role = role.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "UPDATE workspace_members SET role = $1 WHERE workspace_id = $2 AND user_id = $3",
                $is_pg,
            );
            let result = sqlx::query(&__sql)
                .bind(&role)
                .bind(&workspace_id)
                .bind(&user_id)
                .execute(&self.pool)
                .await?;
            if result.rows_affected() == 0 {
                return Err(StorageError::NotFound("Workspace member not found".into()));
            }
            self.get_workspace_member(&workspace_id, &user_id).await
        })
    }

    fn list_workspace_members(&self, workspace_id: &str) -> super::BoxFuture<'_, Result<Vec<WorkspaceMember>, StorageError>> {
        let workspace_id = workspace_id.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, workspace_id, user_id, role, created_at FROM workspace_members WHERE workspace_id = $1 ORDER BY created_at",
                $is_pg,
            );
            let rows = sqlx::query(&__sql).bind(&workspace_id).fetch_all(&self.pool).await?;
            Ok(rows.iter().map(common::row_to_workspace_member).collect())
        })
    }

    fn get_workspace_member(&self, workspace_id: &str, user_id: &str) -> super::BoxFuture<'_, Result<WorkspaceMember, StorageError>> {
        let workspace_id = workspace_id.to_string();
        let user_id = user_id.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, workspace_id, user_id, role, created_at FROM workspace_members WHERE workspace_id = $1 AND user_id = $2",
                $is_pg,
            );
            let row = sqlx::query(&__sql).bind(&workspace_id).bind(&user_id).fetch_one(&self.pool).await?;
            Ok(common::row_to_workspace_member(&row))
        })
    }

    // ── Workspace Invites ───────────────────────────────────────

    fn create_workspace_invite(&self, workspace_id: &str, email: &str, role: &str, token: &str, expires_at: &str) -> super::BoxFuture<'_, Result<WorkspaceInvite, StorageError>> {
        let workspace_id = workspace_id.to_string();
        let email = email.to_string();
        let role = role.to_string();
        let token = token.to_string();
        let expires_at = expires_at.to_string();
        Box::pin(async move {
            let id = new_id();
            let now = now_iso();
            let __sql = common::sql(
                "INSERT INTO workspace_invites (id, workspace_id, email, role, token, expires_at, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                $is_pg,
            );
            sqlx::query(&__sql)
                .bind(&id)
                .bind(&workspace_id)
                .bind(&email)
                .bind(&role)
                .bind(&token)
                .bind(&expires_at)
                .bind(&now)
                .execute(&self.pool)
                .await?;
            Ok(WorkspaceInvite {
                id,
                workspace_id,
                email,
                role: role.parse().unwrap_or(WorkspaceRole::Viewer),
                token,
                expires_at,
                created_at: now,
            })
        })
    }

    fn get_workspace_invite_by_token(&self, token: &str) -> super::BoxFuture<'_, Result<WorkspaceInvite, StorageError>> {
        let token = token.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, workspace_id, email, role, token, expires_at, created_at FROM workspace_invites WHERE token = $1",
                $is_pg,
            );
            let row = sqlx::query(&__sql).bind(&token).fetch_one(&self.pool).await?;
            Ok(common::row_to_workspace_invite(&row))
        })
    }

    fn delete_workspace_invite(&self, id: &str) -> super::BoxFuture<'_, Result<(), StorageError>> {
        let id = id.to_string();
        Box::pin(async move {
            let __sql = common::sql("DELETE FROM workspace_invites WHERE id = $1", $is_pg);
            sqlx::query(&__sql).bind(&id).execute(&self.pool).await?;
            Ok(())
        })
    }

    fn list_workspace_invites(&self, workspace_id: &str) -> super::BoxFuture<'_, Result<Vec<WorkspaceInvite>, StorageError>> {
        let workspace_id = workspace_id.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, workspace_id, email, role, token, expires_at, created_at FROM workspace_invites WHERE workspace_id = $1 ORDER BY created_at",
                $is_pg,
            );
            let rows = sqlx::query(&__sql).bind(&workspace_id).fetch_all(&self.pool).await?;
            Ok(rows.iter().map(common::row_to_workspace_invite).collect())
        })
    }

    // ── Workspace-scoped queries ────────────────────────────────

    fn list_workspace_collections(&self, workspace_id: &str) -> super::BoxFuture<'_, Result<Vec<Collection>, StorageError>> {
        let workspace_id = workspace_id.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, name, description, auth, pre_request_script, test_script, workspace_id, created_at, updated_at FROM collections WHERE workspace_id = $1 ORDER BY name",
                $is_pg,
            );
            let rows = sqlx::query(&__sql).bind(&workspace_id).fetch_all(&self.pool).await?;
            Ok(rows.iter().map(|r| common::row_to_collection(r, &self.encryption_key)).collect())
        })
    }

    fn list_workspace_environments(&self, workspace_id: &str) -> super::BoxFuture<'_, Result<Vec<Environment>, StorageError>> {
        let workspace_id = workspace_id.to_string();
        Box::pin(async move {
            let __sql = common::sql(
                "SELECT id, name, workspace_id, created_at, updated_at FROM environments WHERE workspace_id = $1 ORDER BY name",
                $is_pg,
            );
            let rows = sqlx::query(&__sql).bind(&workspace_id).fetch_all(&self.pool).await?;
            Ok(rows.iter().map(common::row_to_environment).collect())
        })
    }

        } // impl StorageBackend
    }; // @impl
}
