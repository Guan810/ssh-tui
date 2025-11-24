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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostEntry {
    pub host: String,
    pub hostname: String,
    pub user: String,
    pub port: String,
    pub identity_file: String,
    pub extra: Vec<String>,
}

impl HostEntry {
    pub fn validate(&self) -> Result<()> {
        if self.host.trim().is_empty() {
            anyhow::bail!("Host cannot be empty");
        }
        if self.host.contains('*') || self.host.contains('?') {
            anyhow::bail!("Host cannot contain wildcard characters");
        }
        if self.hostname.trim().is_empty() {
            anyhow::bail!("HostName cannot be empty");
        }
        if !self.port.trim().is_empty() {
            let port_num: u16 = self
                .port
                .trim()
                .parse()
                .context("Port must be a number between 1 and 65535")?;
            if port_num == 0 {
                anyhow::bail!("Port must be greater than 0");
            }
        }
        Ok(())
    }
}

pub fn parse_ssh_hosts(config_path: &Path) -> Result<Vec<String>> {
    Ok(load_host_entries_from_path(config_path)?
        .into_iter()
        .map(|entry| entry.host)
        .collect())
}

pub fn load_host_entries() -> Result<Vec<HostEntry>> {
    let path = Config::ssh_config_path()?;
    load_host_entries_from_path(&path)
}

pub fn load_host_entries_from_path(config_path: &Path) -> Result<Vec<HostEntry>> {
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(config_path)
        .context("Failed to read SSH config file")?;

    let mut entries = Vec::new();
    let mut current: Option<HostEntry> = None;

    for raw_line in contents.lines() {
        let line = strip_inline_comment(raw_line).trim();
        if line.is_empty() {
            if let Some(entry) = current.as_mut() {
                entry.extra.push(raw_line.trim_end().to_string());
            }
            continue;
        }

        let mut parts = line.split_whitespace();
        let keyword = parts.next().unwrap_or("");

        if keyword.eq_ignore_ascii_case("host") {
            if let Some(entry) = current.take() {
                if !entry.host.is_empty() {
                    entries.push(entry);
                }
            }

            if let Some(host_name) = parts.next() {
                if host_name.contains('*') || host_name.contains('?') {
                    current = None;
                } else {
                    current = Some(HostEntry {
                        host: host_name.to_string(),
                        ..HostEntry::default()
                    });
                }
            } else {
                current = None;
            }
        } else if let Some(entry) = current.as_mut() {
            let value = parts.collect::<Vec<_>>().join(" ");
            match keyword.to_ascii_lowercase().as_str() {
                "hostname" => entry.hostname = value,
                "user" => entry.user = value,
                "port" => entry.port = value,
                "identityfile" => entry.identity_file = value,
                _ => {
                    let trimmed_line = raw_line.trim();
                    if !trimmed_line.is_empty() {
                        entry.extra.push(trimmed_line.to_string());
                    }
                }
            }
        }
    }

    if let Some(entry) = current {
        if !entry.host.is_empty() {
            entries.push(entry);
        }
    }

    Ok(entries)
}

pub fn upsert_host_entry(entry: &HostEntry) -> Result<()> {
    let path = Config::ssh_config_path()?;
    upsert_host_entry_at_path(&path, entry)
}

pub fn update_host_entry(original_host: &str, entry: &HostEntry) -> Result<()> {
    let path = Config::ssh_config_path()?;
    update_host_entry_at_path(&path, original_host, entry)
}

pub fn upsert_host_entry_at_path(path: &Path, entry: &HostEntry) -> Result<()> {
    entry.validate()?;
    let mut lines = read_config_lines(path)?;

    if let Some((start, end)) = find_host_block(&lines, &entry.host) {
        replace_block(&mut lines, start, end, entry);
    } else {
        append_block(&mut lines, entry);
    }

    write_config_lines(path, &lines)
}

pub fn update_host_entry_at_path(path: &Path, original_host: &str, entry: &HostEntry) -> Result<()> {
    entry.validate()?;
    let mut lines = read_config_lines(path)?;

    if let Some((start, end)) = find_host_block(&lines, original_host) {
        replace_block(&mut lines, start, end, entry);
    } else {
        append_block(&mut lines, entry);
    }

    write_config_lines(path, &lines)
}

fn append_block(lines: &mut Vec<String>, entry: &HostEntry) {
    if !lines.is_empty() && !lines.last().map(|line| line.is_empty()).unwrap_or(false) {
        lines.push(String::new());
    }
    lines.extend(render_host_entry_lines(entry));
}

fn replace_block(lines: &mut Vec<String>, start: usize, end: usize, entry: &HostEntry) {
    lines.splice(start..end, render_host_entry_lines(entry));
}

fn read_config_lines(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(path)
        .context("Failed to read SSH config file")?;

    Ok(contents.lines().map(|line| line.to_string()).collect())
}

fn write_config_lines(path: &Path, lines: &[String]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create SSH config directory")?;
    }

    let mut buffer = String::new();
    for line in lines {
        buffer.push_str(line);
        buffer.push('\n');
    }

    fs::write(path, buffer).context("Failed to write SSH config file")
}

fn strip_inline_comment(line: &str) -> &str {
    if let Some(idx) = line.find('#') {
        &line[..idx]
    } else {
        line
    }
}

