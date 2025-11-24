use crate::config::Config;
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostEntry {
    pub host: String,
    pub hostname: String,
    pub user: String,
    pub port: String,
    pub identity_file: String,
    pub proxy_command: String,
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

pub fn list_entries() -> Result<Vec<HostEntry>> {
    load_host_entries()
}

pub fn load_host_entries() -> Result<Vec<HostEntry>> {
    let path = Config::ssh_config_path()?;
    load_host_entries_from_path(&path)
}

pub fn load_host_entries_from_path(path: &Path) -> Result<Vec<HostEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(path)
        .context("Failed to read SSH config file")?;

    let mut entries = Vec::new();
    let mut current: Option<HostEntry> = None;

    for raw_line in contents.lines() {
        let trimmed_start = raw_line.trim_start();
        if trimmed_start.starts_with('#') {
            if let Some(entry) = current.as_mut() {
                entry.extra.push(raw_line.trim_end().to_string());
            }
            continue;
        }

        let line = strip_inline_comment(raw_line).trim();
        if line.is_empty() {
            if let Some(entry) = current.as_mut() {
                entry.extra.push(String::new());
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
                "proxycommand" => entry.proxy_command = value,
                _ => entry.extra.push(raw_line.trim_end().to_string()),
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

pub fn add_host_entry(entry: &HostEntry) -> Result<()> {
    let path = Config::ssh_config_path()?;
    add_host_entry_at_path(&path, entry)
}

pub fn add_host_entry_at_path(path: &Path, entry: &HostEntry) -> Result<()> {
    entry.validate()?;
    let mut lines = read_config_lines(path)?;

    if find_host_block(&lines, &entry.host).is_some() {
        anyhow::bail!("Host '{}' already exists", entry.host);
    }

    append_block(&mut lines, entry);
    write_config_lines(path, &lines)
}

pub fn upsert_host_entry(entry: &HostEntry) -> Result<()> {
    let path = Config::ssh_config_path()?;
    upsert_host_entry_at_path(&path, entry)
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

pub fn update_host_entry(original_host: &str, entry: &HostEntry) -> Result<()> {
    let path = Config::ssh_config_path()?;
    update_host_entry_at_path(&path, original_host, entry)
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

pub fn delete_host_entry(host: &str) -> Result<()> {
    let path = Config::ssh_config_path()?;
    delete_host_entry_at_path(&path, host)
}

pub fn delete_host_entry_at_path(path: &Path, host: &str) -> Result<()> {
    let mut lines = read_config_lines(path)?;

    if let Some((start, end)) = find_host_block(&lines, host) {
        remove_block(&mut lines, start, end);
        write_config_lines(path, &lines)
    } else {
        anyhow::bail!("Host '{}' not found", host);
    }
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

fn append_block(lines: &mut Vec<String>, entry: &HostEntry) {
    if !lines.is_empty() && !lines.last().map(|line| line.is_empty()).unwrap_or(false) {
        lines.push(String::new());
    }
    lines.extend(render_host_entry_lines(entry));
}

fn replace_block(lines: &mut Vec<String>, start: usize, end: usize, entry: &HostEntry) {
    lines.splice(start..end, render_host_entry_lines(entry));
}

fn remove_block(lines: &mut Vec<String>, start: usize, end: usize) {
    lines.drain(start..end);

    if start < lines.len() {
        while start < lines.len() && lines[start].is_empty() {
            lines.remove(start);
        }
    } else {
        while lines.last().map(|line| line.is_empty()).unwrap_or(false) {
            lines.pop();
        }
    }
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
    if !entry.proxy_command.trim().is_empty() {
        lines.push(format!("  ProxyCommand {}", entry.proxy_command.trim()));
    }

    for extra_line in &entry.extra {
        lines.push(extra_line.clone());
    }

    if !lines.last().map(|line| line.is_empty()).unwrap_or(false) {
        lines.push(String::new());
    }

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

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("ssh_config")
            .join(name)
    }

    #[test]
    fn test_load_host_entries_from_fixture() {
        let path = fixture_path("sample_config");
        let entries = load_host_entries_from_path(&path).unwrap();
        assert_eq!(entries.len(), 2);

        let app = entries.iter().find(|e| e.host == "app-server").unwrap();
        assert_eq!(app.hostname, "app.example.com");
        assert_eq!(app.user, "deploy");
        assert_eq!(app.port, "2222");
        assert_eq!(app.identity_file, "~/.ssh/app_rsa");
        assert_eq!(app.proxy_command, "ssh -W %h:%p bastion");
        assert!(app.extra.iter().any(|line| line.contains("LocalForward")));
        assert!(app.extra.iter().any(|line| line.contains("# inline comment")));
        assert!(app.extra.iter().any(|line| line.contains("ForwardAgent")));
    }

    #[test]
    fn test_missing_config_returns_empty_list() {
        let path = fixture_path("does_not_exist");
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }
        let entries = load_host_entries_from_path(&path).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_upsert_preserves_unknown_directives() {
        let path = fixture_path("sample_config");
        let mut temp = NamedTempFile::new().unwrap();
        let contents = fs::read_to_string(path).unwrap();
        write!(temp, "{}", contents).unwrap();

        let mut entries = load_host_entries_from_path(temp.path()).unwrap();
        let mut entry = entries
            .into_iter()
            .find(|e| e.host == "app-server")
            .unwrap();
        entry.hostname = "new.example.com".to_string();

        update_host_entry_at_path(temp.path(), "app-server", &entry).unwrap();

        let file_contents = fs::read_to_string(temp.path()).unwrap();
        assert!(file_contents.contains("LocalForward 8080 localhost:80"));
        assert!(file_contents.contains("# inline comment"));
        assert!(file_contents.contains("HostName new.example.com"));
    }

    #[test]
    fn test_add_and_delete_host_entry() {
        let path = fixture_path("sample_config");
        let mut temp = NamedTempFile::new().unwrap();
        let contents = fs::read_to_string(path).unwrap();
        write!(temp, "{}", contents).unwrap();

        let new_entry = HostEntry {
            host: "web".to_string(),
            hostname: "web.example.com".to_string(),
            user: "www".to_string(),
            port: "22".to_string(),
            identity_file: "~/.ssh/web_rsa".to_string(),
            proxy_command: String::new(),
            extra: vec!["  ForwardAgent yes".to_string()],
        };

        add_host_entry_at_path(temp.path(), &new_entry).unwrap();
        let contents = fs::read_to_string(temp.path()).unwrap();
        assert!(contents.contains("Host web"));
        assert!(contents.contains("ForwardAgent yes"));

        delete_host_entry_at_path(temp.path(), "web").unwrap();
        let contents = fs::read_to_string(temp.path()).unwrap();
        assert!(!contents.contains("Host web"));
    }

    #[test]
    fn test_add_duplicate_host_fails() {
        let path = fixture_path("sample_config");
        let mut temp = NamedTempFile::new().unwrap();
        let contents = fs::read_to_string(path).unwrap();
        write!(temp, "{}", contents).unwrap();

        let entry = HostEntry {
            host: "app-server".to_string(),
            hostname: "example.com".to_string(),
            user: "user".to_string(),
            port: "22".to_string(),
            identity_file: String::new(),
            proxy_command: String::new(),
            extra: vec![],
        };

        assert!(add_host_entry_at_path(temp.path(), &entry).is_err());
    }

    #[test]
    fn test_delete_unknown_host_fails() {
        let mut temp = NamedTempFile::new().unwrap();
        delete_host_entry_at_path(temp.path(), "missing").unwrap_err();
    }

    #[test]
    fn test_host_entry_validation() {
        let mut entry = HostEntry::default();
        entry.host = "".to_string();
        entry.hostname = "example.com".to_string();
        assert!(entry.validate().is_err());

        entry.host = "valid".to_string();
        entry.hostname.clear();
        assert!(entry.validate().is_err());

        entry.hostname = "example.com".to_string();
        entry.port = "abc".to_string();
        assert!(entry.validate().is_err());

        entry.port = "22".to_string();
        assert!(entry.validate().is_ok());
    }
}
