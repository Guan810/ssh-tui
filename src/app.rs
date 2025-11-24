use crate::{
    config::Config,
    ssh::SshConnection,
    ssh_config::{load_host_entries, update_host_entry, upsert_host_entry, HostEntry},
};
use anyhow::Result;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Normal,
    Edit,
    New,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Host,
    HostName,
    User,
    Port,
    IdentityFile,
}

impl FormField {
    fn next(self) -> Self {
        match self {
            FormField::Host => FormField::HostName,
            FormField::HostName => FormField::User,
            FormField::User => FormField::Port,
            FormField::Port => FormField::IdentityFile,
            FormField::IdentityFile => FormField::Host,
        }
    }

    fn previous(self) -> Self {
        match self {
            FormField::Host => FormField::IdentityFile,
            FormField::HostName => FormField::Host,
            FormField::User => FormField::HostName,
            FormField::Port => FormField::User,
            FormField::IdentityFile => FormField::Port,
        }
    }
}

pub struct App {
    pub hosts: Vec<HostEntry>,
    pub selected: usize,
    pub status: Option<String>,
    pub state: AppState,
    pub form_entry: HostEntry,
    pub form_field: FormField,
    pub form_error: Option<String>,
    #[allow(dead_code)]
    config: Config,
    ssh_connection: SshConnection,
    original_host_name: Option<String>,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let hosts = load_host_entries()?;

        let ssh_connection = SshConnection::new(
            config.ssh_binary.clone(),
            Duration::from_secs(config.timeout),
        );

