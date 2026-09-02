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
use crate::views::experiments::{
    detail::ExperimentDetailView, form::ExperimentFormView, list::ExperimentListView,
};
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
    pub experiment_list: ExperimentListView,
    pub experiment_detail: ExperimentDetailView,
    pub experiment_form: Option<ExperimentFormView>,
    pub webhook_list: WebhookListView,
    pub webhook_detail: WebhookDetailView,
    pub webhook_form: Option<WebhookFormView>,
    pub env_list: EnvironmentListView,

    // Async action channel
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub action_rx: mpsc::UnboundedReceiver<Action>,

    // Handle to the in-flight device-authorization polling task, if any.
    // Stored so it can be aborted on retry/logout/success — otherwise repeated
    // login attempts stack concurrent polling loops that run until expiry.
    pub device_poll_task: Option<tokio::task::JoinHandle<()>>,
    // Presence doubles as the "a refresh is already in flight" guard: the tick
    // handler fires every frame, and without it an expired token would spawn a
    // refresh per tick.
    pub refresh_task: Option<tokio::task::JoinHandle<()>>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let key_tier = config.user_role_tier();
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        // `bearer_token()` returns None when an OAuth access token is present but
        // expired: the refresh below has to run before any request, and falling
        // back to a stale session token here would act with wider scopes than the
        // person most recently granted.
        let api = config
            .bearer_token()
            .map(|token| ApiClient::new(&config.connection.base_url, token));

        let needs_refresh = api.is_none() && !config.auth.refresh_token.is_empty();

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
            experiment_list: ExperimentListView::new(key_tier.clone()),
            experiment_detail: ExperimentDetailView::new(key_tier.clone()),
            experiment_form: None,
            webhook_list: WebhookListView::new(key_tier.clone()),
            webhook_detail: WebhookDetailView::new(key_tier),
            webhook_form: None,
            env_list: EnvironmentListView::new(),
            action_tx,
            action_rx,
            device_poll_task: None,
            refresh_task: None,
        };

        // An hour of inactivity leaves a live refresh token and a dead access
        // token, which is the normal resting state — restore it rather than
        // presenting a login screen to someone who is already signed in.
        if needs_refresh {
            app.login_view.set_restoring();
            app.process_action(Action::OAuthRefreshRequested);
            return app;
        }

        // Navigate to the correct initial view (triggers data loading)
        if app.api.is_some() {
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
            // Refresh ahead of expiry rather than discovering it as a 401 on
            // whatever the person was doing. `can_refresh` also guards against
            // stacking a second refresh while one is in flight.
            if self.config.access_token_expired() && self.can_refresh() {
                self.process_action(Action::OAuthRefreshRequested);
            }
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
                            | View::ExperimentCreate
                            | View::ExperimentEdit(_)
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
            View::ExperimentList => self
                .experiment_list
                .handle_event(event)
                .or_else(|| self.sidebar.handle_event(event)),
            View::ExperimentDetail(_) => self.experiment_detail.handle_event(event),
            View::ExperimentCreate | View::ExperimentEdit(_) => self
                .experiment_form
                .as_mut()
                .and_then(|form| form.handle_event(event)),
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
            Action::OAuthClientRegistered(client_id) => {
                self.config.auth.client_id = client_id;
                let _ = self.config.save();
            }
            Action::OAuthDeviceAuthReceived(device_auth) => {
                self.handle_oauth_device_auth_received(*device_auth);
            }
            Action::OAuthDevicePollResult(outcome) => {
                self.handle_oauth_device_poll_result(*outcome);
            }
            Action::IdentityResolved(identity) => {
                self.handle_identity_resolved(*identity);
            }
            Action::OAuthRefreshRequested => self.handle_oauth_refresh_requested(),
            Action::OAuthRefreshResult(outcome) => {
                self.handle_oauth_refresh_result(*outcome);
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
            Action::ExperimentsLoaded(experiments) => {
                self.experiment_list.set_experiments(experiments)
            }
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
            Action::ExperimentLoaded(experiment) => {
                self.experiment_detail.experiment = Some(*experiment);
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
            Action::SubmitExperimentCreate => self.submit_experiment_create(),
            Action::SubmitExperimentUpdate(key) => self.submit_experiment_update(key),
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
            Action::ExperimentCreated(_) | Action::ExperimentUpdated(_) => {
                self.experiment_form = None;
                self.navigate(View::ExperimentList);
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
            Action::ApiError(ref msg) => {
                // A 401 mid-session usually means the access token lapsed between
                // the expiry check and the request. Refreshing beats telling
                // someone who is signed in that they are not.
                if msg.contains("Unauthorized") && self.can_refresh() {
                    self.process_action(Action::OAuthRefreshRequested);
                    self.toast
                        .show("Refreshing your session…".to_string(), ToastLevel::Info);
                } else {
                    if matches!(self.current_view, View::Login) {
                        self.login_view.set_error(msg);
                    }
                    self.toast.show(msg.clone(), ToastLevel::Error);
                }
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
            View::ExperimentList => self.load_experiments(),
            View::ExperimentDetail(key) => self.load_experiment(key.clone()),
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
            View::ExperimentCreate => {
                self.experiment_form = Some(ExperimentFormView::new_create());
            }
            View::ExperimentEdit(_) => {
                if let Some(experiment) = &self.experiment_detail.experiment {
                    self.experiment_form = Some(ExperimentFormView::new_edit(experiment));
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
            View::ExperimentDetail(_) | View::ExperimentCreate | View::ExperimentEdit(_) => {
                View::ExperimentList
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
            SidebarSection::Experiments => View::ExperimentList,
            SidebarSection::Webhooks => View::WebhookList,
            SidebarSection::Environments => View::EnvironmentList,
        };
        self.navigate(view);
    }

    /// Begin an OAuth 2.1 device-grant login (RFC 8628).
    ///
    /// Two round trips before anything is shown: this installation registers
    /// itself as a public client the first time (RFC 7591, cached in the config
    /// thereafter), then asks for a device code. Registering per-login instead
    /// would leave an orphan client row behind on every sign-in.
    fn handle_browser_login_requested(&mut self) {
        let base_url = self.config.connection.base_url.clone();
        let existing_client_id = self.config.auth.client_id.clone();
        let tx = self.action_tx.clone();
        let hostname = Self::device_name();

        tokio::spawn(async move {
            let client = ApiClient::new_unauthenticated(&base_url);

            let client_id =
                match Self::ensure_client_id(&client, existing_client_id, &hostname).await {
                    Ok(id) => id,
                    Err(e) => {
                        let _ = tx.send(Action::ApiError(Self::login_error_message(&base_url, &e)));
                        return;
                    }
                };

            match client
                .request_oauth_device_auth(&client_id, None, Some(&hostname))
                .await
            {
                Ok(resp) => {
                    // The client_id is carried on the response path so the poll
                    // loop and the config write both use the one that was
                    // actually registered, rather than re-reading state that a
                    // concurrent login may have changed.
                    let _ = tx.send(Action::OAuthClientRegistered(client_id));
                    let _ = tx.send(Action::OAuthDeviceAuthReceived(Box::new(resp)));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(Self::login_error_message(&base_url, &e)));
                }
            }
        });
    }

    /// The name shown on the approval page and in the sessions list.
    fn device_name() -> String {
        std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "FlagDash CLI".to_string())
    }

    async fn ensure_client_id(
        client: &ApiClient,
        existing: String,
        hostname: &str,
    ) -> Result<String, crate::api::error::ApiError> {
        if !existing.is_empty() {
            return Ok(existing);
        }

        let registered = client
            .register_oauth_client(&format!("FlagDash CLI ({hostname})"))
            .await?;

        Ok(registered.client_id)
    }

    fn login_error_message(base_url: &str, e: &crate::api::error::ApiError) -> String {
        let text = e.to_string();

        if text.contains("Network") || text.contains("error sending request") {
            format!("Unable to connect to {base_url}. Is the server running?")
        } else {
            format!("Failed to start login: {text}")
        }
    }

    fn handle_oauth_device_auth_received(
        &mut self,
        device_auth: crate::api::types::OAuthDeviceAuthResponse,
    ) {
        self.login_view.set_waiting_oauth(&device_auth);

        // Prefer the prefilled URL so the person does not retype the code; the
        // approval page still shows it for them to compare against this screen,
        // which is the whole security property of RFC 8628.
        let target = device_auth
            .verification_uri_complete
            .clone()
            .unwrap_or_else(|| device_auth.verification_uri.clone());
        let _ = open::that(&target);

        let base_url = self.config.connection.base_url.clone();
        let client_id = self.config.auth.client_id.clone();
        let device_code = device_auth.device_code.clone();
        let expires_in = device_auth.expires_in.max(0) as u64;
        let tx = self.action_tx.clone();

        // Abort any previous polling loop before starting a new one, so repeated
        // login attempts (Esc → retry) don't stack concurrent pollers.
        self.abort_device_poll();

        let handle = tokio::spawn(async move {
            let client = ApiClient::new_unauthenticated(&base_url);

            // The server advertises the minimum gap between polls and answers
            // `slow_down` to anything faster. Backing off on that rather than
            // ignoring it is what keeps a login from rate-limiting itself.
            let mut interval = device_auth.interval.max(1);
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(expires_in);

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

                if tokio::time::Instant::now() >= deadline {
                    let _ = tx.send(Action::OAuthDevicePollResult(Box::new(
                        crate::api::types::DevicePollOutcome::Expired,
                    )));
                    return;
                }

                match client
                    .poll_oauth_device_token(&client_id, &device_code)
                    .await
                {
                    Ok(outcome) => {
                        let keep_going = outcome.keep_polling();

                        if matches!(outcome, crate::api::types::DevicePollOutcome::SlowDown) {
                            interval += 5;
                        }

                        // Pending is the flow working, and reporting it on every
                        // tick would repaint the view for no reason.
                        if !matches!(outcome, crate::api::types::DevicePollOutcome::Pending) {
                            let _ = tx.send(Action::OAuthDevicePollResult(Box::new(outcome)));
                        }

                        if !keep_going {
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ApiError(format!("Poll error: {e}")));
                        return;
                    }
                }
            }
        });

        self.device_poll_task = Some(handle);
    }

    /// Abort the in-flight device-auth polling task, if any.
    fn abort_device_poll(&mut self) {
        if let Some(handle) = self.device_poll_task.take() {
            handle.abort();
        }
    }

    fn handle_oauth_device_poll_result(&mut self, outcome: crate::api::types::DevicePollOutcome) {
        use crate::api::types::DevicePollOutcome as Outcome;

        match outcome {
            Outcome::Granted(tokens) => {
                // The loop has already returned on its side; drop the handle so a
                // finished task is not held.
                self.abort_device_poll();

                self.config.set_oauth_tokens(
                    tokens.access_token,
                    tokens.refresh_token,
                    tokens.expires_in,
                    tokens.scope,
                );
                let _ = self.config.save();

                self.rebuild_api_client();

                // An OAuth token response carries no identity, so who this is has
                // to be asked for separately before the UI can name them.
                self.request_identity();
            }

            // Pending never reaches here — the poll loop swallows it — but a
            // SlowDown is worth surfacing so a stalled login is explicable.
            Outcome::Pending | Outcome::SlowDown => {}

            Outcome::Expired => {
                self.login_view
                    .set_error("Login expired. Press Enter to try again.");
            }

            Outcome::Denied => {
                self.login_view
                    .set_error("Login denied. Press Enter to try again.");
            }

            Outcome::Failed(reason) => {
                self.login_view
                    .set_error(&format!("Login error: {reason}. Press Enter to retry."));
            }
        }
    }

    /// Whether a refresh is possible and not already running.
    fn can_refresh(&self) -> bool {
        !self.config.auth.refresh_token.is_empty()
            && !self.config.auth.client_id.is_empty()
            && self.refresh_task.is_none()
    }

    /// Exchange the refresh token for a new pair.
    ///
    /// The refresh token rotates, so the response has to be persisted even though
    /// only the access token was wanted — dropping the new refresh token would
    /// strand the session at the next expiry.
    fn handle_oauth_refresh_requested(&mut self) {
        if !self.can_refresh() {
            return;
        }

        let base_url = self.config.connection.base_url.clone();
        let client_id = self.config.auth.client_id.clone();
        let refresh_token = self.config.auth.refresh_token.clone();
        let tx = self.action_tx.clone();

        let handle = tokio::spawn(async move {
            let client = ApiClient::new_unauthenticated(&base_url);

            match client.refresh_oauth_token(&client_id, &refresh_token).await {
                Ok(outcome) => {
                    let _ = tx.send(Action::OAuthRefreshResult(Box::new(outcome)));
                }
                Err(e) => {
                    let _ = tx.send(Action::OAuthRefreshResult(Box::new(
                        crate::api::types::DevicePollOutcome::Failed(e.to_string()),
                    )));
                }
            }
        });

        self.refresh_task = Some(handle);
    }

    fn handle_oauth_refresh_result(&mut self, outcome: crate::api::types::DevicePollOutcome) {
        self.refresh_task = None;

        match outcome {
            crate::api::types::DevicePollOutcome::Granted(tokens) => {
                self.config.set_oauth_tokens(
                    tokens.access_token,
                    tokens.refresh_token,
                    tokens.expires_in,
                    tokens.scope,
                );
                let _ = self.config.save();
                self.rebuild_api_client();
                self.request_identity();
            }

            // A refresh token is only refused when it is revoked, rotated away, or
            // 30 days old. None of those are recoverable without the person, so
            // the credential is cleared rather than retried into a loop.
            other => {
                self.config.clear_auth();
                let _ = self.config.save();
                self.api = None;
                self.status_bar.connected = false;
                self.header.connected = false;
                self.current_view = View::Login;
                self.login_view.set_error(&format!(
                    "Your session ended ({other:?}). Press Enter to sign in."
                ));
            }
        }
    }

    /// Ask the server who the current credential belongs to.
    fn request_identity(&mut self) {
        let Some(api) = self.api.clone() else { return };
        let tx = self.action_tx.clone();

        tokio::spawn(async move {
            match api.whoami().await {
                Ok(identity) => {
                    let _ = tx.send(Action::IdentityResolved(Box::new(identity)));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(format!(
                        "Signed in, but could not read your profile: {e}"
                    )));
                }
            }
        });
    }

    fn handle_identity_resolved(&mut self, identity: crate::api::types::IdentityResponse) {
        if let Some(user) = &identity.user {
            self.config.auth.user_name = user.name.clone();
            self.config.auth.user_email = user.email.clone();
            self.config.auth.user_role = user.role.clone();
        }

        if let Some(credential) = &identity.credential {
            // What the *token* may do, which can be narrower than the role. The
            // server enforces it either way; carrying it lets the UI say so.
            self.config.auth.scope = credential.scope.clone();
        }

        let _ = self.config.save();

        self.apply_key_tier();

        self.status_bar.connected = true;
        self.header.connected = true;
        self.login_view.set_success();

        self.process_action(Action::LoginSuccess);
    }

    /// Point the API client at whichever credential is current.
    fn rebuild_api_client(&mut self) {
        match self.config.bearer_token() {
            Some(token) => {
                self.api = Some(ApiClient::new(&self.config.connection.base_url, token));
            }
            None => self.api = None,
        }
    }

    /// Push the current credential's tier into every view that gates on it.
    ///
    /// One place rather than nineteen assignments at each call site: a view added
    /// later and missed here shows the wrong affordances, and nothing fails.
    fn apply_key_tier(&mut self) {
        let key_tier = self.config.user_role_tier();

        self.flag_list.key_tier = key_tier.clone();
        self.flag_detail.key_tier = key_tier.clone();
        self.config_list.key_tier = key_tier.clone();
        self.config_detail.key_tier = key_tier.clone();
        self.ai_config_list.key_tier = key_tier.clone();
        self.ai_config_detail.key_tier = key_tier.clone();
        self.experiment_list.key_tier = key_tier.clone();
        self.experiment_detail.key_tier = key_tier.clone();
        self.webhook_list.key_tier = key_tier.clone();
        self.webhook_detail.key_tier = key_tier;
    }

    fn handle_logout(&mut self) {
        // Stop any in-flight device-auth polling loop on logout.
        self.abort_device_poll();

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
                | View::ExperimentList
                | View::ExperimentDetail(_)
                | View::WebhookList
                | View::WebhookDetail(_)
                | View::EnvironmentList
        )
    }

    fn is_searching(&self) -> bool {
        self.flag_list.search.active
            || self.config_list.search.active
            || self.ai_config_list.search.active
            || self.experiment_list.search.active
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
            View::ExperimentList => self.load_experiments(),
            View::ExperimentDetail(key) => self.load_experiment(key.clone()),
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
            sorted_flags.sort_by_key(|f| std::cmp::Reverse(f.updated_at));
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

    fn load_experiments(&self) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.list_experiments(&project_id).await {
                Ok(experiments) => {
                    let _ = tx.send(Action::ExperimentsLoaded(experiments));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn load_experiment(&self, key: String) {
        let Some(api) = &self.api else { return };
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.get_experiment(&key, &project_id).await {
                Ok(experiment) => {
                    let _ = tx.send(Action::ExperimentLoaded(Box::new(experiment)));
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

    fn submit_experiment_create(&mut self) {
        let Some(form) = &self.experiment_form else {
            return;
        };
        let Some(api) = &self.api else { return };
        let request = form.create_request();
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.create_experiment(&project_id, &request).await {
                Ok(experiment) => {
                    let _ = tx.send(Action::ExperimentCreated(Box::new(experiment)));
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Experiment created".to_string(),
                        level: ToastLevel::Success,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::ApiError(e.to_string()));
                }
            }
        });
    }

    fn submit_experiment_update(&mut self, key: String) {
        let Some(form) = &self.experiment_form else {
            return;
        };
        let Some(api) = &self.api else { return };
        let request = form.update_request();
        let api = api.clone();
        let project_id = self.config.defaults.project_id.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match api.update_experiment(&key, &project_id, &request).await {
                Ok(experiment) => {
                    let _ = tx.send(Action::ExperimentUpdated(Box::new(experiment)));
                    let _ = tx.send(Action::Toast(ToastMessage {
                        message: "Experiment updated".to_string(),
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
            self.toast.render(frame, area);
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
            View::ExperimentList => self.experiment_list.render(frame, area),
            View::ExperimentDetail(_) => self.experiment_detail.render(frame, area),
            View::ExperimentCreate | View::ExperimentEdit(_) => {
                if let Some(form) = &self.experiment_form {
                    form.render(frame, area);
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
