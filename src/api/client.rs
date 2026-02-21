use crate::api::error::ApiError;
use crate::api::types::*;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// HTTP client for the FlagDash management API.
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    session_token: String,
}

impl ApiClient {
    pub fn new(base_url: &str, session_token: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            session_token: session_token.to_string(),
        }
    }

    /// Create a client that can only make unauthenticated requests.
    /// Used for device auth flow before the user has a session token.
    pub fn new_unauthenticated(base_url: &str) -> Self {
        Self::new(base_url, "")
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let resp = self
            .client
            .get(self.url(path))
            .bearer_auth(&self.session_token)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        self.handle_response(resp).await
    }

    async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, ApiError> {
        let mut req = self
            .client
            .post(self.url(path))
            .bearer_auth(&self.session_token);

        if let Some(b) = body {
            req = req.json(b);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;
        self.handle_response(resp).await
    }

    async fn put<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let resp = self
            .client
            .put(self.url(path))
            .bearer_auth(&self.session_token)
            .json(body)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        self.handle_response(resp).await
    }

    async fn delete(&self, path: &str) -> Result<(), ApiError> {
        let resp = self
            .client
            .delete(self.url(path))
            .bearer_auth(&self.session_token)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        if (200..300).contains(&status) {
            Ok(())
        } else {
            let err = self.parse_error(resp).await;
            Err(err)
        }
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, ApiError> {
        let status = resp.status().as_u16();
        if (200..300).contains(&status) {
            let body = resp
                .text()
                .await
                .map_err(|e| ApiError::Parse(e.to_string()))?;
            serde_json::from_str::<T>(&body).map_err(|e| {
                let preview = if body.len() > 200 {
                    format!("{}...", &body[..200])
                } else {
                    body
                };
                ApiError::Parse(format!("{e} | body: {preview}"))
            })
        } else {
            Err(self.parse_error(resp).await)
        }
    }

    async fn parse_error(&self, resp: reqwest::Response) -> ApiError {
        let status = resp.status().as_u16();
        match status {
            401 => ApiError::Unauthorized,
            403 => ApiError::Forbidden,
            404 => {
                let body: ErrorResponse = resp.json().await.unwrap_or(ErrorResponse {
                    error: "Not found".into(),
                    message: String::new(),
                });
                ApiError::NotFound(body.detail().to_string())
            }
            422 => {
                let body: ErrorResponse = resp.json().await.unwrap_or(ErrorResponse {
                    error: "Validation error".into(),
                    message: String::new(),
                });
                ApiError::Validation(body.detail().to_string())
            }
            429 => ApiError::RateLimited,
            _ => {
                let body: ErrorResponse = resp.json().await.unwrap_or(ErrorResponse {
                    error: format!("HTTP {status}"),
                    message: String::new(),
                });
                ApiError::Http {
                    status,
                    message: body.detail().to_string(),
                }
            }
        }
    }

    // ── Unauthenticated requests ────────────────────────────────────

    async fn post_no_auth<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, ApiError> {
        let mut req = self.client.post(self.url(path));

        if let Some(b) = body {
            req = req.json(b);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;
        self.handle_response(resp).await
    }

    // ── Device Auth ─────────────────────────────────────────────────

    /// POST /api/v1/auth/device -- Request device authorization
    pub async fn request_device_auth(
        &self,
        device_name: Option<&str>,
    ) -> Result<DeviceAuthResponse, ApiError> {
        let body = DeviceAuthRequest {
            device_name: device_name.map(|s| s.to_string()),
        };
        self.post_no_auth("/auth/device", Some(&body)).await
    }

    /// POST /api/v1/auth/device/token -- Poll for token
    pub async fn poll_device_token(
        &self,
        device_code: &str,
    ) -> Result<DeviceTokenResponse, ApiError> {
        let body = DeviceTokenRequest {
            device_code: device_code.to_string(),
        };
        self.post_no_auth("/auth/device/token", Some(&body)).await
    }

    // ── Flags ────────────────────────────────────────────────────────

    pub async fn list_flags(&self, project_id: &str) -> Result<Vec<ManagedFlag>, ApiError> {
        let resp: ManagedFlagsResponse = self
            .get(&format!(
                "/manage/flags?project_id={}",
                urlencoding(project_id)
            ))
            .await?;
        Ok(resp.flags)
    }

    pub async fn get_flag(&self, key: &str, project_id: &str) -> Result<ManagedFlag, ApiError> {
        let resp: ManagedFlagResponse = self
            .get(&format!(
                "/manage/flags/{}?project_id={}",
                urlencoding(key),
                urlencoding(project_id)
            ))
            .await?;
        Ok(resp.flag)
    }

    pub async fn create_flag(&self, req: &CreateFlagRequest) -> Result<ManagedFlag, ApiError> {
        let resp: ManagedFlagResponse = self.post("/manage/flags", Some(req)).await?;
        Ok(resp.flag)
    }

    pub async fn update_flag(
        &self,
        key: &str,
        project_id: &str,
        req: &UpdateFlagRequest,
    ) -> Result<ManagedFlag, ApiError> {
        let resp: ManagedFlagResponse = self
            .put(
                &format!(
                    "/manage/flags/{}?project_id={}",
                    urlencoding(key),
                    urlencoding(project_id)
                ),
                req,
            )
            .await?;
        Ok(resp.flag)
    }

    pub async fn delete_flag(&self, key: &str, project_id: &str) -> Result<(), ApiError> {
        self.delete(&format!(
            "/manage/flags/{}?project_id={}",
            urlencoding(key),
            urlencoding(project_id)
        ))
        .await
    }

    pub async fn toggle_flag(
        &self,
        key: &str,
        project_id: &str,
        environment_id: &str,
    ) -> Result<FlagEnvironmentResponse, ApiError> {
        self.post::<(), FlagEnvironmentResponse>(
            &format!(
                "/manage/flags/{}/toggle?project_id={}&environment_id={}",
                urlencoding(key),
                urlencoding(project_id),
                urlencoding(environment_id)
            ),
            None,
        )
        .await
    }

    pub async fn set_rollout(
        &self,
        key: &str,
        project_id: &str,
        environment_id: &str,
        percentage: i32,
    ) -> Result<FlagEnvironmentResponse, ApiError> {
        let body = UpdateRolloutRequest {
            rollout_percentage: percentage,
        };
        self.put(
            &format!(
                "/manage/flags/{}/rollout?project_id={}&environment_id={}",
                urlencoding(key),
                urlencoding(project_id),
                urlencoding(environment_id)
            ),
            &body,
        )
        .await
    }

    pub async fn update_rules(
        &self,
        key: &str,
        project_id: &str,
        environment_id: &str,
        rules: serde_json::Value,
    ) -> Result<FlagEnvironmentResponse, ApiError> {
        let body = UpdateRulesRequest { rules };
        self.put(
            &format!(
                "/manage/flags/{}/rules?project_id={}&environment_id={}",
                urlencoding(key),
                urlencoding(project_id),
                urlencoding(environment_id)
            ),
            &body,
        )
        .await
    }

    pub async fn set_variations(
        &self,
        key: &str,
        project_id: &str,
        environment_id: &str,
        variations: Vec<VariationInput>,
    ) -> Result<Vec<Variation>, ApiError> {
        let body = SetVariationsRequest { variations };
        let resp: VariationsResponse = self
            .put(
                &format!(
                    "/manage/flags/{}/variations?project_id={}&environment_id={}",
                    urlencoding(key),
                    urlencoding(project_id),
                    urlencoding(environment_id)
                ),
                &body,
            )
            .await?;
        Ok(resp.variations)
    }

    pub async fn delete_variations(
        &self,
        key: &str,
        project_id: &str,
        environment_id: &str,
    ) -> Result<(), ApiError> {
        self.delete(&format!(
            "/manage/flags/{}/variations?project_id={}&environment_id={}",
            urlencoding(key),
            urlencoding(project_id),
            urlencoding(environment_id)
        ))
        .await
    }

    // ── Schedules ────────────────────────────────────────────────────

    pub async fn list_schedules(
        &self,
        key: &str,
        project_id: &str,
        environment_id: &str,
    ) -> Result<Vec<Schedule>, ApiError> {
        let resp: SchedulesResponse = self
            .get(&format!(
                "/manage/flags/{}/schedules?project_id={}&environment_id={}",
                urlencoding(key),
                urlencoding(project_id),
                urlencoding(environment_id)
            ))
            .await?;
        Ok(resp.schedules)
    }

    pub async fn create_schedule(
        &self,
        key: &str,
        project_id: &str,
        environment_id: &str,
        req: &CreateScheduleRequest,
    ) -> Result<Schedule, ApiError> {
        let resp: ScheduleResponse = self
            .post(
                &format!(
                    "/manage/flags/{}/schedules?project_id={}&environment_id={}",
                    urlencoding(key),
                    urlencoding(project_id),
                    urlencoding(environment_id)
                ),
                Some(req),
            )
            .await?;
        Ok(resp.schedule)
    }

    pub async fn cancel_schedule(
        &self,
        key: &str,
        project_id: &str,
        schedule_id: &str,
    ) -> Result<(), ApiError> {
        self.delete(&format!(
            "/manage/flags/{}/schedules/{}?project_id={}",
            urlencoding(key),
            urlencoding(schedule_id),
            urlencoding(project_id)
        ))
        .await
    }

    // ── Configs ──────────────────────────────────────────────────────

    pub async fn list_configs(&self, project_id: &str) -> Result<Vec<ManagedConfig>, ApiError> {
        let resp: ManagedConfigsResponse = self
            .get(&format!(
                "/manage/configs?project_id={}",
                urlencoding(project_id)
            ))
            .await?;
        Ok(resp.configs)
    }

    pub async fn get_config(&self, key: &str, project_id: &str) -> Result<ManagedConfig, ApiError> {
        let resp: ManagedConfigResponse = self
            .get(&format!(
                "/manage/configs/{}?project_id={}",
                urlencoding(key),
                urlencoding(project_id)
            ))
            .await?;
        Ok(resp.config)
    }

    pub async fn create_config(
        &self,
        req: &CreateConfigRequest,
    ) -> Result<ManagedConfig, ApiError> {
        let resp: ManagedConfigResponse = self.post("/manage/configs", Some(req)).await?;
        Ok(resp.config)
    }

    pub async fn update_config(
        &self,
        key: &str,
        project_id: &str,
        req: &UpdateConfigRequest,
    ) -> Result<ManagedConfig, ApiError> {
        let resp: ManagedConfigResponse = self
            .put(
                &format!(
                    "/manage/configs/{}?project_id={}",
                    urlencoding(key),
                    urlencoding(project_id)
                ),
                req,
            )
            .await?;
        Ok(resp.config)
    }

    pub async fn delete_config(&self, key: &str, project_id: &str) -> Result<(), ApiError> {
        self.delete(&format!(
            "/manage/configs/{}?project_id={}",
            urlencoding(key),
            urlencoding(project_id)
        ))
        .await
    }

    pub async fn set_config_value(
        &self,
        key: &str,
        project_id: &str,
        environment_id: &str,
        value: serde_json::Value,
    ) -> Result<ConfigEnvironmentResponse, ApiError> {
        let body = UpdateConfigValueRequest { value };
        self.put(
            &format!(
                "/manage/configs/{}/value?project_id={}&environment_id={}",
                urlencoding(key),
                urlencoding(project_id),
                urlencoding(environment_id)
            ),
            &body,
        )
        .await
    }

    // ── AI Configs ───────────────────────────────────────────────────

    pub async fn list_ai_configs(
        &self,
        project_id: &str,
        environment_id: &str,
    ) -> Result<Vec<ManagedAiConfig>, ApiError> {
        let resp: ManagedAiConfigsResponse = self
            .get(&format!(
                "/manage/ai-configs?project_id={}&environment_id={}",
                urlencoding(project_id),
                urlencoding(environment_id)
            ))
            .await?;
        Ok(resp.ai_configs)
    }

    pub async fn get_ai_config(
        &self,
        file_name: &str,
        project_id: &str,
        environment_id: &str,
    ) -> Result<ManagedAiConfig, ApiError> {
        let resp: ManagedAiConfigResponse = self
            .get(&format!(
                "/manage/ai-configs/{}?project_id={}&environment_id={}",
                urlencoding(file_name),
                urlencoding(project_id),
                urlencoding(environment_id)
            ))
            .await?;
        Ok(resp.ai_config)
    }

    pub async fn create_ai_config(
        &self,
        req: &CreateAiConfigRequest,
    ) -> Result<ManagedAiConfig, ApiError> {
        let resp: ManagedAiConfigResponse = self.post("/manage/ai-configs", Some(req)).await?;
        Ok(resp.ai_config)
    }

    pub async fn update_ai_config(
        &self,
        file_name: &str,
        project_id: &str,
        environment_id: &str,
        req: &UpdateAiConfigRequest,
    ) -> Result<ManagedAiConfig, ApiError> {
        let resp: ManagedAiConfigResponse = self
            .put(
                &format!(
                    "/manage/ai-configs/{}?project_id={}&environment_id={}",
                    urlencoding(file_name),
                    urlencoding(project_id),
                    urlencoding(environment_id)
                ),
                req,
            )
            .await?;
        Ok(resp.ai_config)
    }

    pub async fn delete_ai_config(
        &self,
        file_name: &str,
        project_id: &str,
        environment_id: &str,
    ) -> Result<(), ApiError> {
        self.delete(&format!(
            "/manage/ai-configs/{}?project_id={}&environment_id={}",
            urlencoding(file_name),
            urlencoding(project_id),
            urlencoding(environment_id)
        ))
        .await
    }

    pub async fn initialize_ai_configs(
        &self,
        project_id: &str,
        environment_id: &str,
    ) -> Result<Vec<ManagedAiConfig>, ApiError> {
        let body = InitializeAiConfigsRequest {
            project_id: project_id.to_string(),
            environment_id: environment_id.to_string(),
        };
        let resp: ManagedAiConfigsResponse = self
            .post("/manage/ai-configs/initialize", Some(&body))
            .await?;
        Ok(resp.ai_configs)
    }

    // ── Webhooks ─────────────────────────────────────────────────────

    pub async fn list_webhooks(&self, project_id: &str) -> Result<Vec<WebhookEndpoint>, ApiError> {
        let resp: WebhookEndpointsResponse = self
            .get(&format!(
                "/manage/webhooks?project_id={}",
                urlencoding(project_id)
            ))
            .await?;
        Ok(resp.endpoints)
    }

    pub async fn get_webhook(&self, id: &str) -> Result<WebhookEndpoint, ApiError> {
        let resp: WebhookEndpointResponse = self
            .get(&format!("/manage/webhooks/{}", urlencoding(id)))
            .await?;
        Ok(resp.endpoint)
    }

    pub async fn create_webhook(
        &self,
        req: &CreateWebhookRequest,
    ) -> Result<WebhookEndpoint, ApiError> {
        let resp: WebhookEndpointResponse = self.post("/manage/webhooks", Some(req)).await?;
        Ok(resp.endpoint)
    }

    pub async fn update_webhook(
        &self,
        id: &str,
        req: &UpdateWebhookRequest,
    ) -> Result<WebhookEndpoint, ApiError> {
        let resp: WebhookEndpointResponse = self
            .put(&format!("/manage/webhooks/{}", urlencoding(id)), req)
            .await?;
        Ok(resp.endpoint)
    }

    pub async fn delete_webhook(&self, id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/manage/webhooks/{}", urlencoding(id)))
            .await
    }

    pub async fn regenerate_webhook_secret(&self, id: &str) -> Result<WebhookEndpoint, ApiError> {
        let resp: WebhookEndpointResponse = self
            .post::<(), WebhookEndpointResponse>(
                &format!("/manage/webhooks/{}/regenerate-secret", urlencoding(id)),
                None,
            )
            .await?;
        Ok(resp.endpoint)
    }

    pub async fn reactivate_webhook(&self, id: &str) -> Result<WebhookEndpoint, ApiError> {
        let resp: WebhookEndpointResponse = self
            .post::<(), WebhookEndpointResponse>(
                &format!("/manage/webhooks/{}/reactivate", urlencoding(id)),
                None,
            )
            .await?;
        Ok(resp.endpoint)
    }

    pub async fn list_webhook_deliveries(
        &self,
        id: &str,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<WebhookDelivery>, ApiError> {
        let resp: WebhookDeliveriesResponse = self
            .get(&format!(
                "/manage/webhooks/{}/deliveries?limit={}&offset={}",
                urlencoding(id),
                limit,
                offset
            ))
            .await?;
        Ok(resp.deliveries)
    }

    // ── Projects & Environments ─────────────────────────────────────

    pub async fn list_projects(&self) -> Result<Vec<Project>, ApiError> {
        let resp: ProjectsResponse = self.get("/manage/projects").await?;
        Ok(resp.projects)
    }

    pub async fn list_environments(&self, project_id: &str) -> Result<Vec<Environment>, ApiError> {
        let resp: EnvironmentsResponse = self
            .get(&format!(
                "/manage/environments?project_id={}",
                urlencoding(project_id)
            ))
            .await?;
        Ok(resp.environments)
    }

    // ── Validation ───────────────────────────────────────────────────

    /// Quick health check: tries to list projects. Returns Ok if the key is valid.
    pub async fn validate_key(&self) -> Result<(), ApiError> {
        self.list_projects().await?;
        Ok(())
    }
}

fn urlencoding(s: &str) -> String {
    urlencoding_encode(s)
}

fn urlencoding_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}
