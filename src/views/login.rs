use crate::action::Action;
use crate::api::types::OAuthDeviceAuthResponse;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

#[derive(Debug, Clone, PartialEq)]
enum LoginState {
    Idle,
    /// An existing session is being restored from a refresh token. Shown instead
    /// of the sign-in prompt, which would tell someone who *is* signed in that
    /// they are not.
    Restoring,
    WaitingForBrowser {
        user_code: String,
        verification_url: String,
    },
    Success,
    Error(String),
}

pub struct LoginView {
    state: LoginState,
    spinner_tick: u8,
}

impl LoginView {
    pub fn new() -> Self {
        Self {
            state: LoginState::Idle,
            spinner_tick: 0,
        }
    }

    pub fn set_error(&mut self, msg: &str) {
        self.state = LoginState::Error(msg.to_string());
    }

    /// Show the code and the URL for an OAuth device-grant login.
    ///
    /// Deliberately the plain `verification_uri`, not the prefilled variant: the
    /// prefilled one is opened in the browser for convenience, but what is shown
    /// here is what a person types by hand, and they should then confirm the code
    /// against this screen rather than trust a link.
    pub fn set_waiting_oauth(&mut self, device_auth: &OAuthDeviceAuthResponse) {
        self.state = LoginState::WaitingForBrowser {
            user_code: device_auth.user_code.clone(),
            verification_url: device_auth.verification_uri.clone(),
        };
    }

    pub fn set_restoring(&mut self) {
        self.state = LoginState::Restoring;
    }

    pub fn set_success(&mut self) {
        self.state = LoginState::Success;
    }

    pub fn tick(&mut self) {
        self.spinner_tick = self.spinner_tick.wrapping_add(1);
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Tick = event {
            self.tick();
            return None;
        }

        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }

            match &self.state {
                LoginState::Idle => {
                    if key.code == KeyCode::Enter {
                        return Some(Action::BrowserLoginRequested);
                    }
                    if key.code == KeyCode::Esc {
                        return Some(Action::Quit);
                    }
                }
                LoginState::WaitingForBrowser { .. } => {
                    if key.code == KeyCode::Esc {
                        self.state = LoginState::Idle;
                        return None;
                    }
                }
                // Restoring resolves on its own within a request or two. Esc
                // abandons it and offers a fresh sign-in instead of hanging.
                LoginState::Restoring => {
                    if key.code == KeyCode::Esc {
                        self.state = LoginState::Idle;
                        return None;
                    }
                }
                LoginState::Error(_) => {
                    if key.code == KeyCode::Enter {
                        self.state = LoginState::Idle;
                        return Some(Action::BrowserLoginRequested);
                    }
                    if key.code == KeyCode::Esc {
                        self.state = LoginState::Idle;
                        return None;
                    }
                }
                LoginState::Success => {
                    // Handled by app.rs via LoginSuccess action
                }
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        match &self.state {
            LoginState::Idle => self.render_idle(frame, area),
            LoginState::WaitingForBrowser {
                user_code,
                verification_url,
            } => self.render_waiting(frame, area, user_code, verification_url),
            LoginState::Restoring => self.render_restoring(frame, area),
            LoginState::Success => self.render_success(frame, area),
            LoginState::Error(msg) => self.render_error(frame, area, msg),
        }
    }