fn render_host_entry_lines(entry: &HostEntry) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("Host {}", entry.host.trim()));
    if !entry.hostname.trim().is_empty() {
        lines.push(format!("  HostName {}", entry.hostname.trim()));
    }
    if !entry.user.trim().is_empty() {
        lines.push(format!("  User {}", entry.user.trim()));
    }
    if !entry.port.trim().is_empty() {
        lines.push(format!("  Port {}", entry.port.trim()));
    }
    if !entry.identity_file.trim().is_empty() {
        lines.push(format!("  IdentityFile {}", entry.identity_file.trim()));
    }
    for extra_line in &entry.extra {
        lines.push(extra_line.clone());
    }
    lines.push(String::new());
    lines
}

fn host_name_from_line(line: &str) -> Option<String> {
    let stripped = strip_inline_comment(line);
    let trimmed = stripped.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let mut parts = trimmed.split_whitespace();
    let keyword = parts.next()?;
    if keyword.eq_ignore_ascii_case("host") {
        parts.next().map(|name| name.to_string())
    } else {
        None
    }
}

fn find_host_block(lines: &[String], host: &str) -> Option<(usize, usize)> {
    let mut index = 0;
    while index < lines.len() {
        if let Some(name) = host_name_from_line(&lines[index]) {
            let start = index;
            index += 1;
            while index < lines.len() {
                if host_name_from_line(&lines[index]).is_some() {
                    break;
                }
                index += 1;
            }

            if name == host {
                return Some((start, index));
            }
        } else {
            index += 1;
        }
    }
    None
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

    #[test]
    fn test_load_host_entries_from_path() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Host server1").unwrap();
        writeln!(file, "  HostName example.com").unwrap();
        writeln!(file, "  User admin").unwrap();
        writeln!(file, "  Port 2222").unwrap();
        writeln!(file, "  IdentityFile ~/.ssh/id_rsa").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "Host server2").unwrap();
        writeln!(file, "  HostName 192.168.1.1").unwrap();

        let entries = load_host_entries_from_path(file.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].host, "server1");
        assert_eq!(entries[0].hostname, "example.com");
        assert_eq!(entries[0].user, "admin");
        assert_eq!(entries[0].port, "2222");
        assert_eq!(entries[0].identity_file, "~/.ssh/id_rsa");
    }

    #[test]
    fn test_upsert_host_entry_at_path_creates_new_block() {
        let file = NamedTempFile::new().unwrap();
        let entry = HostEntry {
            host: "newserver".to_string(),
            hostname: "example.com".to_string(),
            user: "admin".to_string(),
            port: "2222".to_string(),
            identity_file: "~/.ssh/id_rsa".to_string(),
            extra: Vec::new(),
        };

        upsert_host_entry_at_path(file.path(), &entry).unwrap();

        let contents = fs::read_to_string(file.path()).unwrap();
        assert!(contents.contains("Host newserver"));
        assert!(contents.contains("HostName example.com"));
        assert!(contents.contains("User admin"));
    }

    #[test]
    fn test_update_host_entry_at_path_replaces_existing_block() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Host server1").unwrap();
        writeln!(file, "  HostName old.example.com").unwrap();
        writeln!(file, "  User olduser").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "Host server2").unwrap();
        writeln!(file, "  HostName 192.168.1.1").unwrap();

        let entry = HostEntry {
            host: "server1".to_string(),
            hostname: "new.example.com".to_string(),
            user: "newuser".to_string(),
            port: "2222".to_string(),
            identity_file: String::new(),
            extra: Vec::new(),
        };

        update_host_entry_at_path(file.path(), "server1", &entry).unwrap();

        let contents = fs::read_to_string(file.path()).unwrap();
        assert!(contents.contains("Host server1"));
        assert!(contents.contains("HostName new.example.com"));
        assert!(contents.contains("User newuser"));
        assert!(contents.contains("Port 2222"));
        assert!(!contents.contains("old.example.com"));
    }

    #[test]
    fn test_update_preserves_extra_lines() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Host server1").unwrap();
        writeln!(file, "  HostName old.example.com").unwrap();
        writeln!(file, "  ProxyCommand ssh jump").unwrap();
        writeln!(file, "  User admin").unwrap();

        let mut entries = load_host_entries_from_path(file.path()).unwrap();
        let mut entry = entries.remove(0);
        entry.hostname = "new.example.com".to_string();

        update_host_entry_at_path(file.path(), "server1", &entry).unwrap();

        let contents = fs::read_to_string(file.path()).unwrap();
        assert!(contents.contains("ProxyCommand ssh jump"));
        assert!(contents.contains("HostName new.example.com"));
    }

    #[test]
    fn test_host_entry_validation() {
        let mut entry = HostEntry::default();
        entry.host = String::new();
        entry.hostname = "example.com".to_string();
        assert!(entry.validate().is_err());

        entry.host = "valid".to_string();
        entry.hostname.clear();
        assert!(entry.validate().is_err());

        entry.hostname = "example.com".to_string();
        entry.port = "abc".to_string();
        assert!(entry.validate().is_err());

        entry.port = "2222".to_string();
        assert!(entry.validate().is_ok());
    }
}
