use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_ssh_binary")]
    pub ssh_binary: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_ssh_binary() -> String {
    "ssh".to_string()
}

fn default_timeout() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ssh_binary: default_ssh_binary(),
            timeout: default_timeout(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            let config: Config = toml::from_str(&contents)
                .context("Failed to parse config file")?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        Ok(home.join(".config").join("ssh-tui").join("config.toml"))
    }

    pub fn ssh_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        Ok(home.join(".ssh").join("config"))
    }
}

pub fn parse_ssh_hosts(config_path: &Path) -> Result<Vec<String>> {
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(config_path)
        .context("Failed to read SSH config file")?;

    let mut hosts = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Host ") {
            let host = trimmed.strip_prefix("Host ").unwrap().trim();
            if !host.contains('*') && !host.contains('?') {
                hosts.push(host.to_string());
            }
        }
    }

    Ok(hosts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_ssh_hosts() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Host server1").unwrap();
        writeln!(file, "  HostName example.com").unwrap();
        writeln!(file, "  User admin").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "Host server2").unwrap();
        writeln!(file, "  HostName 192.168.1.1").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "Host *").unwrap();
        writeln!(file, "  ServerAliveInterval 60").unwrap();

        let hosts = parse_ssh_hosts(file.path()).unwrap();
        assert_eq!(hosts, vec!["server1", "server2"]);
    }

    #[test]
    fn test_parse_ssh_hosts_empty() {
        let file = NamedTempFile::new().unwrap();
        let hosts = parse_ssh_hosts(file.path()).unwrap();
        assert_eq!(hosts, Vec::<String>::new());
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.ssh_binary, "ssh");
        assert_eq!(config.timeout, 30);
    }
}
