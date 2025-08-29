use crate::exports_downloader::ExporterDownloader;
use std::fs;
use std::process::Command;

const PROCESS_CPU_AGENT_PORT: u16 = 9256;

pub struct ProcessCpuAgentSetup {
    install_path: String,
    download_url: String,
    downloader: ExporterDownloader,
}

impl ProcessCpuAgentSetup {
    pub fn new(download_url: Option<String>) -> Self {
        let default_url = match std::env::consts::OS {
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
        };

        Self {
            #[cfg(windows)]
            install_path: "C:\\Program Files\\prometheus\\process-cpu-agent".to_string(),
            #[cfg(not(windows))]
            install_path: "/opt/prometheus/process-cpu-agent".to_string(),
            download_url: download_url.unwrap_or(default_url),
            downloader: ExporterDownloader::new(),
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
        fs::create_dir_all(&self.install_path)?;
        Ok(())
    }

    fn download_binary(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(windows)]
        let target_binary = format!("{}\\process-cpu-agent.exe", self.install_path);
        #[cfg(not(windows))]
        let target_binary = format!("{}/process-cpu-agent", self.install_path);

        // Download the binary
        self.downloader
            .download_file(&self.download_url, &target_binary)?;

        println!("Process CPU Agent binary downloaded to: {target_binary}");

        Ok(())
    }

    #[cfg(not(windows))]
    fn setup_linux_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let service_content = format!(
            r#"[Unit]
Description=Process CPU Agent for Prometheus
After=network.target

[Service]
Type=simple
ExecStart={}/process-cpu-agent --port {}
Restart=always
RestartSec=10
User=prometheus
Group=prometheus

[Install]
WantedBy=multi-user.target"#,
            self.install_path, PROCESS_CPU_AGENT_PORT
        );

        let service_path = "/etc/systemd/system/process-cpu-agent.service";
        fs::write(service_path, service_content)?;

        println!("Systemd service created at: {service_path}");

        // Reload systemd
        Command::new("systemctl").args(["daemon-reload"]).output()?;

        Ok(())
    }

    #[cfg(windows)]
    fn setup_windows_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let binary_path = format!("{}\\process-cpu-agent.exe", self.install_path);

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
                &format!(
                    "binPath= \"{} --port {}\"",
                    binary_path, PROCESS_CPU_AGENT_PORT
                ),
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

    pub fn create_config_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_content = format!(
            r#"# Process CPU Agent Configuration
# This agent collects CPU usage metrics for individual processes

# Listening port
port: {PROCESS_CPU_AGENT_PORT}

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
        );

        #[cfg(windows)]
        let config_path = format!("{}\\config.yaml", self.install_path);
        #[cfg(not(windows))]
        let config_path = format!("{}/config.yaml", self.install_path);

        fs::write(&config_path, config_content)?;
        println!("Configuration file created at: {config_path}");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_default_url_selection() {
        let setup = ProcessCpuAgentSetup::new(None);
        
        match std::env::consts::OS {
            "windows" => {
                assert!(setup.download_url.contains("windows"));
                assert!(setup.download_url.ends_with(".exe"));
            }
            "linux" => {
                assert!(setup.download_url.contains("linux"));
            }
            "macos" => {
                assert!(setup.download_url.contains("darwin"));
            }
            _ => {}
        }
    }

    #[test]
    fn test_install_path_by_os() {
        let setup = ProcessCpuAgentSetup::new(None);
        
        #[cfg(windows)]
        assert!(setup.install_path.starts_with("C:\\"));
        
        #[cfg(not(windows))]
        assert!(setup.install_path.starts_with("/opt/"));
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
        assert!(content.contains("port: 9256"));
        assert!(content.contains("Process CPU Agent Configuration"));
    }

    #[test]
    fn test_download_binary_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");
        fs::create_dir_all(&test_path).unwrap();
        
        // Use a URL that will definitely fail
        let mut setup = ProcessCpuAgentSetup::new(Some("http://192.0.2.1:9999/agent".to_string()));
        setup.install_path = test_path.to_str().unwrap().to_string();
        
        let result = setup.download_binary();
        assert!(result.is_err());
    }

    #[test]
    fn test_port_constant() {
        assert_eq!(PROCESS_CPU_AGENT_PORT, 9256);
    }
}
