use crate::{
    config::{parse_ssh_hosts, Config},
    ssh::SshConnection,
};
use anyhow::Result;
use std::time::Duration;

pub struct App {
    pub hosts: Vec<String>,
    pub selected: usize,
    pub status: Option<String>,
    #[allow(dead_code)]
    config: Config,
    ssh_connection: SshConnection,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let ssh_config_path = Config::ssh_config_path()?;
        let hosts = parse_ssh_hosts(&ssh_config_path)?;

        let ssh_connection = SshConnection::new(
            config.ssh_binary.clone(),
            Duration::from_secs(config.timeout),
        );

        Ok(Self {
            hosts,
            selected: 0,
            status: None,
            config,
            ssh_connection,
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
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.hosts.len() - 1;
        }
    }

    pub fn selected_host(&self) -> Option<&str> {
        self.hosts.get(self.selected).map(|s| s.as_str())
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_wraps_around() {
        let mut app = App {
            hosts: vec!["host1".to_string(), "host2".to_string(), "host3".to_string()],
            selected: 2,
            status: None,
            config: Config::default(),
            ssh_connection: SshConnection::new("ssh".to_string(), Duration::from_secs(30)),
        };

        app.next();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_previous_wraps_around() {
        let mut app = App {
            hosts: vec!["host1".to_string(), "host2".to_string(), "host3".to_string()],
            selected: 0,
            status: None,
            config: Config::default(),
            ssh_connection: SshConnection::new("ssh".to_string(), Duration::from_secs(30)),
        };

        app.previous();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn test_selected_host() {
        let app = App {
            hosts: vec!["host1".to_string(), "host2".to_string()],
            selected: 1,
            status: None,
            config: Config::default(),
            ssh_connection: SshConnection::new("ssh".to_string(), Duration::from_secs(30)),
        };

        assert_eq!(app.selected_host(), Some("host2"));
    }

    #[test]
    fn test_empty_hosts() {
        let mut app = App {
            hosts: vec![],
            selected: 0,
            status: None,
            config: Config::default(),
            ssh_connection: SshConnection::new("ssh".to_string(), Duration::from_secs(30)),
        };

        app.next();
        assert_eq!(app.selected, 0);

        app.previous();
        assert_eq!(app.selected, 0);

        assert_eq!(app.selected_host(), None);
    }
}