    /// One frame of the waiting spinner. Shared so the restoring and waiting
    /// screens cannot drift out of step with each other.
    fn spinner_frame(&self) -> &'static str {
        const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
        FRAMES[(self.spinner_tick as usize / 2) % FRAMES.len()]
    }

    fn render_restoring(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(8), // Logo
            Constraint::Length(2), // Message
            Constraint::Min(0),
        ])
        .split(area);

        let logo = Paragraph::new(theme::LOGO)
            .style(theme::title())
            .alignment(Alignment::Center);
        frame.render_widget(logo, chunks[1]);

        let msg = Paragraph::new(Line::from(vec![
            Span::styled(self.spinner_frame(), theme::title()),
            Span::styled("  Restoring your session…", theme::dim()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(msg, chunks[2]);
    }

    fn render_idle(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(8), // Logo
            Constraint::Length(2), // Welcome text
            Constraint::Length(2), // Instructions
            Constraint::Min(0),
        ])
        .split(area);

        // Logo
        let logo = Paragraph::new(theme::LOGO)
            .style(theme::title())
            .alignment(Alignment::Center);
        frame.render_widget(logo, chunks[1]);

        // Welcome
        let welcome = Paragraph::new(Line::from(vec![Span::styled(
            "Press Enter to log in with your browser",
            theme::dim(),
        )]))
        .alignment(Alignment::Center);
        frame.render_widget(welcome, chunks[2]);

        // Instructions
        let instructions = Paragraph::new(Line::from(vec![
            Span::styled("Enter", theme::title()),
            Span::styled(" to log in  ", theme::dim()),
            Span::styled("Esc", theme::title()),
            Span::styled(" to quit", theme::dim()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[3]);
    }

    fn render_waiting(
        &self,
        frame: &mut Frame,
        area: Rect,
        user_code: &str,
        verification_url: &str,
    ) {
        let chunks = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(8), // Logo
            Constraint::Length(2), // "Opening browser..."
            Constraint::Length(2), // Verification URL
            Constraint::Length(1), // spacer
            Constraint::Length(2), // "Your code:"
            Constraint::Length(3), // User code (large)
            Constraint::Length(1), // spacer
            Constraint::Length(2), // Spinner + waiting
            Constraint::Length(2), // Instructions
            Constraint::Min(0),
        ])
        .split(area);

        // Logo
        let logo = Paragraph::new(theme::LOGO)
            .style(theme::title())
            .alignment(Alignment::Center);
        frame.render_widget(logo, chunks[1]);

        // Opening browser message
        let msg = Paragraph::new(Line::from(vec![Span::styled(
            "A browser window should have opened for you to log in.",
            theme::dim(),
        )]))
        .alignment(Alignment::Center);
        frame.render_widget(msg, chunks[2]);

        // Verification URL
        let url_line = Paragraph::new(Line::from(vec![
            Span::styled("If not, go to: ", theme::dim()),
            Span::styled(verification_url, theme::title()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(url_line, chunks[3]);

        // "Your code:" label
        let label = Paragraph::new(Line::from(vec![Span::styled(
            "Enter this code when prompted:",
            theme::dim(),
        )]))
        .alignment(Alignment::Center);
        frame.render_widget(label, chunks[5]);

        // User code displayed prominently
        let code_display = Paragraph::new(Line::from(vec![Span::styled(
            format!("  {}  ", user_code),
            theme::heading(),
        )]))
        .alignment(Alignment::Center);
        frame.render_widget(code_display, chunks[6]);

        // Spinner
        let waiting = Paragraph::new(Line::from(vec![
            Span::styled(format!("{} ", self.spinner_frame()), theme::title()),
            Span::styled("Waiting for authorization...", theme::dim()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(waiting, chunks[8]);

        // Instructions
        let instructions = Paragraph::new(Line::from(vec![
            Span::styled("Esc", theme::title()),
            Span::styled(" to cancel", theme::dim()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[9]);
    }

    fn render_success(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(8), // Logo
            Constraint::Length(2), // Success message
            Constraint::Min(0),
        ])
        .split(area);

        let logo = Paragraph::new(theme::LOGO)
            .style(theme::title())
            .alignment(Alignment::Center);
        frame.render_widget(logo, chunks[1]);

        let msg = Paragraph::new(Line::from(vec![Span::styled(
            "Logged in successfully! Loading...",
            theme::status_on(),
        )]))
        .alignment(Alignment::Center);
        frame.render_widget(msg, chunks[2]);
    }

    fn render_error(&self, frame: &mut Frame, area: Rect, error_msg: &str) {
        let chunks = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(8), // Logo
            Constraint::Length(2), // Error message
            Constraint::Length(2), // Instructions
            Constraint::Min(0),
        ])
        .split(area);

        let logo = Paragraph::new(theme::LOGO)
            .style(theme::title())
            .alignment(Alignment::Center);
        frame.render_widget(logo, chunks[1]);

        let error = Paragraph::new(Line::from(Span::styled(error_msg, theme::status_off())))
            .alignment(Alignment::Center);
        frame.render_widget(error, chunks[2]);

        let instructions = Paragraph::new(Line::from(vec![
            Span::styled("Enter", theme::title()),
            Span::styled(" to retry  ", theme::dim()),
            Span::styled("Esc", theme::title()),
            Span::styled(" to go back", theme::dim()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[3]);
    }
}
