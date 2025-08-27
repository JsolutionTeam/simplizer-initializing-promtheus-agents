use std::fs;
use std::path::Path;
use std::process::Command;

const PROCESS_CPU_AGENT_PORT: u16 = 9256;

pub struct ProcessCpuAgentSetup {
    install_path: String,
}

impl ProcessCpuAgentSetup {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            install_path: "C:\\Program Files\\prometheus\\process-cpu-agent".to_string(),
            #[cfg(not(windows))]
            install_path: "/opt/prometheus/process-cpu-agent".to_string(),
        }
    }

    pub fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Setting up Process CPU Agent...");

        self.create_directories()?;
        self.copy_binary()?;

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

    fn copy_binary(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(windows)]
        let target_binary = format!("{}\\process-cpu-agent.exe", self.install_path);
        #[cfg(not(windows))]
        let target_binary = format!("{}/process-cpu-agent", self.install_path);

        use std::io::Write;
        let mut file = fs::File::create(&target_binary)?;

        #[cfg(windows)]
        file.write_all(include_bytes!("../lib/process-cpu-agent.exe"))?;
        #[cfg(not(windows))]
        file.write_all(include_bytes!("../lib/process-cpu-agent"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&target_binary)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&target_binary, permissions)?;
        }

        println!("Process CPU Agent binary extracted to: {target_binary}");

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
WantedBy=multi-user.target
"#,
            self.install_path, PROCESS_CPU_AGENT_PORT
        );

        let service_path = "/etc/systemd/system/process-cpu-agent.service";

        if Path::new("/etc/systemd/system").exists() {
            let mut file = fs::File::create(service_path)?;
            use std::io::Write;
            file.write_all(service_content.as_bytes())?;

            println!("Systemd service created at: {service_path}");

            Command::new("systemctl")
                .args(["daemon-reload"])
                .output()?;

            println!("Run 'systemctl enable --now process-cpu-agent' to start the service");
        } else {
            println!("Systemd not found. Creating startup script instead...");
            self.create_startup_script()?;
        }

        Ok(())
    }

    #[cfg(windows)]
    fn setup_windows_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let binary_path = format!("{}\\process-cpu-agent.exe", self.install_path);

        println!("Creating Windows service for Process CPU Agent...");

        let _ = Command::new("sc")
            .args(&["delete", "ProcessCpuAgent"])
            .output();

        let output = Command::new("sc")
            .args(&[
                "create",
                "ProcessCpuAgent",
                "binPath=",
                &format!("{} --port {}", binary_path, PROCESS_CPU_AGENT_PORT),
                "DisplayName=",
                "Process CPU Agent",
                "start=",
                "auto",
                "obj=",
                "LocalSystem",
            ])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to create service: {}", error).into());
        }

        Command::new("sc")
            .args(&[
                "description",
                "ProcessCpuAgent",
                "Process CPU monitoring agent for Prometheus",
            ])
            .output()?;

        println!("Windows service created successfully");
        println!("Start the service with: sc start ProcessCpuAgent");

        Ok(())
    }

    #[cfg(not(windows))]
    fn create_startup_script(&self) -> Result<(), Box<dyn std::error::Error>> {
        let script_content = format!(
            r#"#!/bin/bash
# Process CPU Agent startup script

AGENT_PATH="{}/process-cpu-agent"
PORT={}
LOG_FILE="/var/log/process-cpu-agent.log"

echo "Starting Process CPU Agent on port $PORT..." >> $LOG_FILE
nohup $AGENT_PATH --port $PORT >> $LOG_FILE 2>&1 &
echo $! > /var/run/process-cpu-agent.pid
echo "Process CPU Agent started with PID $(cat /var/run/process-cpu-agent.pid)" >> $LOG_FILE
"#,
            self.install_path, PROCESS_CPU_AGENT_PORT
        );

        let script_path = format!("{}/start-agent.sh", self.install_path);
        let mut file = fs::File::create(&script_path)?;
        use std::io::Write;
        file.write_all(script_content.as_bytes())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&script_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script_path, permissions)?;
        }

        println!("Startup script created at: {script_path}");
        println!("Run the script to start the agent: {script_path}");

        Ok(())
    }

    pub fn create_config_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_content = r#"# Process CPU Agent Configuration
port: 9256
interval: 10s
processes:
  - name: "chrome"
    pattern: "chrome.*"
  - name: "firefox"
    pattern: "firefox.*"
  - name: "node"
    pattern: "node.*"
  - name: "python"
    pattern: "python.*"
  - name: "java"
    pattern: "java.*"
metrics:
  enable_cpu: true
  enable_memory: true
  enable_io: true
  enable_network: false
"#;

        #[cfg(windows)]
        let config_path = format!("{}\\config.yml", self.install_path);
        #[cfg(not(windows))]
        let config_path = format!("{}/config.yml", self.install_path);

        let mut file = fs::File::create(&config_path)?;
        use std::io::Write;
        file.write_all(config_content.as_bytes())?;

        println!("Configuration file created at: {config_path}");
        Ok(())
    }
}
