use crate::action::{
    Action, ConfirmAction, DashboardData, DashboardFlag, SidebarSection, ToastLevel, ToastMessage,
    View,
};
use crate::api::client::ApiClient;
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::environment_switcher::EnvironmentSwitcher;
use crate::components::header::Header;
use crate::components::sidebar::Sidebar;
use crate::components::status_bar::StatusBar;
use crate::components::toast::Toast;
use crate::config::AppConfig;
use crate::event::Event;
use crate::views::ai_configs::{
    detail::AiConfigDetailView, form::AiConfigFormView, list::AiConfigListView,
};
use crate::views::configs::{
    detail::ConfigDetailView, form::ConfigFormView, list::ConfigListView,
    value_editor::ConfigValueEditorView,
};
use crate::views::dashboard::DashboardView;
use crate::views::environments::list::EnvironmentListView;
use crate::views::flags::{
    detail::FlagDetailView, form::FlagFormView, list::FlagListView, rollout::FlagRolloutView,
    rules::FlagRulesView, schedules::FlagSchedulesView, toggle::FlagToggleView,
    variations::FlagVariationsView,
};
use crate::views::login::LoginView;
use crate::views::project_picker::ProjectPickerView;
use crate::views::webhooks::{
    detail::WebhookDetailView, form::WebhookFormView, list::WebhookListView,
};
use anyhow::Result;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::Frame;
use std::collections::HashSet;
use tokio::sync::mpsc;

pub struct App {
    pub config: AppConfig,
    pub api: Option<ApiClient>,
    pub running: bool,

    // Layout components
    pub header: Header,
    pub sidebar: Sidebar,
    pub status_bar: StatusBar,
    pub toast: Toast,
    pub confirm: ConfirmDialog,
    pub env_switcher: EnvironmentSwitcher,

    // Current view
    pub current_view: View,
    pub pending_confirm: Option<ConfirmAction>,

    // View state
    pub project_picker: ProjectPickerView,
    pub login_view: LoginView,
    pub dashboard_view: DashboardView,
    pub flag_list: FlagListView,
    pub flag_detail: FlagDetailView,
    pub flag_form: Option<FlagFormView>,
    pub flag_toggle: Option<FlagToggleView>,
    pub flag_rollout: Option<FlagRolloutView>,
    pub flag_rules: Option<FlagRulesView>,
    pub flag_variations: Option<FlagVariationsView>,
    pub flag_schedules: Option<FlagSchedulesView>,
    pub config_list: ConfigListView,
    pub config_detail: ConfigDetailView,
    pub config_form: Option<ConfigFormView>,
    pub config_value_editor: Option<ConfigValueEditorView>,
    pub ai_config_list: AiConfigListView,
    pub ai_config_detail: AiConfigDetailView,
    pub ai_config_form: Option<AiConfigFormView>,
    pub webhook_list: WebhookListView,
    pub webhook_detail: WebhookDetailView,
    pub webhook_form: Option<WebhookFormView>,
    pub env_list: EnvironmentListView,

    // Async action channel
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub action_rx: mpsc::UnboundedReceiver<Action>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let key_tier = config.user_role_tier();
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let api = if config.has_session_token() {
            Some(ApiClient::new(
                &config.connection.base_url,
                &config.auth.session_token,
            ))
        } else {
            None
        };

        let mut app = Self {
            status_bar: StatusBar::new(&config.connection.base_url),
            config,
            api,
            running: true,
            header: Header::new(),
            sidebar: Sidebar::new(),
            toast: Toast::new(),
            confirm: ConfirmDialog::new(),
            env_switcher: EnvironmentSwitcher::new(),
            current_view: View::Login,
            pending_confirm: None,
            project_picker: ProjectPickerView::new(),
            login_view: LoginView::new(),
            dashboard_view: DashboardView::new(),
            flag_list: FlagListView::new(key_tier.clone()),
            flag_detail: FlagDetailView::new(key_tier.clone()),
            flag_form: None,
            flag_toggle: None,
            flag_rollout: None,
            flag_rules: None,
            flag_variations: None,
            flag_schedules: None,
            config_list: ConfigListView::new(key_tier.clone()),
            config_detail: ConfigDetailView::new(key_tier.clone()),
            config_form: None,
            config_value_editor: None,
            ai_config_list: AiConfigListView::new(key_tier.clone()),
            ai_config_detail: AiConfigDetailView::new(key_tier.clone()),
            ai_config_form: None,
            webhook_list: WebhookListView::new(key_tier.clone()),
            webhook_detail: WebhookDetailView::new(key_tier),
            webhook_form: None,
            env_list: EnvironmentListView::new(),
            action_tx,
            action_rx,
        };

        // Navigate to the correct initial view (triggers data loading)
        if app.config.has_session_token() {
            app.status_bar.connected = true;
            app.header.connected = true;
            app.header.project_name = app.config.defaults.project_name.clone();
            app.header.environment_name = app.config.defaults.environment_name.clone();
            // Always show project picker on startup (with saved defaults pre-selected)
            app.project_picker.set_saved_defaults(
                &app.config.defaults.project_id,
                &app.config.defaults.environment_id,
            );
            app.navigate(View::ProjectPicker);
        }

