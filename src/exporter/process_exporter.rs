use crate::exporter::downloader;
use std::process::Command;

const PROCESS_CPU_AGENT_PORT: u16 = 31416;

pub struct ProcessCpuAgentSetup {
    install_path: String,
    download_url: String,
}

impl ProcessCpuAgentSetup {
    pub fn new(download_url: Option<String>) -> Self {
        let default_url = generate_default_download_url();

        Self {
            install_path: get_default_install_path(),
            download_url: download_url.unwrap_or(default_url),
        }
    }

    pub fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Setting up Process CPU Agent...");
        println!("Download URL: {}", self.download_url);

        self.create_directories()?;
        self.download_binary()?;

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

    fn download_binary(&self) -> Result<(), Box<dyn std::error::Error>> {
        let target_binary = get_binary_path(&self.install_path);
        downloader::download_file(&self.download_url, &target_binary)?;

        println!("Process CPU Agent binary downloaded to: {target_binary}");
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

        Ok(())
    }

    #[cfg(windows)]
    fn setup_windows_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        setup_windows_service(&self.install_path, PROCESS_CPU_AGENT_PORT)
    }

    pub fn create_config_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_content = create_config_content(PROCESS_CPU_AGENT_PORT);
        let config_path = get_config_path(&self.install_path);

        downloader::write_file(&config_path, config_content.as_bytes())?;
        println!("Configuration file created at: {config_path}");

        Ok(())
    }
}

/// Generate default download URL based on OS and architecture
pub fn generate_default_download_url() -> String {
    match std::env::consts::OS {
        "windows" => "https://github.com/your-org/process-cpu-agent/releases/latest/download/process-cpu-agent-windows-amd64.exe".to_string(),
        "linux" => {
            let arch = if crate::os_detector::is_64bit() {
                "amd64"
            } else {
                "386"
            };
            format!("https://github.com/your-org/process-cpu-agent/releases/latest/download/process-cpu-agent-linux-{arch}")
        }
        "macos" => {
            let arch = if std::env::consts::ARCH == "aarch64" {
                "arm64"
            } else {
                "amd64"
            };
            format!("https://github.com/your-org/process-cpu-agent/releases/latest/download/process-cpu-agent-darwin-{arch}")
        }
        _ => "https://github.com/your-org/process-cpu-agent/releases/latest/download/process-cpu-agent-linux-amd64".to_string(),
    }
}

/// Get default install path based on OS
pub fn get_default_install_path() -> String {
    #[cfg(windows)]
    return "C:\\Program Files\\prometheus\\process-cpu-agent".to_string();

    #[cfg(not(windows))]
    return "/opt/prometheus/process-cpu-agent".to_string();
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
    return format!("{}\\config.yaml", install_path);

    #[cfg(not(windows))]
    return format!("{install_path}/config.yaml");
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

/// Setup Windows service
#[cfg(windows)]
pub fn setup_windows_service(
    install_path: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let binary_path = get_binary_path(install_path);
    println!("Creating Windows service...");

    // Check if service exists
    let check = Command::new("sc")
        .args(["query", "ProcessCpuAgent"])
        .output()?;

    if check.status.success() {
        // Stop and delete existing service
        Command::new("sc")
            .args(["stop", "ProcessCpuAgent"])
            .output()?;

        Command::new("sc")
            .args(["delete", "ProcessCpuAgent"])
            .output()?;
    }

    // Create new service
    let output = Command::new("sc")
        .args([
            "create",
            "ProcessCpuAgent",
            &format!("binPath= \"{} --port {}\"", binary_path, port),
            "DisplayName= \"Process CPU Agent for Prometheus\"",
            "start= auto",
        ])
        .output()?;

    if output.status.success() {
        println!("Windows service created successfully");
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to create Windows service: {}", error).into());
    }

    Ok(())
}

/// Create configuration file content
pub fn create_config_content(port: u16) -> String {
    format!(
        r#"# Process CPU Agent Configuration
# This agent collects CPU usage metrics for individual processes

# Listening port
port: {port}

# Process filters (optional)
# Only monitor processes matching these patterns
# process_filters:
#   - "java"
#   - "python"
#   - "node"

# Collection interval in seconds
interval: 15

# Maximum number of processes to track
max_processes: 100
"#
    )
}

/// Setup Process CPU Agent with custom parameters
pub fn setup_process_cpu_agent(
    download_url: Option<String>,
    install_path: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = download_url.unwrap_or_else(generate_default_download_url);
    let path = install_path.unwrap_or_else(get_default_install_path);

    println!("Setting up Process CPU Agent...");
    println!("Download URL: {url}");

    // Create directories
    downloader::ensure_directory_exists(&path)?;

    // Download binary
    let target_binary = get_binary_path(&path);
    downloader::download_file(&url, &target_binary)?;
    println!("Process CPU Agent binary downloaded to: {target_binary}");

    // Setup service
    #[cfg(windows)]
    setup_windows_service(&path, PROCESS_CPU_AGENT_PORT)?;

    #[cfg(not(windows))]
    {
        let service_content = create_linux_service_content(&path, PROCESS_CPU_AGENT_PORT);
        let service_path = "/etc/systemd/system/process-cpu-agent.service";

        downloader::write_file(service_path, service_content.as_bytes())?;
        println!("Systemd service created at: {service_path}");

        Command::new("systemctl").args(["daemon-reload"]).output()?;
    }

    // Create config file
    let config_content = create_config_content(PROCESS_CPU_AGENT_PORT);
    let config_path = get_config_path(&path);

    downloader::write_file(&config_path, config_content.as_bytes())?;
    println!("Configuration file created at: {config_path}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_process_cpu_agent_setup_creation() {
        let setup = ProcessCpuAgentSetup::new(None);
        assert!(!setup.download_url.is_empty());
        assert!(!setup.install_path.is_empty());
    }

    #[test]
    fn test_custom_download_url() {
        let custom_url = "https://example.com/custom-agent.exe".to_string();
        let setup = ProcessCpuAgentSetup::new(Some(custom_url.clone()));
        assert_eq!(setup.download_url, custom_url);
    }

    #[test]
    fn test_generate_default_download_url() {
        let url = generate_default_download_url();

        match std::env::consts::OS {
            "windows" => {
                assert!(url.contains("windows"));
                assert!(url.ends_with(".exe"));
            }
            "linux" => {
                assert!(url.contains("linux"));
            }
            "macos" => {
                assert!(url.contains("darwin"));
            }
            _ => {
                assert!(url.contains("linux"));
            }
        }
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

        assert!(config_path.contains("config.yaml"));
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

        let config_path = test_path.join("config.yaml");
        assert!(config_path.exists());

        let content = fs::read_to_string(config_path).unwrap();
        assert!(content.contains("port: 31416"));
        assert!(content.contains("Process CPU Agent Configuration"));
    }

    #[test]
    fn test_create_config_content() {
        let content = create_config_content(31416);

        assert!(content.contains("port: 31416"));
        assert!(content.contains("Process CPU Agent Configuration"));
        assert!(content.contains("interval: 15"));
        assert!(content.contains("max_processes: 100"));
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
    fn test_download_binary_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");
        fs::create_dir_all(&test_path).unwrap();

        let mut setup = ProcessCpuAgentSetup::new(Some("http://192.0.2.1:9999/agent".to_string()));
        setup.install_path = test_path.to_str().unwrap().to_string();

        let result = setup.download_binary();
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
