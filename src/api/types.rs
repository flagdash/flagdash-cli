use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// Deserializes a value that may be `null` or missing into `T::default()`.
/// Use with `#[serde(default, deserialize_with = "null_default")]`.
fn null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(|v| v.unwrap_or_default())
}

// ── Management tier types ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagEnvironmentData {
    pub id: String,
    pub environment_id: String,
    pub enabled: bool,
    pub value: serde_json::Value,
    pub rules: serde_json::Value,
    pub rollout_percentage: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedFlag {
    pub id: String,
    pub key: String,
    pub name: String,
    #[serde(default, deserialize_with = "null_default")]
    pub description: String,
    pub flag_type: String,
    pub default_value: serde_json::Value,
    #[serde(default, deserialize_with = "null_default")]
    pub tags: Vec<String>,
    #[serde(default, deserialize_with = "null_default")]
    pub is_archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, deserialize_with = "null_default")]
    pub environments: Vec<FlagEnvironmentData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedFlagsResponse {
    pub flags: Vec<ManagedFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedFlagResponse {
    pub flag: ManagedFlag,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagEnvironmentResponse {
    pub flag_environment: FlagEnvironmentResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagEnvironmentResponseData {
    pub id: String,
    pub enabled: bool,
    pub environment_id: String,
    pub feature_flag_id: String,
    #[serde(default)]
    pub rules: serde_json::Value,
    #[serde(default)]
    pub rollout_percentage: i32,
}

// ── Variations ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variation {
    pub id: String,
    pub key: String,
    pub name: String,
    pub value: serde_json::Value,
    pub weight: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariationsResponse {
    pub variations: Vec<Variation>,
}

// ── Schedules ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub action: String,
    pub scheduled_at: DateTime<Utc>,
    #[serde(default)]
    pub executed_at: Option<DateTime<Utc>>,
    pub status: String,
    #[serde(default, deserialize_with = "null_default")]
    pub payload: serde_json::Value,
    #[serde(default, deserialize_with = "null_default")]
    pub error_message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulesResponse {
    pub schedules: Vec<Schedule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResponse {
    pub schedule: Schedule,
}

// ── Configs ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEnvironmentValue {
    pub id: String,
    pub environment_id: String,
    pub value: serde_json::Value,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedConfig {
    pub id: String,
    pub key: String,
    pub name: String,
    #[serde(default, deserialize_with = "null_default")]
    pub description: String,
    pub config_type: String,
    pub default_value: serde_json::Value,
    #[serde(default, deserialize_with = "null_default")]
    pub tags: Vec<String>,
    #[serde(default, deserialize_with = "null_default")]
    pub is_archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, deserialize_with = "null_default")]
    pub environments: Vec<ConfigEnvironmentValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedConfigsResponse {
    pub configs: Vec<ManagedConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedConfigResponse {
    pub config: ManagedConfig,
}

// ── Secrets ──────────────────────────────────────────────────────────
//
// Metadata only, on purpose. Nothing in this file can carry a decrypted value:
// the management API has no action that returns one, and the CLI must never
// write a credential into its config file or any cache.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedSecret {
    pub id: String,
    pub key: String,
    pub name: String,
    #[serde(default, deserialize_with = "null_default")]
    pub description: String,
    pub format: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, deserialize_with = "null_default")]
    pub has_value: bool,
    #[serde(default)]
    pub current_version_id: Option<String>,
    #[serde(default)]
    pub pending_version_id: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    pub approval_pending: bool,
    #[serde(default)]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedSecretsResponse {
    pub secrets: Vec<ManagedSecret>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedSecretResponse {
    pub secret: ManagedSecret,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretVersion {
    pub version_id: String,
    pub created_at: DateTime<Utc>,
    #[serde(default, deserialize_with = "null_default")]
    pub created_by_id: String,
    #[serde(default, deserialize_with = "null_default")]
    pub current: bool,
    #[serde(default, deserialize_with = "null_default")]
    pub pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretHistoryResponse {
    pub secret: ManagedSecret,
    pub versions: Vec<SecretVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretVersionResponse {
    pub version: SecretVersion,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateSecretRequest {
    pub project_id: String,
    pub environment_id: String,
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub format: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplaceSecretRequest {
    pub value: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoreSecretRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEnvironmentResponse {
    pub config_environment: ConfigEnvironmentResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEnvironmentResponseData {
    pub id: String,
    pub value: serde_json::Value,
    pub environment_id: String,
    pub remote_config_id: String,
}

// ── AI Configs ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedAiConfig {
    pub id: String,
    pub file_name: String,
    pub file_type: String,
    pub content: String,
    #[serde(default, deserialize_with = "null_default")]
    pub is_active: bool,
    #[serde(default, deserialize_with = "null_default")]
    pub metadata: serde_json::Value,
    #[serde(default, deserialize_with = "null_default")]
    pub folder: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub project_id: String,
    pub environment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedAiConfigsResponse {
    pub ai_configs: Vec<ManagedAiConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedAiConfigResponse {
    pub ai_config: ManagedAiConfig,
}

// ── Experiments ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedExperiment {
    pub id: String,
    pub project_id: String,
    pub key: String,
    pub name: String,
    #[serde(default, deserialize_with = "null_default")]
    pub hypothesis: String,
    #[serde(default, deserialize_with = "null_default")]
    pub description: String,
    pub status: String,
    pub randomization_unit: String,
    #[serde(default, deserialize_with = "null_default")]
    pub layer_key: String,
    #[serde(default, deserialize_with = "null_default")]
    pub variants: serde_json::Value,
    #[serde(default, deserialize_with = "null_default")]
    pub parameters: serde_json::Value,
    #[serde(default, deserialize_with = "null_default")]
    pub metrics: serde_json::Value,
    #[serde(default)]
    pub target_exposures: Option<i32>,
    #[serde(default)]
    pub target_duration_days: Option<i32>,
    #[serde(default, deserialize_with = "null_default")]
    pub decision: String,
    #[serde(default, deserialize_with = "null_default")]
    pub decision_notes: String,
    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedExperimentsResponse {
    pub experiments: Vec<ManagedExperiment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedExperimentResponse {
    pub experiment: ManagedExperiment,
}

// ── Webhooks ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpoint {
    pub id: String,
    pub url: String,
    #[serde(default, deserialize_with = "null_default")]
    pub description: String,
    pub environment_id: String,
    pub event_types: Vec<String>,
    pub is_active: bool,
    #[serde(default, deserialize_with = "null_default")]
    pub consecutive_failures: i32,
    #[serde(default)]
    pub disabled_at: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "null_default")]
    pub disabled_reason: String,
    #[serde(default, deserialize_with = "null_default")]
    pub signing_secret: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpointsResponse {
    pub endpoints: Vec<WebhookEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpointResponse {
    pub endpoint: WebhookEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub id: String,
    pub event_type: String,
    pub status: String,
    pub http_status: i32,
    #[serde(default, deserialize_with = "null_default")]
    pub error_message: String,
    pub attempt_count: i32,
    pub max_attempts: i32,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeliveriesResponse {
    pub deliveries: Vec<WebhookDelivery>,
}

// ── Projects ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectsResponse {
    pub projects: Vec<Project>,
}

// ── Environments ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub slug: String,
    #[serde(default, deserialize_with = "null_default")]
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentsResponse {
    pub environments: Vec<Environment>,
}

// ── Request types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CreateFlagRequest {
    pub project_id: String,
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub flag_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateFlagRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateRulesRequest {
    pub rules: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateRolloutRequest {
    pub rollout_percentage: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct VariationInput {
    pub key: String,
    pub name: String,
    pub value: serde_json::Value,
    pub weight: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetVariationsRequest {
    pub variations: Vec<VariationInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateScheduleRequest {
    pub action: String,
    pub scheduled_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateConfigRequest {
    pub project_id: String,
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub config_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateConfigValueRequest {
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateAiConfigRequest {
    pub project_id: String,
    pub environment_id: String,
    pub file_name: String,
    pub file_type: String,
    pub content: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub folder: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateAiConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateExperimentRequest {
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hypothesis: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub variants: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateExperimentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hypothesis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InitializeAiConfigsRequest {
    pub project_id: String,
    pub environment_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateWebhookRequest {
    pub project_id: String,
    pub environment_id: String,
    pub url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub event_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateWebhookRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

// ── Device Auth ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTokenResponse {
    #[serde(default)]
    pub session_token: Option<String>,
    #[serde(default)]
    pub account: Option<DeviceTokenAccount>,
    #[serde(default)]
    pub user: Option<DeviceTokenUser>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTokenAccount {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTokenUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceAuthRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceTokenRequest {
    pub device_code: String,
}

// ── OAuth 2.1 device grant (RFC 8628) ────────────────────────────────
//
// The successor to the DeviceAuth types above. Same shape of flow, but the
// credential it yields is a scoped, revocable OAuth token pair rather than an
// unscoped session token, and it is issued by the authorization server at
// /oauth/* rather than by /api/v1/auth/device.

/// RFC 7591 dynamic client registration. Done once per installation.
#[derive(Debug, Clone, Serialize)]
pub struct OAuthRegisterRequest {
    pub client_name: String,
    /// A device-flow client has no redirect URI; the server only requires them
    /// for grants that actually redirect.
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthRegisterResponse {
    pub client_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthDeviceAuthRequest {
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthDeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    /// The same URL with the code prefilled, for when a browser can be opened.
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_in: i64,
    /// Seconds to wait between polls. Honour it: polling faster earns `slow_down`.
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthTokenRequest {
    pub grant_type: String,
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(default)]
    pub scope: String,
}

/// What the token endpoint says while a device is still waiting.
///
/// These are not interchangeable and must not be collapsed: `Pending` and
/// `SlowDown` mean keep going, the rest mean stop. A client that treats them all
/// as failure quits on its first poll; one that treats them all as pending never
/// quits at all.
#[derive(Debug, Clone, PartialEq)]
pub enum DevicePollOutcome {
    Granted(OAuthTokenResponse),
    Pending,
    SlowDown,
    Denied,
    Expired,
    Failed(String),
}

impl DevicePollOutcome {
    /// Map an RFC 8628 §3.5 error code onto an outcome.
    pub fn from_error(code: &str, description: &str) -> Self {
        match code {
            "authorization_pending" => DevicePollOutcome::Pending,
            "slow_down" => DevicePollOutcome::SlowDown,
            "access_denied" => DevicePollOutcome::Denied,
            "expired_token" => DevicePollOutcome::Expired,
            other if description.is_empty() => DevicePollOutcome::Failed(other.to_string()),
            other => DevicePollOutcome::Failed(format!("{other}: {description}")),
        }
    }

    /// Whether the client should poll again.
    pub fn keep_polling(&self) -> bool {
        matches!(
            self,
            DevicePollOutcome::Pending | DevicePollOutcome::SlowDown
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthErrorResponse {
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub error_description: String,
}

// ── Identity ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityResponse {
    pub account: IdentityAccount,
    #[serde(default)]
    pub user: Option<IdentityUser>,
    #[serde(default)]
    pub credential: Option<IdentityCredential>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityAccount {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityCredential {
    #[serde(rename = "type")]
    pub credential_type: String,
    #[serde(default)]
    pub scope: String,
}

// ── Error response ───────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub message: String,
}

impl ErrorResponse {
    pub fn detail(&self) -> &str {
        if !self.message.is_empty() {
            &self.message
        } else if !self.error.is_empty() {
            &self.error
        } else {
            "Unknown error"
        }
    }
}
