use crate::exporter::downloader;
use std::fs;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
const PROCESS_CPU_AGENT_PORT: u16 = 31416;
const EMBEDDED_PROCESS_AGENT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/process_cpu_agent.bin"));
const EMBEDDED_PROCESS_AGENT_CONFIG: &str = include_str!("../../lib/process-cpu-agent-config.toml");

#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x00000008;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, PartialEq)]
enum AgentSource {
    Embedded,
    Remote(String),
}

pub struct ProcessCpuAgentSetup {
    install_path: String,
    source: AgentSource,
}
impl ProcessCpuAgentSetup {
    pub fn new(download_url: Option<String>) -> Self {
        let source = match download_url {
            Some(url) if !url.trim().is_empty() => AgentSource::Remote(url),
            _ => AgentSource::Embedded,
        };

        Self {
            install_path: get_default_install_path(),
            source,
        }
    }
    pub fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Setting up Process CPU Agent...");
        match &self.source {
            AgentSource::Embedded => println!("Using embedded Process CPU Agent binary"),
            AgentSource::Remote(url) => println!("Download URL: {url}"),
        }

        self.create_directories()?;
        self.write_binary()?;
        // Ensure configuration file exists before wiring services so that
        // the agent can start with a valid config on first run.
        self.create_config_file()?;
        #[cfg(windows)]
        {
            self.setup_windows_service()?;
        }

        #[cfg(not(windows))]
        {
            self.setup_linux_service()?;
        }

        Ok(())
    }

    fn create_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        downloader::ensure_directory_exists(&self.install_path)
    }
    fn write_binary(&self) -> Result<(), Box<dyn std::error::Error>> {
        let target_binary = get_binary_path(&self.install_path);
        match &self.source {
            AgentSource::Embedded => {
                if let Some(parent) = std::path::Path::new(&target_binary).parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&target_binary, EMBEDDED_PROCESS_AGENT)?;
                println!(
                    "Process CPU Agent binary written from embedded artifact: {target_binary}"
                );
            }
            AgentSource::Remote(url) => {
                downloader::download_file(url, &target_binary)?;
                println!("Process CPU Agent binary downloaded to: {target_binary}");
            }
        }
        Ok(())
    }
    #[cfg(not(windows))]
    fn setup_linux_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let service_content =
            create_linux_service_content(&self.install_path, PROCESS_CPU_AGENT_PORT);
        let service_path = "/etc/systemd/system/process-cpu-agent.service";

        downloader::write_file(service_path, service_content.as_bytes())?;
        println!("Systemd service created at: {service_path}");

        Command::new("systemctl").args(["daemon-reload"]).output()?;
        Command::new("systemctl")
            .args(["enable", "--now", "process-cpu-agent"])
            .output()?;
        println!("Process CPU Agent service enabled and started");

        Ok(())
    }

    #[cfg(windows)]
    fn setup_windows_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        setup_windows_service(&self.install_path, PROCESS_CPU_AGENT_PORT)
    }
}

impl ProcessCpuAgentSetup {
    pub fn create_config_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = get_config_path(&self.install_path);

        if let Some(parent) = std::path::Path::new(&config_path).parent() {
            fs::create_dir_all(parent)?;
        }

        downloader::write_file(&config_path, EMBEDDED_PROCESS_AGENT_CONFIG.as_bytes())?;
        println!("Configuration file created at: {config_path}");

        Ok(())
    }
}

/// Get default install path based on OS
pub fn get_default_install_path() -> String {
    #[cfg(windows)]
    {
        // Prefer per-user install under LOCALAPPDATA to avoid Program Files write restrictions.
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return format!("{local_app_data}\\prometheus\\process-cpu-agent");
        }

        // Fallback to a machine-wide writable location if LOCALAPPDATA is unavailable.
        "C:\\ProgramData\\prometheus\\process-cpu-agent".to_string()
    }

    #[cfg(not(windows))]
    {
        "/opt/prometheus/process-cpu-agent".to_string()
    }
}

/// Get binary path based on install path and OS
pub fn get_binary_path(install_path: &str) -> String {
    #[cfg(windows)]
    return format!("{}\\process-cpu-agent.exe", install_path);

    #[cfg(not(windows))]
    return format!("{install_path}/process-cpu-agent");
}

/// Get config file path based on install path and OS
pub fn get_config_path(install_path: &str) -> String {
    #[cfg(windows)]
    return format!("{}\\config.toml", install_path);

    #[cfg(not(windows))]
    return format!("{install_path}/config.toml");
}

/// Create Linux systemd service content
#[cfg(not(windows))]
pub fn create_linux_service_content(install_path: &str, port: u16) -> String {
    format!(
        r#"[Unit]
Description=Process CPU Agent for Prometheus
After=network.target

[Service]
Type=simple
ExecStart={install_path}/process-cpu-agent --port {port}
Restart=always
RestartSec=10
User=prometheus
Group=prometheus

[Install]
WantedBy=multi-user.target"#
    )
}