        Ok(Self {
            hosts,
            selected: 0,
            status: None,
            state: AppState::Normal,
            form_entry: HostEntry::default(),
            form_field: FormField::Host,
            form_error: None,
            config,
            ssh_connection,
            original_host_name: None,
        })
    }

    pub fn next(&mut self) {
        if self.hosts.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.hosts.len();
    }

    pub fn previous(&mut self) {
        if self.hosts.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.hosts.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    pub fn selected_host(&self) -> Option<&HostEntry> {
        self.hosts.get(self.selected)
    }

    pub fn selected_host_name(&self) -> Option<&str> {
        self.selected_host().map(|entry| entry.host.as_str())
    }

    pub fn connect_to_host(&mut self, host: &str) -> Result<String> {
        self.ssh_connection.connect(host)
    }

    pub fn set_status(&mut self, result: Result<String>) {
        match result {
            Ok(msg) => self.status = Some(msg),
            Err(e) => self.status = Some(format!("Error: {}", e)),
        }
    }

    pub fn is_form_active(&self) -> bool {
        !matches!(self.state, AppState::Normal)
    }

    pub fn enter_edit_mode(&mut self) {
        if let Some(entry) = self.selected_host().cloned() {
            self.form_entry = entry.clone();
            self.original_host_name = Some(entry.host);
            self.form_field = FormField::Host;
            self.form_error = None;
            self.state = AppState::Edit;
        }
    }

    pub fn enter_new_mode(&mut self) {
        self.form_entry = HostEntry::default();
        self.original_host_name = None;
        self.form_field = FormField::Host;
        self.form_error = None;
        self.state = AppState::New;
    }

    pub fn cancel_form(&mut self) {
        self.state = AppState::Normal;
        self.form_entry = HostEntry::default();
        self.form_error = None;
        self.original_host_name = None;
    }

    pub fn focus_next_field(&mut self) {
        if self.is_form_active() {
            self.form_field = self.form_field.next();
        }
    }

    pub fn focus_previous_field(&mut self) {
        if self.is_form_active() {
            self.form_field = self.form_field.previous();
        }
    }

    pub fn handle_form_input(&mut self, ch: char) {
        if !self.is_form_active() || ch.is_control() {
            return;
        }
        self.form_error = None;
        let field = self.current_field_mut();
        field.push(ch);
    }

    pub fn handle_form_backspace(&mut self) {
        if !self.is_form_active() {
            return;
        }
        self.form_error = None;
        let field = self.current_field_mut();
        field.pop();
    }

    pub fn save_form(&mut self) {
        if !self.is_form_active() {
            return;
        }

        let mode = self.state;
        let entry = self.form_entry.clone();

        if let Err(err) = entry.validate() {
            self.form_error = Some(err.to_string());
            return;
        }

        let result = match mode {
            AppState::Edit => {
                let original = self
                    .original_host_name
                    .clone()
                    .unwrap_or_else(|| entry.host.clone());
                update_host_entry(&original, &entry)
            }
            AppState::New => upsert_host_entry(&entry),
            AppState::Normal => Ok(()),
        };

        match result {
            Ok(()) => {
                if let Err(err) = self.refresh_hosts(Some(entry.host.clone())) {
                    self.form_error = Some(err.to_string());
                    return;
                }
                self.state = AppState::Normal;
                self.form_entry = HostEntry::default();
                self.form_error = None;
                self.original_host_name = None;
                let action = match mode {
                    AppState::Edit => "updated",
                    AppState::New => "created",
                    AppState::Normal => "saved",
                };
                self.status = Some(format!("Host '{}' {} successfully", entry.host, action));
            }
            Err(err) => {
                self.form_error = Some(err.to_string());
            }
        }
    }

    fn refresh_hosts(&mut self, focus: Option<String>) -> Result<()> {
        self.hosts = load_host_entries()?;
        if self.hosts.is_empty() {
            self.selected = 0;
            return Ok(());
        }

        if let Some(host) = focus {
            if let Some(index) = self.hosts.iter().position(|entry| entry.host == host) {
                self.selected = index;
                return Ok(());
            }
        }

        if self.selected >= self.hosts.len() {
            self.selected = self.hosts.len() - 1;
        }
        Ok(())
    }

    fn current_field_mut(&mut self) -> &mut String {
        match self.form_field {
            FormField::Host => &mut self.form_entry.host,
            FormField::HostName => &mut self.form_entry.hostname,
            FormField::User => &mut self.form_entry.user,
            FormField::Port => &mut self.form_entry.port,
            FormField::IdentityFile => &mut self.form_entry.identity_file,
        }
    }

    #[cfg(test)]
    pub fn test_with_hosts(hosts: Vec<HostEntry>) -> Self {
        Self {
            hosts,
            selected: 0,
            status: None,
            state: AppState::Normal,
            form_entry: HostEntry::default(),
            form_field: FormField::Host,
            form_error: None,
            config: Config::default(),
            ssh_connection: SshConnection::new("ssh".to_string(), Duration::from_secs(30)),
            original_host_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::SshConnection;

    fn host(name: &str) -> HostEntry {
        HostEntry {
            host: name.to_string(),
            hostname: "example.com".to_string(),
            user: "user".to_string(),
            port: String::new(),
            identity_file: String::new(),
            proxy_command: String::new(),
            extra: Vec::new(),
        }
    }

    fn test_app() -> App {
        App {
            hosts: vec![host("a"), host("b"), host("c")],
            selected: 0,
            status: None,
            state: AppState::Normal,
            form_entry: HostEntry::default(),
            form_field: FormField::Host,
            form_error: None,
            config: Config::default(),
            ssh_connection: SshConnection::new("ssh".to_string(), Duration::from_secs(30)),
            original_host_name: None,
        }
    }

    #[test]
    fn next_wraps() {
        let mut app = test_app();
        app.selected = 2;
        app.next();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn previous_wraps() {
        let mut app = test_app();
        app.previous();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn selected_host_returns_entry() {
        let app = test_app();
        assert_eq!(app.selected_host_name(), Some("a"));
    }

    #[test]
    fn focus_navigation_cycles_fields() {
        let mut app = test_app();
        app.enter_new_mode();
        app.focus_next_field();
        assert_eq!(app.form_field, FormField::HostName);
        app.focus_previous_field();
        assert_eq!(app.form_field, FormField::Host);
    }

    #[test]
    fn handle_form_input_updates_field() {
        let mut app = test_app();
        app.enter_new_mode();
        app.handle_form_input('s');
        app.handle_form_input('1');
        assert_eq!(app.form_entry.host, "s1");
        app.handle_form_backspace();
        assert_eq!(app.form_entry.host, "s");
    }
}