        app
    }

    pub fn handle_event(&mut self, event: &Event) -> Result<()> {
        // Tick: auto-dismiss toasts
        if matches!(event, Event::Tick) {
            self.toast.tick();
            return Ok(());
        }

        // Environment switcher overlay takes priority
        if self.env_switcher.is_visible() {
            if let Some(action) = self.env_switcher.handle_event(event) {
                self.process_action(action);
            }
            return Ok(());
        }

        // Confirm dialog takes priority
        if self.confirm.is_visible() {
            if let Some(action) = self.confirm.handle_event(event) {
                self.process_action(action);
            }
            return Ok(());
        }

        // Global quit
        if let Event::Key(key) = event {
            if key.kind == crossterm::event::KeyEventKind::Press {
                if key.code == crossterm::event::KeyCode::Char('q')
                    && !matches!(
                        self.current_view,
                        View::Login
                            | View::FlagCreate
                            | View::FlagEdit(_)
                            | View::FlagRules(_)
                            | View::ConfigCreate
                            | View::ConfigEdit(_)
                            | View::ConfigValueEditor(_)
                            | View::AiConfigCreate
                            | View::AiConfigEdit(_)
                            | View::WebhookCreate
                            | View::WebhookEdit(_)
                    )
                {
                    self.running = false;
                    return Ok(());
                }

                // Global 'e' for environment switcher, 'p' for project picker, 'l' for logout
                if self.is_main_view() && !self.is_searching() {
                    match key.code {
                        crossterm::event::KeyCode::Char('e') => {
                            self.open_environment_switcher();
                            return Ok(());
                        }
                        crossterm::event::KeyCode::Char('p') => {
                            self.project_picker.set_saved_defaults(
                                &self.config.defaults.project_id,
                                &self.config.defaults.environment_id,
                            );
                            self.navigate(View::ProjectPicker);
                            return Ok(());
                        }
                        crossterm::event::KeyCode::Char('l') => {
                            self.process_action(Action::Logout);
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Route to current view
        let action = match &self.current_view {
            View::Login => self.login_view.handle_event(event),
            View::ProjectPicker => self.project_picker.handle_event(event),
            View::Dashboard => {
                // Up/Down navigate recent flags; 1-6 handled by sidebar
                if let Event::Key(key) = event {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        match key.code {
                            crossterm::event::KeyCode::Down
                            | crossterm::event::KeyCode::Char('j') => {
                                self.dashboard_view.select_next();
                                return Ok(());
                            }
                            crossterm::event::KeyCode::Up
                            | crossterm::event::KeyCode::Char('k') => {
                                self.dashboard_view.select_prev();
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
                self.dashboard_view
                    .handle_event(event)
                    .or_else(|| self.sidebar.handle_event(event))
            }
            View::FlagList => self
                .flag_list
                .handle_event(event)
                .or_else(|| self.sidebar.handle_event(event)),
            View::FlagDetail(_) => self.flag_detail.handle_event(event),
            View::FlagCreate | View::FlagEdit(_) => {
                self.flag_form.as_mut().and_then(|f| f.handle_event(event))
            }
            View::FlagToggle(_) => self
                .flag_toggle
                .as_mut()
                .and_then(|v| v.handle_event(event)),
            View::FlagRollout(_) => self
                .flag_rollout
                .as_mut()
                .and_then(|v| v.handle_event(event)),
            View::FlagRules(_) => self.flag_rules.as_mut().and_then(|v| v.handle_event(event)),
            View::FlagVariations(_) => self
                .flag_variations
                .as_mut()
                .and_then(|v| v.handle_event(event)),
            View::FlagSchedules(_) => self
                .flag_schedules
                .as_mut()
                .and_then(|v| v.handle_event(event)),
            View::ConfigList => self
                .config_list
                .handle_event(event)
                .or_else(|| self.sidebar.handle_event(event)),
            View::ConfigDetail(_) => self.config_detail.handle_event(event),
            View::ConfigCreate | View::ConfigEdit(_) => self
                .config_form
                .as_mut()
                .and_then(|f| f.handle_event(event)),
            View::ConfigValueEditor(_) => self
                .config_value_editor
                .as_mut()
                .and_then(|v| v.handle_event(event)),
            View::AiConfigList => self
                .ai_config_list
                .handle_event(event)
                .or_else(|| self.sidebar.handle_event(event)),
            View::AiConfigDetail(_) => self.ai_config_detail.handle_event(event),
            View::AiConfigCreate | View::AiConfigEdit(_) => self
                .ai_config_form
                .as_mut()
                .and_then(|f| f.handle_event(event)),
            View::WebhookList => self
                .webhook_list
                .handle_event(event)
                .or_else(|| self.sidebar.handle_event(event)),
            View::WebhookDetail(_) => self.webhook_detail.handle_event(event),
            View::WebhookCreate | View::WebhookEdit(_) => self
                .webhook_form
                .as_mut()
                .and_then(|f| f.handle_event(event)),
            View::EnvironmentList => {
                self.env_list.handle_event(event);
                self.sidebar.handle_event(event)
            }
        };

        if let Some(action) = action {
            self.process_action(action);
        }

        Ok(())
    }

    pub fn process_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::Navigate(view) => self.navigate(view),
            Action::Back => self.go_back(),
            Action::SelectSection(section) => self.select_section(section),
            Action::Toast(msg) => self.toast.show(msg.message, msg.level),
            Action::ShowConfirm(confirm_action) => {
                self.pending_confirm = Some(confirm_action.clone());
                self.confirm.show(confirm_action);
            }
            Action::ConfirmAccepted => {
                if let Some(confirm_action) = self.pending_confirm.take() {
                    self.execute_confirm(confirm_action);
                }
            }
            Action::ConfirmDismissed => {
                self.pending_confirm = None;
            }
            Action::BrowserLoginRequested => self.handle_browser_login_requested(),
            Action::DeviceAuthReceived(device_auth) => {
                self.handle_device_auth_received(*device_auth);
            }
            Action::DeviceTokenPollResult(response) => {
                self.handle_device_token_poll_result(*response);
            }
            Action::LoginSuccess => {
                self.project_picker.set_saved_defaults(
                    &self.config.defaults.project_id,
                    &self.config.defaults.environment_id,
                );
                self.navigate(View::ProjectPicker);
            }
            Action::Logout => self.handle_logout(),
            Action::FlagsLoaded(flags) => self.flag_list.set_flags(flags),
            Action::ConfigsLoaded(configs) => self.config_list.set_configs(configs),
            Action::AiConfigsLoaded(configs) => self.ai_config_list.set_ai_configs(configs),
            Action::WebhooksLoaded(webhooks) => self.webhook_list.set_webhooks(webhooks),
            Action::EnvironmentsLoaded(envs) => {
                // Forward environments to sub-views that need them
                if let Some(v) = &mut self.flag_toggle {
                    v.environments = envs.clone();
                }
                if let Some(v) = &mut self.flag_rollout {
                    v.environments = envs.clone();
                }
                if let Some(v) = &mut self.flag_rules {
                    v.environments = envs.clone();
                }
                if let Some(v) = &mut self.flag_variations {
                    v.environments = envs.clone();
                }
                if let Some(v) = &mut self.flag_schedules {
                    v.environments = envs.clone();
                }
                if let Some(v) = &mut self.config_value_editor {
                    v.environments = envs.clone();
                }
                self.env_list.set_environments(envs);
            }
            Action::FlagLoaded(flag) => {
                if let Some(v) = &mut self.flag_toggle {
                    v.flag = Some((*flag).clone());
                }
                self.flag_detail.flag = Some(*flag);
            }
            Action::ConfigLoaded(config) => {
                self.config_detail.config = Some(*config);
            }
            Action::AiConfigLoaded(config) => {
                self.ai_config_detail.config = Some(*config);
            }
            Action::WebhookLoaded(webhook) => {
                self.webhook_detail.webhook = Some(*webhook);
            }
            Action::DeliveriesLoaded(deliveries) => {
                self.webhook_detail.deliveries = deliveries;
            }
            Action::SchedulesLoaded(schedules) => {
                if let Some(v) = &mut self.flag_schedules {
                    v.set_schedules(schedules);
                }
            }
            Action::VariationsLoaded(variations) => {
                if let Some(v) = &mut self.flag_variations {
                    v.set_variations(variations);
                }
            }
            Action::DashboardLoaded(data) => {
                self.dashboard_view.data = Some(data);
            }
            Action::SwitcherEnvironmentsLoaded(envs) => {
                self.env_switcher.set_environments(envs);
            }
            Action::EnvironmentSwitched {
                environment_id,
                environment_name,
            } => {
                self.config.defaults.environment_id = environment_id;
                self.config.defaults.environment_name = environment_name.clone();
                let _ = self.config.save();
                self.header.environment_name = environment_name.clone();
                self.toast.show(
                    format!("Switched to {}", environment_name),
                    ToastLevel::Success,
                );
                self.reload_current_view();
            }
            Action::EnvironmentSwitcherDismissed => {}
            Action::ProjectsLoaded(projects) => {
                self.project_picker.set_projects(projects);
            }
            Action::PickerProjectChosen(pid) => {
                self.load_picker_environments(pid);
            }
            Action::PickerEnvironmentsLoaded(envs) => {
                self.project_picker.set_environments(envs);
            }
            Action::ProjectSelected {
                project_id,
                environment_id,
                project_name,
                environment_name,
            } => {
                self.config.defaults.project_id = project_id;
                self.config.defaults.environment_id = environment_id;
                self.config.defaults.project_name = project_name.clone();
                self.config.defaults.environment_name = environment_name.clone();
                let _ = self.config.save();
                self.header.project_name = project_name;
                self.header.environment_name = environment_name;
                self.status_bar.connected = true;
                self.header.connected = true;
                self.navigate(View::Dashboard);
            }
            Action::SubmitFlagCreate => self.submit_flag_create(),
            Action::SubmitFlagUpdate(key) => self.submit_flag_update(key),
            Action::SubmitFlagToggle(key) => self.submit_flag_toggle(key),
            Action::SubmitRolloutUpdate(key) => self.submit_rollout_update(key),
            Action::SubmitRulesUpdate(key) => self.submit_rules_update(key),
            Action::SubmitConfigCreate => self.submit_config_create(),
            Action::SubmitConfigUpdate(key) => self.submit_config_update(key),
            Action::SubmitConfigValueUpdate(key) => self.submit_config_value_update(key),
            Action::SubmitAiConfigCreate => self.submit_ai_config_create(),
            Action::SubmitAiConfigUpdate(name) => self.submit_ai_config_update(name),
            Action::SubmitWebhookCreate => self.submit_webhook_create(),
            Action::SubmitWebhookUpdate(id) => self.submit_webhook_update(id),
            Action::FlagCreated(_) | Action::FlagUpdated(_) => {
                self.flag_form = None;
                self.navigate(View::FlagList);
            }
            Action::FlagDeleted(_) => {
                self.navigate(View::FlagList);
            }
            Action::FlagToggled | Action::RolloutUpdated => {
                // Reload flag detail after toggle/rollout change
                let key = match &self.current_view {
                    View::FlagToggle(k) | View::FlagRollout(k) => Some(k.clone()),
                    _ => None,
                };
                if let Some(k) = key {
                    self.navigate(View::FlagDetail(k));
                }
            }
            Action::RulesUpdated => {
                self.flag_rules = None;
                let key = match &self.current_view {
                    View::FlagRules(k) => Some(k.clone()),
                    _ => None,
                };
                if let Some(k) = key {
                    self.navigate(View::FlagDetail(k));
                }
            }
            Action::VariationsUpdated(_) | Action::VariationsDeleted => {
                let key = match &self.current_view {
                    View::FlagVariations(k) => Some(k.clone()),
                    _ => None,
                };
                if let Some(k) = key {
                    self.navigate(View::FlagDetail(k));
                }
            }
            Action::ScheduleCreated(_) | Action::ScheduleCancelled(_) => {
                // Reload schedules in place
                let key = match &self.current_view {
                    View::FlagSchedules(k) => Some(k.clone()),
                    _ => None,
                };
                if let Some(k) = key {
                    self.load_schedules(k);
                }
            }
            Action::ConfigCreated(_) | Action::ConfigUpdated(_) => {
                self.config_form = None;
                self.navigate(View::ConfigList);
            }
            Action::ConfigDeleted(_) => {
                self.navigate(View::ConfigList);
            }
            Action::ConfigValueUpdated => {
                self.config_value_editor = None;
                let key = match &self.current_view {
                    View::ConfigValueEditor(k) => Some(k.clone()),
                    _ => None,
                };
                if let Some(k) = key {
                    self.navigate(View::ConfigDetail(k));
                }
            }
            Action::AiConfigCreated(_) | Action::AiConfigUpdated(_) => {
                self.ai_config_form = None;
                self.navigate(View::AiConfigList);
            }
            Action::AiConfigDeleted(_) | Action::AiConfigsInitialized(_) => {
                self.navigate(View::AiConfigList);
            }
            Action::WebhookCreated(_) | Action::WebhookUpdated(_) => {
                self.webhook_form = None;
                self.navigate(View::WebhookList);
            }
            Action::WebhookDeleted(_) => {
                self.navigate(View::WebhookList);
            }
            Action::WebhookSecretRegenerated(webhook) | Action::WebhookReactivated(webhook) => {
                self.webhook_detail.webhook = Some(*webhook);
            }
            Action::ApiError(msg) => {
                self.toast.show(msg, ToastLevel::Error);
            }
            Action::SetLoading(loading) => {
                self.status_bar.loading = loading;
            }
            _ => {}
        }
    }

    fn navigate(&mut self, view: View) {
        self.current_view = view;
        // Trigger data loading for new views
        match &self.current_view {
            View::ProjectPicker => self.load_projects(),
            View::Dashboard => self.load_dashboard(),
            View::FlagList => self.load_flags(),
            View::FlagDetail(key) => self.load_flag(key.clone()),
            View::ConfigList => self.load_configs(),
            View::ConfigDetail(key) => self.load_config(key.clone()),
            View::AiConfigList => self.load_ai_configs(),
            View::AiConfigDetail(name) => self.load_ai_config(name.clone()),
            View::WebhookList => self.load_webhooks(),
            View::WebhookDetail(id) => self.load_webhook(id.clone()),
            View::EnvironmentList => self.load_environments(),
            View::FlagCreate => {
                self.flag_form = Some(FlagFormView::new_create(&self.config.defaults.project_id));
            }
            View::FlagEdit(_) => {
                if let Some(flag) = &self.flag_detail.flag {
                    self.flag_form = Some(FlagFormView::new_edit(
                        &self.config.defaults.project_id,
                        flag,
                    ));
                }
            }
            View::ConfigCreate => {
                self.config_form =
                    Some(ConfigFormView::new_create(&self.config.defaults.project_id));
            }
            View::ConfigEdit(_) => {
                if let Some(config) = &self.config_detail.config {
                    self.config_form = Some(ConfigFormView::new_edit(
                        &self.config.defaults.project_id,
                        config,
                    ));
                }
            }
            View::AiConfigCreate => {
                self.ai_config_form = Some(AiConfigFormView::new_create(
                    &self.config.defaults.project_id,
                    &self.config.defaults.environment_id,
                ));
            }
            View::AiConfigEdit(_) => {
                if let Some(config) = &self.ai_config_detail.config {
                    self.ai_config_form = Some(AiConfigFormView::new_edit(
                        &self.config.defaults.project_id,
                        &self.config.defaults.environment_id,
                        config,
                    ));
                }
            }
            View::WebhookCreate => {
                self.webhook_form = Some(WebhookFormView::new_create(
                    &self.config.defaults.project_id,
                    &self.config.defaults.environment_id,
                ));
            }
            View::WebhookEdit(_) => {
                if let Some(webhook) = &self.webhook_detail.webhook {
                    self.webhook_form = Some(WebhookFormView::new_edit(
                        &self.config.defaults.project_id,
                        &self.config.defaults.environment_id,
                        webhook,
                    ));
                }
            }
            View::FlagToggle(key) => {
                self.flag_toggle = Some(FlagToggleView::new(key));
                self.load_flag(key.clone());
                self.load_environments();
            }
            View::FlagRollout(key) => {
                self.flag_rollout = Some(FlagRolloutView::new(key));
                self.load_environments();
            }
            View::FlagRules(key) => {
                self.flag_rules = Some(FlagRulesView::new(key));
                self.load_environments();
            }
            View::FlagVariations(key) => {
                self.flag_variations = Some(FlagVariationsView::new(key));
                self.load_environments();
            }
            View::FlagSchedules(key) => {
                self.flag_schedules = Some(FlagSchedulesView::new(key));
                self.load_environments();
            }
            View::ConfigValueEditor(key) => {
                let mut editor = ConfigValueEditorView::new(key);
                // Pre-fill with current config value from the first environment
                if let Some(config) = &self.config_detail.config {
                    if let Some(env) = config.environments.first() {
                        editor.set_value(&env.value);
                    } else {
                        editor.set_value(&config.default_value);
                    }
                }
                self.config_value_editor = Some(editor);
                self.load_environments();
            }
            _ => {}
        }
    }

    fn go_back(&mut self) {
        let back_view = match &self.current_view {
            View::FlagDetail(_) | View::FlagCreate | View::FlagEdit(_) => View::FlagList,
            View::FlagToggle(k)
            | View::FlagRollout(k)
            | View::FlagRules(k)
            | View::FlagVariations(k)
            | View::FlagSchedules(k) => View::FlagDetail(k.clone()),
            View::ConfigDetail(_) | View::ConfigCreate | View::ConfigEdit(_) => View::ConfigList,
            View::ConfigValueEditor(k) => View::ConfigDetail(k.clone()),
            View::AiConfigDetail(_) | View::AiConfigCreate | View::AiConfigEdit(_) => {
                View::AiConfigList
            }
            View::WebhookDetail(_) | View::WebhookCreate | View::WebhookEdit(_) => {
                View::WebhookList
            }
            _ => View::Dashboard,
        };
        self.navigate(back_view);
    }

    fn select_section(&mut self, section: SidebarSection) {
        let view = match section {
            SidebarSection::Dashboard => View::Dashboard,
            SidebarSection::Flags => View::FlagList,
            SidebarSection::Configs => View::ConfigList,
            SidebarSection::AiConfigs => View::AiConfigList,
            SidebarSection::Webhooks => View::WebhookList,
            SidebarSection::Environments => View::EnvironmentList,
        };
        self.navigate(view);
    }

    fn handle_browser_login_requested(&mut self) {
        let base_url = self.config.connection.base_url.clone();
        let tx = self.action_tx.clone();
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "FlagDash CLI".to_string());

        tokio::spawn(async move {
            let client = ApiClient::new_unauthenticated(&base_url);
            match client.request_device_auth(Some(&hostname)).await {
                Ok(resp) => {
                    let _ = tx.send(Action::DeviceAuthReceived(Box::new(resp)));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(format!("Failed to start login: {}", e)));
                }
            }
        });
    }

    fn handle_device_auth_received(&mut self, device_auth: crate::api::types::DeviceAuthResponse) {
        // Update the login view
        self.login_view.set_waiting(&device_auth);

        // Open the browser
        let _ = open::that(&device_auth.verification_url);

        // Start polling for the token
        let base_url = self.config.connection.base_url.clone();
        let device_code = device_auth.device_code.clone();
        let interval = device_auth.interval;
        let expires_in = device_auth.expires_in;
        let tx = self.action_tx.clone();

        tokio::spawn(async move {
            let client = ApiClient::new_unauthenticated(&base_url);
            let max_polls = expires_in / interval.max(1);
            let sleep_duration = std::time::Duration::from_secs(interval.max(2));

            for _ in 0..max_polls {
                tokio::time::sleep(sleep_duration).await;
                match client.poll_device_token(&device_code).await {
                    Ok(resp) => {
                        let _ = tx.send(Action::DeviceTokenPollResult(Box::new(resp.clone())));
                        // If we got a token or a terminal error, stop polling
                        if resp.session_token.is_some() {
                            return;
                        }
                        if let Some(err) = &resp.error {
                            if err != "authorization_pending" && err != "slow_down" {
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ApiError(format!("Poll error: {}", e)));
                        return;
                    }
                }
            }
            // Expired
            let _ = tx.send(Action::DeviceTokenPollResult(Box::new(
                crate::api::types::DeviceTokenResponse {
                    session_token: None,
                    account: None,
                    user: None,
                    expires_at: None,
                    error: Some("expired_token".to_string()),
                },
            )));
        });
    }

    fn handle_device_token_poll_result(
        &mut self,
        response: crate::api::types::DeviceTokenResponse,
    ) {
        if let Some(token) = response.session_token {
            // Success! Store the session token and user info
            self.config.auth.session_token = token;

            if let Some(user) = &response.user {
                self.config.auth.user_name = user.name.clone();
                self.config.auth.user_email = user.email.clone();
                self.config.auth.user_role = user.role.clone();
            }

            if let Some(expires_at) = &response.expires_at {
                self.config.auth.token_expires_at = expires_at.clone();
            }

            let _ = self.config.save();

            // Set up the API client with the new session token
            self.api = Some(ApiClient::new(
                &self.config.connection.base_url,
                &self.config.auth.session_token,
            ));

            // Update key tier for views
            let key_tier = self.config.user_role_tier();
            self.flag_list.key_tier = key_tier.clone();
            self.flag_detail.key_tier = key_tier.clone();
            self.config_list.key_tier = key_tier.clone();
            self.config_detail.key_tier = key_tier.clone();
            self.ai_config_list.key_tier = key_tier.clone();
            self.ai_config_detail.key_tier = key_tier.clone();
            self.webhook_list.key_tier = key_tier.clone();
            self.webhook_detail.key_tier = key_tier;

            self.status_bar.connected = true;
            self.header.connected = true;
            self.login_view.set_success();

            self.process_action(Action::LoginSuccess);
        } else if let Some(err) = &response.error {
            match err.as_str() {
                "authorization_pending" | "slow_down" => {
                    // Still waiting, do nothing (polling continues)
                }
                "expired_token" => {
                    self.login_view
                        .set_error("Login expired. Press Enter to try again.");
                }
                "access_denied" => {
                    self.login_view
                        .set_error("Login denied. Press Enter to try again.");
                }
                _ => {
                    self.login_view
                        .set_error(&format!("Login error: {}. Press Enter to retry.", err));
                }
            }
        }
    }

    fn handle_logout(&mut self) {
        self.config.clear_auth();
        let _ = self.config.save();
        self.api = None;
        self.status_bar.connected = false;
        self.header.connected = false;
        self.header.project_name.clear();
        self.header.environment_name.clear();
        self.login_view = LoginView::new();
        self.toast.show("Logged out".to_string(), ToastLevel::Info);
        self.current_view = View::Login;
    }

    fn execute_confirm(&mut self, action: ConfirmAction) {
        let api = match &self.api {
            Some(a) => a.clone(),
            None => return,
        };
        let project_id = self.config.defaults.project_id.clone();
        let env_id = self.config.defaults.environment_id.clone();
        let tx = self.action_tx.clone();

        match action {
            ConfirmAction::DeleteFlag(key) => {
                tokio::spawn(async move {
                    match api.delete_flag(&key, &project_id).await {
                        Ok(()) => {
                            let _ = tx.send(Action::FlagDeleted(key));
                            let _ = tx.send(Action::Toast(ToastMessage {
                                message: "Flag deleted".to_string(),
                                level: ToastLevel::Success,
                            }));
                        }
                        Err(e) => {
                            let _ = tx.send(Action::ApiError(e.to_string()));
                        }
                    }
                });
            }
            ConfirmAction::DeleteConfig(key) => {
                tokio::spawn(async move {
                    match api.delete_config(&key, &project_id).await {
                        Ok(()) => {
                            let _ = tx.send(Action::ConfigDeleted(key));
                            let _ = tx.send(Action::Toast(ToastMessage {
                                message: "Config deleted".to_string(),
                                level: ToastLevel::Success,
                            }));
                        }
                        Err(e) => {
                            let _ = tx.send(Action::ApiError(e.to_string()));
                        }
                    }
                });
            }
            ConfirmAction::DeleteAiConfig(name) => {
                tokio::spawn(async move {
                    match api.delete_ai_config(&name, &project_id, &env_id).await {
                        Ok(()) => {
                            let _ = tx.send(Action::AiConfigDeleted(name));
                            let _ = tx.send(Action::Toast(ToastMessage {
                                message: "AI config deleted".to_string(),
                                level: ToastLevel::Success,
                            }));
                        }
                        Err(e) => {
                            let _ = tx.send(Action::ApiError(e.to_string()));
                        }
                    }
                });
            }
            ConfirmAction::DeleteWebhook(id) => {
                tokio::spawn(async move {
                    match api.delete_webhook(&id).await {
                        Ok(()) => {
                            let _ = tx.send(Action::WebhookDeleted(id));
                            let _ = tx.send(Action::Toast(ToastMessage {
                                message: "Webhook deleted".to_string(),
                                level: ToastLevel::Success,
                            }));
                        }
                        Err(e) => {
                            let _ = tx.send(Action::ApiError(e.to_string()));
                        }
                    }
                });
            }
            ConfirmAction::CancelSchedule {
                flag_key,
                schedule_id,
            } => {
                tokio::spawn(async move {
                    match api
                        .cancel_schedule(&flag_key, &project_id, &schedule_id)
                        .await
                    {
                        Ok(()) => {
                            let _ = tx.send(Action::ScheduleCancelled(schedule_id));
                            let _ = tx.send(Action::Toast(ToastMessage {
                                message: "Schedule cancelled".to_string(),
                                level: ToastLevel::Success,
                            }));
                        }
                        Err(e) => {
                            let _ = tx.send(Action::ApiError(e.to_string()));
                        }
                    }
                });
            }
            ConfirmAction::DeleteVariations(key) => {
                let env_id2 = env_id;
                tokio::spawn(async move {
                    match api.delete_variations(&key, &project_id, &env_id2).await {
                        Ok(()) => {
                            let _ = tx.send(Action::VariationsDeleted);
                            let _ = tx.send(Action::Toast(ToastMessage {
                                message: "Variations deleted".to_string(),
                                level: ToastLevel::Success,
                            }));
                        }
                        Err(e) => {
                            let _ = tx.send(Action::ApiError(e.to_string()));
                        }
                    }
                });
            }
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────

    fn is_main_view(&self) -> bool {
        matches!(
            self.current_view,
            View::Dashboard
                | View::FlagList
                | View::FlagDetail(_)
                | View::ConfigList
                | View::ConfigDetail(_)
                | View::AiConfigList
                | View::AiConfigDetail(_)
                | View::WebhookList
                | View::WebhookDetail(_)
                | View::EnvironmentList
        )
    }

    fn is_searching(&self) -> bool {
        self.flag_list.search.active
            || self.config_list.search.active
            || self.ai_config_list.search.active
    }

    fn open_environment_switcher(&mut self) {
        self.env_switcher.show(&self.config.defaults.environment_id);
        // Fetch environments for the current project
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.list_environments(&project_id).await {
                Ok(envs) => {
                    let _ = tx.send(Action::SwitcherEnvironmentsLoaded(envs));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn reload_current_view(&mut self) {
        match &self.current_view {
            View::Dashboard => self.load_dashboard(),
            View::FlagList => self.load_flags(),
            View::FlagDetail(key) => self.load_flag(key.clone()),
            View::ConfigList => self.load_configs(),
            View::ConfigDetail(key) => self.load_config(key.clone()),
            View::AiConfigList => self.load_ai_configs(),
            View::AiConfigDetail(name) => self.load_ai_config(name.clone()),
            View::WebhookList => self.load_webhooks(),
            View::WebhookDetail(id) => self.load_webhook(id.clone()),
            View::EnvironmentList => self.load_environments(),
            _ => {}
        }
    }

    // ── Data loading ─────────────────────────────────────────────────

    fn load_projects(&self) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.list_projects().await {
                Ok(projects) => {
                    let _ = tx.send(Action::ProjectsLoaded(projects));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_picker_environments(&self, project_id: String) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.list_environments(&project_id).await {
                Ok(envs) => {
                    let _ = tx.send(Action::PickerEnvironmentsLoaded(envs));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_dashboard(&self) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let env_id = self.config.defaults.environment_id.clone();
        let tx = self.action_tx.clone();

        tokio::spawn(async move {
            if project_id.is_empty() {
                let _ = tx.send(Action::ApiError("No project selected".into()));
                let _ = tx.send(Action::SetLoading(false));
                return;
            }

            let flags = api.list_flags(&project_id).await.unwrap_or_default();
            let configs = api.list_configs(&project_id).await.unwrap_or_default();
            let webhooks = api.list_webhooks(&project_id).await.unwrap_or_default();
            let ai_configs = api
                .list_ai_configs(&project_id, &env_id)
                .await
                .unwrap_or_default();

            // Compute subtitles
            let active_flags = flags
                .iter()
                .filter(|f| f.environments.iter().any(|e| e.enabled))
                .count();
            let active_configs = configs
                .iter()
                .filter(|c| {
                    !c.environments.is_empty() && c.environments.iter().all(|e| e.is_active)
                })
                .count();
            let active_webhooks = webhooks.iter().filter(|w| w.is_active).count();
            let ai_env_set: HashSet<&str> = ai_configs
                .iter()
                .map(|a| a.environment_id.as_str())
                .collect();
            let ai_env_count = ai_env_set.len();

            let flag_subtitle = if flags.is_empty() {
                "no flags".to_string()
            } else {
                format!("{} active", active_flags)
            };
            let config_subtitle = if configs.is_empty() || active_configs == configs.len() {
                "all active".to_string()
            } else {
                format!("{} active", active_configs)
            };
            let ai_config_subtitle = format!("{} env", ai_env_count);
            let webhook_subtitle = if !webhooks.is_empty() && active_webhooks == webhooks.len() {
                "all healthy".to_string()
            } else {
                format!("{} active", active_webhooks)
            };

            // Recent flags for dashboard table (up to 8, most recently updated)
            let mut sorted_flags = flags.clone();
            sorted_flags.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            let recent_flags: Vec<DashboardFlag> = sorted_flags
                .iter()
                .take(8)
                .map(|f| {
                    let rollout = f.environments.first().map(|e| e.rollout_percentage);
                    let enabled = f.environments.iter().any(|e| e.enabled);
                    let value = format_json_value(&f.default_value);
                    DashboardFlag {
                        key: f.key.clone(),
                        flag_type: f.flag_type.clone(),
                        rollout,
                        value,
                        enabled,
                        updated_at: f.updated_at,
                    }
                })
                .collect();

            let _ = tx.send(Action::DashboardLoaded(DashboardData {
                flag_count: flags.len(),
                config_count: configs.len(),
                webhook_count: webhooks.len(),
                ai_config_count: ai_configs.len(),
                flag_subtitle,
                config_subtitle,
                ai_config_subtitle,
                webhook_subtitle,
                recent_flags,
            }));
            let _ = tx.send(Action::SetLoading(false));
        });
    }

    fn load_flags(&self) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.list_flags(&project_id).await {
                Ok(flags) => {
                    let _ = tx.send(Action::FlagsLoaded(flags));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_flag(&self, key: String) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.get_flag(&key, &project_id).await {
                Ok(flag) => {
                    let _ = tx.send(Action::FlagLoaded(Box::new(flag)));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_configs(&self) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.list_configs(&project_id).await {
                Ok(configs) => {
                    let _ = tx.send(Action::ConfigsLoaded(configs));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_config(&self, key: String) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.get_config(&key, &project_id).await {
                Ok(config) => {
                    let _ = tx.send(Action::ConfigLoaded(Box::new(config)));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_ai_configs(&self) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let env_id = self.config.defaults.environment_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.list_ai_configs(&project_id, &env_id).await {
                Ok(configs) => {
                    let _ = tx.send(Action::AiConfigsLoaded(configs));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_ai_config(&self, name: String) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let env_id = self.config.defaults.environment_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.get_ai_config(&name, &project_id, &env_id).await {
                Ok(config) => {
                    let _ = tx.send(Action::AiConfigLoaded(Box::new(config)));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_webhooks(&self) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.list_webhooks(&project_id).await {
                Ok(webhooks) => {
                    let _ = tx.send(Action::WebhooksLoaded(webhooks));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_webhook(&self, id: String) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let tx = self.action_tx.clone();
        let id2 = id.clone();
        tokio::spawn(async move {
            match api.get_webhook(&id).await {
                Ok(webhook) => {
                    let _ = tx.send(Action::WebhookLoaded(Box::new(webhook)));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
            if let Ok(deliveries) = api.list_webhook_deliveries(&id2, 50, 0).await {
                let _ = tx.send(Action::DeliveriesLoaded(deliveries));
            }
        });
    }

    fn load_schedules(&self, flag_key: String) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let env_id = self.config.defaults.environment_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.list_schedules(&flag_key, &project_id, &env_id).await {
                Ok(schedules) => {
                    let _ = tx.send(Action::SchedulesLoaded(schedules));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_environments(&self) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.list_environments(&project_id).await {
                Ok(envs) => {
                    let _ = tx.send(Action::EnvironmentsLoaded(envs));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    // ── Form submissions ──────────────────────────────────────────────

    fn submit_flag_create(&mut self) {
        let Some(form) = &self.flag_form else { return };
        let Some(api) = &self.api else { return };
        let req = form.create_request();
        let api = api.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.create_flag(&req).await {
                Ok(flag) => {
                    let _ = tx.send(Action::FlagCreated(Box::new(flag)));
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Flag created".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_flag_update(&mut self, key: String) {
        let Some(form) = &self.flag_form else { return };
        let Some(api) = &self.api else { return };
        let req = form.update_request();
        let project_id = self.config.defaults.project_id.clone();
        let api = api.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.update_flag(&key, &project_id, &req).await {
                Ok(flag) => {
                    let _ = tx.send(Action::FlagUpdated(Box::new(flag)));
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Flag updated".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_config_create(&mut self) {
        let Some(form) = &self.config_form else {
            return;
        };
        let Some(api) = &self.api else { return };
        let req = form.create_request();
        let api = api.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.create_config(&req).await {
                Ok(config) => {
                    let _ = tx.send(Action::ConfigCreated(Box::new(config)));
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Config created".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_config_update(&mut self, key: String) {
        let Some(form) = &self.config_form else {
            return;
        };
        let Some(api) = &self.api else { return };
        let req = form.update_request();
        let project_id = self.config.defaults.project_id.clone();
        let api = api.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.update_config(&key, &project_id, &req).await {
                Ok(config) => {
                    let _ = tx.send(Action::ConfigUpdated(Box::new(config)));
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Config updated".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_ai_config_create(&mut self) {
        let Some(form) = &self.ai_config_form else {
            return;
        };
        let Some(api) = &self.api else { return };
        let req = form.create_request();
        let api = api.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.create_ai_config(&req).await {
                Ok(config) => {
                    let _ = tx.send(Action::AiConfigCreated(Box::new(config)));
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "AI config created".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_ai_config_update(&mut self, name: String) {
        let Some(form) = &self.ai_config_form else {
            return;
        };
        let Some(api) = &self.api else { return };
        let req = form.update_request();
        let project_id = self.config.defaults.project_id.clone();
        let env_id = self.config.defaults.environment_id.clone();
        let api = api.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api
                .update_ai_config(&name, &project_id, &env_id, &req)
                .await
            {
                Ok(config) => {
                    let _ = tx.send(Action::AiConfigUpdated(Box::new(config)));
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "AI config updated".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_webhook_create(&mut self) {
        let Some(form) = &self.webhook_form else {
            return;
        };
        let Some(api) = &self.api else { return };
        let req = form.create_request();
        let api = api.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.create_webhook(&req).await {
                Ok(webhook) => {
                    let _ = tx.send(Action::WebhookCreated(Box::new(webhook)));
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Webhook created".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_webhook_update(&mut self, id: String) {
        let Some(form) = &self.webhook_form else {
            return;
        };
        let Some(api) = &self.api else { return };
        let req = form.update_request();
        let api = api.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.update_webhook(&id, &req).await {
                Ok(webhook) => {
                    let _ = tx.send(Action::WebhookUpdated(Box::new(webhook)));
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Webhook updated".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_flag_toggle(&mut self, key: String) {
        let Some(view) = &self.flag_toggle else {
            return;
        };
        let Some(env_id) = view.selected_environment_id() else {
            return;
        };
        let env_id = env_id.to_string();
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.toggle_flag(&key, &project_id, &env_id).await {
                Ok(_) => {
                    let _ = tx.send(Action::FlagToggled);
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Flag toggled".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_rollout_update(&mut self, key: String) {
        let Some(view) = &self.flag_rollout else {
            return;
        };
        let Some(env_id) = view.selected_environment_id() else {
            return;
        };
        let env_id = env_id.to_string();
        let percentage = view.percentage;
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api
                .set_rollout(&key, &project_id, &env_id, percentage)
                .await
            {
                Ok(_) => {
                    let _ = tx.send(Action::RolloutUpdated);
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: format!("Rollout set to {}%", percentage),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_rules_update(&mut self, key: String) {
        let Some(view) = &self.flag_rules else { return };
        let Some(env_id) = view.selected_environment_id() else {
            return;
        };
        let env_id = env_id.to_string();
        let rules = match view.parse_rules() {
            Ok(r) => r,
            Err(e) => {
                self.toast
                    .show(format!("Invalid JSON: {}", e), ToastLevel::Error);
                return;
            }
        };
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.update_rules(&key, &project_id, &env_id, rules).await {
                Ok(_) => {
                    let _ = tx.send(Action::RulesUpdated);
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Rules updated".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_config_value_update(&mut self, key: String) {
        let Some(view) = &self.config_value_editor else {
            return;
        };
        let Some(env_id) = view.selected_environment_id() else {
            return;
        };
        let env_id = env_id.to_string();
        let value = match view.parse_value() {
            Ok(v) => v,
            Err(e) => {
                self.toast
                    .show(format!("Invalid JSON: {}", e), ToastLevel::Error);
                return;
            }
        };
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api
                .set_config_value(&key, &project_id, &env_id, value)
                .await
            {
                Ok(_) => {
                    let _ = tx.send(Action::ConfigValueUpdated);
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Config value updated".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    // ── Rendering ────────────────────────────────────────────────────

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if matches!(self.current_view, View::Login) {
            self.login_view.render(frame, area);
            return;
        }

        if matches!(self.current_view, View::ProjectPicker) {
            // Render with header + status bar but no sidebar
            let main_chunks = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(area);

            self.header.render(frame, main_chunks[0]);
            self.status_bar.render(frame, main_chunks[2]);
            self.project_picker.render(frame, main_chunks[1]);
            self.toast.render(frame, area);
            return;
        }

        // Main layout: header (2) | tab bar (2) | content | status bar (2)
        let main_chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);

        self.header.render(frame, main_chunks[0]);
        self.sidebar.render(frame, main_chunks[1]);
        self.status_bar.render(frame, main_chunks[3]);

        let content_area = Rect {
            x: main_chunks[2].x + 1,
            y: main_chunks[2].y,
            width: main_chunks[2].width.saturating_sub(2),
            height: main_chunks[2].height,
        };

        self.render_view(frame, content_area);

        // Overlays
        self.toast.render(frame, area);
        self.confirm.render(frame, area);
        self.env_switcher.render(frame, area);
    }

    fn render_view(&mut self, frame: &mut Frame, area: Rect) {
        match &self.current_view {
            View::Dashboard => self.dashboard_view.render(frame, area),
            View::FlagList => self.flag_list.render(frame, area),
            View::FlagDetail(_) => self.flag_detail.render(frame, area),
            View::FlagCreate | View::FlagEdit(_) => {
                if let Some(f) = &self.flag_form {
                    f.render(frame, area);
                }
            }
            View::FlagToggle(_) => {
                if let Some(v) = &mut self.flag_toggle {
                    v.render(frame, area);
                }
            }
            View::FlagRollout(_) => {
                if let Some(v) = &self.flag_rollout {
                    v.render(frame, area);
                }
            }
            View::FlagRules(_) => {
                if let Some(v) = &self.flag_rules {
                    v.render(frame, area);
                }
            }
            View::FlagVariations(_) => {
                if let Some(v) = &mut self.flag_variations {
                    v.render(frame, area);
                }
            }
            View::FlagSchedules(_) => {
                if let Some(v) = &mut self.flag_schedules {
                    v.render(frame, area);
                }
            }
            View::ConfigList => self.config_list.render(frame, area),
            View::ConfigDetail(_) => self.config_detail.render(frame, area),
            View::ConfigCreate | View::ConfigEdit(_) => {
                if let Some(f) = &self.config_form {
                    f.render(frame, area);
                }
            }
            View::ConfigValueEditor(_) => {
                if let Some(v) = &self.config_value_editor {
                    v.render(frame, area);
                }
            }
            View::AiConfigList => self.ai_config_list.render(frame, area),
            View::AiConfigDetail(_) => self.ai_config_detail.render(frame, area),
            View::AiConfigCreate | View::AiConfigEdit(_) => {
                if let Some(f) = &self.ai_config_form {
                    f.render(frame, area);
                }
            }
            View::WebhookList => self.webhook_list.render(frame, area),
            View::WebhookDetail(_) => self.webhook_detail.render(frame, area),
            View::WebhookCreate | View::WebhookEdit(_) => {
                if let Some(f) = &self.webhook_form {
                    f.render(frame, area);
                }
            }
            View::EnvironmentList => self.env_list.render(frame, area),
            View::Login | View::ProjectPicker => {} // handled above
        }
    }
}

fn format_json_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("\"{}\"", s),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Object(map) => {
            // FlagDash wraps values as {"value": <actual>}
            if let Some(inner) = map.get("value") {
                format_json_value(inner)
            } else if map.len() == 1 {
                format_json_value(map.values().next().unwrap())
            } else {
                v.to_string()
            }
        }
        serde_json::Value::Array(_) => v.to_string(),
    }
}