/// Setup Windows scheduled task (Windows Task Scheduler)
#[cfg(windows)]
pub fn setup_windows_service(
    install_path: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = port; // port is configured via config.toml; CLI arg is not needed on Windows
    let binary_path = get_binary_path(install_path);
    println!("Creating Windows scheduled task...");

    // Register a Task Scheduler job that runs the agent at user logon
    // under the current user account.
    let task_name = "ProcessCpuAgent";

    let task_run = format!("cmd.exe /C cd /d {} && {}", install_path, binary_path);

    let output = Command::new("schtasks")
        .args([
            "/Create", "/TN", task_name, "/SC", "ONLOGON", "/F", "/TR", &task_run,
        ])
        .output()?;

    if output.status.success() {
        println!("Windows scheduled task registered successfully");

        // 설치 직후 한 번 바로 실행 시도: 작업 스케줄러 정의는 그대로 두고,
        // 바이너리를 현재 콘솔/프로세스와 완전히 분리된(detached) 프로세스로 실행한다.
        let spawn_result = Command::new(&binary_path)
            .current_dir(install_path)
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
            .spawn();

        match spawn_result {
            Ok(_) => {
                println!("ProcessCpuAgent started immediately after installation");
            }
            Err(e) => {
                println!("Warning: Failed to start ProcessCpuAgent immediately: {e}");
            }
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Failed to create Windows scheduled task: {}\n{}",
            stderr.trim(),
            stdout.trim()
        )
        .into());
    }

    Ok(())
}

/// Setup Process CPU Agent with custom parameters
pub fn setup_process_cpu_agent(
    download_url: Option<String>,
    install_path: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut setup = ProcessCpuAgentSetup::new(download_url);
    if let Some(path) = install_path {
        setup.install_path = path;
    }

    setup.create_config_file()?;
    setup.setup()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_process_cpu_agent_setup_creation() {
        let setup = ProcessCpuAgentSetup::new(None);
        assert_eq!(setup.source, AgentSource::Embedded);
        assert!(!setup.install_path.is_empty());
    }

    #[test]
    fn test_write_binary_from_embedded() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");
        fs::create_dir_all(&test_path).unwrap();

        let mut setup = ProcessCpuAgentSetup::new(None);
        setup.install_path = test_path.to_str().unwrap().to_string();

        setup.write_binary().unwrap();
        let binary_path = PathBuf::from(get_binary_path(&setup.install_path));
        assert!(binary_path.exists());
        let content = fs::read(binary_path).unwrap();
        assert_eq!(content, EMBEDDED_PROCESS_AGENT);
    }

    #[test]
    fn test_custom_download_url() {
        let custom_url = "https://example.com/custom-agent.exe".to_string();
        let setup = ProcessCpuAgentSetup::new(Some(custom_url.clone()));
        assert_eq!(setup.source, AgentSource::Remote(custom_url));
    }

    #[test]
    fn test_get_default_install_path() {
        let path = get_default_install_path();

        #[cfg(windows)]
        assert!(path.starts_with("C:\\"));

        #[cfg(not(windows))]
        assert!(path.starts_with("/opt/"));
    }

    #[test]
    fn test_get_binary_path() {
        let install_path = "/opt/prometheus";
        let binary_path = get_binary_path(install_path);

        #[cfg(windows)]
        assert!(binary_path.ends_with(".exe"));

        #[cfg(not(windows))]
        assert_eq!(binary_path, "/opt/prometheus/process-cpu-agent");
    }

    #[test]
    fn test_get_config_path() {
        let install_path = "/opt/prometheus";
        let config_path = get_config_path(install_path);

        assert!(config_path.contains("config.toml"));
    }

    #[test]
    fn test_create_directories() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");

        let mut setup = ProcessCpuAgentSetup::new(None);
        setup.install_path = test_path.to_str().unwrap().to_string();

        let result = setup.create_directories();
        assert!(result.is_ok());
        assert!(test_path.exists());
    }

    #[test]
    fn test_create_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");
        fs::create_dir_all(&test_path).unwrap();

        let mut setup = ProcessCpuAgentSetup::new(None);
        setup.install_path = test_path.to_str().unwrap().to_string();

        let result = setup.create_config_file();
        assert!(result.is_ok());

        let config_path = test_path.join("config.toml");
        assert!(config_path.exists());

        let content = fs::read_to_string(config_path).unwrap();
        assert!(content.contains("[server]"));
        assert!(content.contains("port = 31416"));
        assert!(content.contains("[process]"));
    }

    #[cfg(not(windows))]
    #[test]
    fn test_create_linux_service_content() {
        let content = create_linux_service_content("/opt/prometheus", 31416);

        assert!(content.contains("Description=Process CPU Agent for Prometheus"));
        assert!(content.contains("/opt/prometheus"));
        assert!(content.contains("--port 31416"));
        assert!(content.contains("WantedBy=multi-user.target"));
    }
    #[test]
    fn test_write_binary_with_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");
        fs::create_dir_all(&test_path).unwrap();

        let mut setup = ProcessCpuAgentSetup::new(Some("http://192.0.2.1:9999/agent".to_string()));
        setup.install_path = test_path.to_str().unwrap().to_string();

        let result = setup.write_binary();
        assert!(result.is_err());
    }
    #[test]
    fn test_port_constant() {
        assert_eq!(PROCESS_CPU_AGENT_PORT, 31416);
    }

    #[test]
    fn test_setup_process_cpu_agent_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");

        let result = setup_process_cpu_agent(
            Some("http://192.0.2.1:9999/agent".to_string()),
            Some(test_path.to_str().unwrap().to_string()),
        );
        assert!(result.is_err());
    }
}
