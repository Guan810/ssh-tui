use anyhow::{Context, Result};
use std::{
    process::{Command, ExitStatus},
    time::Duration,
};

pub trait CommandExecutor {
    fn execute(&self, ssh_binary: &str, host: &str) -> Result<ExitStatus>;
}

pub struct RealCommandExecutor;

impl CommandExecutor for RealCommandExecutor {
    fn execute(&self, ssh_binary: &str, host: &str) -> Result<ExitStatus> {
        let status = Command::new(ssh_binary)
            .arg(host)
            .status()
            .context("Failed to execute SSH command")?;
        Ok(status)
    }
}

pub struct SshConnection {
    executor: Box<dyn CommandExecutor>,
    ssh_binary: String,
    #[allow(dead_code)]
    timeout: Duration,
}

impl SshConnection {
    pub fn new(ssh_binary: String, timeout: Duration) -> Self {
        Self {
            executor: Box::new(RealCommandExecutor),
            ssh_binary,
            timeout,
        }
    }

    #[allow(dead_code)]
    pub fn with_executor(
        ssh_binary: String,
        timeout: Duration,
        executor: Box<dyn CommandExecutor>,
    ) -> Self {
        Self {
            executor,
            ssh_binary,
            timeout,
        }
    }

    pub fn connect(&self, host: &str) -> Result<String> {
        let status = self.executor.execute(&self.ssh_binary, host)?;

        if status.success() {
            Ok(format!("Successfully connected to {}", host))
        } else {
            match status.code() {
                Some(code) => Ok(format!("Connection to {} exited with code {}", host, code)),
                None => Ok(format!("Connection to {} terminated by signal", host)),
            }
        }
    }

    #[allow(dead_code)]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::ExitStatus;

    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    struct MockCommandExecutor {
        success: bool,
        exit_code: Option<i32>,
    }

    impl CommandExecutor for MockCommandExecutor {
        fn execute(&self, _ssh_binary: &str, _host: &str) -> Result<ExitStatus> {
            #[cfg(unix)]
            {
                let status = if self.success {
                    ExitStatus::from_raw(0)
                } else if let Some(code) = self.exit_code {
                    ExitStatus::from_raw(code << 8)
                } else {
                    ExitStatus::from_raw(1)
                };
                Ok(status)
            }
            #[cfg(not(unix))]
            {
                panic!("Mock tests only work on Unix systems");
            }
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_successful_connection() {
        let executor = Box::new(MockCommandExecutor {
            success: true,
            exit_code: None,
        });
        let connection =
            SshConnection::with_executor("ssh".to_string(), Duration::from_secs(30), executor);

        let result = connection.connect("test-host").unwrap();
        assert_eq!(result, "Successfully connected to test-host");
    }

    #[test]
    #[cfg(unix)]
    fn test_failed_connection_with_code() {
        let executor = Box::new(MockCommandExecutor {
            success: false,
            exit_code: Some(255),
        });
        let connection =
            SshConnection::with_executor("ssh".to_string(), Duration::from_secs(30), executor);

        let result = connection.connect("test-host").unwrap();
        assert_eq!(result, "Connection to test-host exited with code 255");
    }

    #[test]
    fn test_timeout_configuration() {
        let connection = SshConnection::new("ssh".to_string(), Duration::from_secs(60));
        assert_eq!(connection.timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_custom_ssh_binary() {
        let executor = Box::new(MockCommandExecutor {
            success: true,
            exit_code: None,
        });
        let connection = SshConnection::with_executor(
            "/usr/local/bin/ssh".to_string(),
            Duration::from_secs(30),
            executor,
        );

        let result = connection.connect("test-host");
        assert!(result.is_ok());
    }
}
