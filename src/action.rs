use crate::api::types::*;
use chrono::{DateTime, Utc};

/// Actions flow through the app as a message bus.
/// Components emit actions, the app loop dispatches them.
#[derive(Debug, Clone)]
pub enum Action {
    // Navigation
    Navigate(View),
    Back,
    Quit,

    // Sidebar
    SelectSection(SidebarSection),

    // Data loaded from API
    FlagsLoaded(Vec<ManagedFlag>),
    FlagLoaded(Box<ManagedFlag>),
    ConfigsLoaded(Vec<ManagedConfig>),
    ConfigLoaded(Box<ManagedConfig>),
    AiConfigsLoaded(Vec<ManagedAiConfig>),
    AiConfigLoaded(Box<ManagedAiConfig>),
    WebhooksLoaded(Vec<WebhookEndpoint>),
    WebhookLoaded(Box<WebhookEndpoint>),
    DeliveriesLoaded(Vec<WebhookDelivery>),
    EnvironmentsLoaded(Vec<Environment>),
    SchedulesLoaded(Vec<Schedule>),
    VariationsLoaded(Vec<Variation>),
    DashboardLoaded(DashboardData),

    // Mutations completed
    FlagCreated(Box<ManagedFlag>),
    FlagUpdated(Box<ManagedFlag>),
    FlagDeleted(String),
    FlagToggled,
    RolloutUpdated,
    RulesUpdated,
    VariationsUpdated(Vec<Variation>),
    VariationsDeleted,
    ScheduleCreated(Box<Schedule>),
    ScheduleCancelled(String),
    ConfigCreated(Box<ManagedConfig>),
    ConfigUpdated(Box<ManagedConfig>),
    ConfigDeleted(String),
    ConfigValueUpdated,
    AiConfigCreated(Box<ManagedAiConfig>),
    AiConfigUpdated(Box<ManagedAiConfig>),
    AiConfigDeleted(String),
    AiConfigsInitialized(Vec<ManagedAiConfig>),
    WebhookCreated(Box<WebhookEndpoint>),
    WebhookUpdated(Box<WebhookEndpoint>),
    WebhookDeleted(String),
    WebhookSecretRegenerated(Box<WebhookEndpoint>),
    WebhookReactivated(Box<WebhookEndpoint>),

    // UI
    Toast(ToastMessage),
    ShowConfirm(ConfirmAction),
    ConfirmAccepted,
    ConfirmDismissed,
    ApiError(String),
    Tick,
    SetLoading(bool),

    // Form submissions
    SubmitFlagCreate,
    SubmitFlagUpdate(String),    // original key
    SubmitFlagToggle(String),    // flag key
    SubmitRolloutUpdate(String), // flag key
    SubmitRulesUpdate(String),   // flag key
    SubmitConfigCreate,
    SubmitConfigUpdate(String),      // original key
    SubmitConfigValueUpdate(String), // config key
    SubmitAiConfigCreate,
    SubmitAiConfigUpdate(String), // original file_name
    SubmitWebhookCreate,
    SubmitWebhookUpdate(String), // original id

    // Login / Auth
    BrowserLoginRequested,
    DeviceAuthReceived(Box<DeviceAuthResponse>),
    DeviceTokenPollResult(Box<DeviceTokenResponse>),
    LoginSuccess,
    Logout,

    // Project picker
    ProjectsLoaded(Vec<Project>),
    PickerProjectChosen(String),
    PickerEnvironmentsLoaded(Vec<Environment>),
    ProjectSelected {
        project_id: String,
        environment_id: String,
        project_name: String,
        environment_name: String,
    },

    // Environment switcher
    SwitcherEnvironmentsLoaded(Vec<Environment>),
    EnvironmentSwitched {
        environment_id: String,
        environment_name: String,
    },
    EnvironmentSwitcherDismissed,
}

#[derive(Debug, Clone)]
pub struct ToastMessage {
    pub message: String,
    pub level: ToastLevel,
}

#[derive(Debug, Clone)]
pub enum ToastLevel {
    Success,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteFlag(String),
    DeleteConfig(String),
    DeleteAiConfig(String),
    DeleteWebhook(String),
    CancelSchedule {
        flag_key: String,
        schedule_id: String,
    },
    DeleteVariations(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SidebarSection {
    Dashboard,
    Flags,
    Configs,
    AiConfigs,
    Webhooks,
    Environments,
}

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Login,
    ProjectPicker,
    Dashboard,
    FlagList,
    FlagDetail(String),
    FlagCreate,
    FlagEdit(String),
    FlagToggle(String),
    FlagRollout(String),
    FlagRules(String),
    FlagVariations(String),
    FlagSchedules(String),
    ConfigList,
    ConfigDetail(String),
    ConfigCreate,
    ConfigEdit(String),
    ConfigValueEditor(String),
    AiConfigList,
    AiConfigDetail(String),
    AiConfigCreate,
    AiConfigEdit(String),
    WebhookList,
    WebhookDetail(String),
    WebhookCreate,
    WebhookEdit(String),
    EnvironmentList,
}

#[derive(Debug, Clone)]
pub struct DashboardFlag {
    pub key: String,
    pub flag_type: String,
    pub rollout: Option<i32>,
    pub value: String,
    pub enabled: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DashboardData {
    pub flag_count: usize,
    pub config_count: usize,
    pub webhook_count: usize,
    pub ai_config_count: usize,
    pub flag_subtitle: String,
    pub config_subtitle: String,
    pub ai_config_subtitle: String,
    pub webhook_subtitle: String,
    pub recent_flags: Vec<DashboardFlag>,
}
