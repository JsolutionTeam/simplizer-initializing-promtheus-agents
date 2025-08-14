use std::fs;
use std::io::Write;
use std::process::Command;

const WINDOWS_EXPORTER_VERSION: &str = "0.25.1";
const WINDOWS_EXPORTER_PORT: u16 = 9182;

pub struct WindowsExporterSetup {
    version: String,
    install_path: String,
}

impl WindowsExporterSetup {
    pub fn new() -> Self {
        Self {
            version: WINDOWS_EXPORTER_VERSION.to_string(),
            install_path: "C:\\Program Files\\prometheus".to_string(),
        }
    }

    pub fn download_url(&self, arch: &str) -> String {
        let arch_suffix = if arch == "x86_64" { "amd64" } else { "386" };
        format!(
            "https://github.com/prometheus-community/windows_exporter/releases/download/v{}/windows_exporter-{}-{}.msi",
            self.version, self.version, arch_suffix
        )
    }

    pub fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Setting up Windows Exporter v{}", self.version);

        let arch = crate::os_detector::get_arch();

        self.create_directories()?;
        self.download_installer(arch)?;
        self.install_msi()?;
        self.configure_service()?;

        Ok(())
    }

    fn create_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&self.install_path)?;
        Ok(())
    }

    fn download_installer(&self, arch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let url = self.download_url(arch);
        println!("Downloading from: {}", url);

        let response = reqwest::blocking::get(&url)?;
        let bytes = response.bytes()?;

        let installer_path = format!("{}\\windows_exporter.msi", self.install_path);
        let mut file = fs::File::create(&installer_path)?;
        file.write_all(&bytes)?;

        println!(
            "Windows Exporter installer downloaded to: {}",
            installer_path
        );
        Ok(())
    }

    fn install_msi(&self) -> Result<(), Box<dyn std::error::Error>> {
        let installer_path = format!("{}\\windows_exporter.msi", self.install_path);

        println!("Installing Windows Exporter...");

        let output = Command::new("msiexec")
            .args(&[
                "/i",
                &installer_path,
                "/quiet",
                "/norestart",
                &format!("INSTALLDIR={}", self.install_path),
                "ENABLED_COLLECTORS=cpu,cs,logical_disk,net,os,service,system,textfile,process,memory"
            ])
            .output()?;

        if output.status.success() {
            println!("Windows Exporter installed successfully");
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Installation failed: {}", error).into());
        }

        Ok(())
    }

    fn configure_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Configuring Windows Exporter service...");

        Command::new("sc")
            .args(&["config", "windows_exporter", "start=auto"])
            .output()?;

        let output = Command::new("sc")
            .args(&["start", "windows_exporter"])
            .output()?;

        if output.status.success() {
            println!("Windows Exporter service started successfully");
            println!(
                "Metrics available at: http://localhost:{}/metrics",
                WINDOWS_EXPORTER_PORT
            );
        } else {
            println!("Please start the service manually: sc start windows_exporter");
        }

        Ok(())
    }

    pub fn create_config_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_content = r#"# Windows Exporter Configuration
# Collectors to enable
collectors:
  enabled:
    - cpu
    - cs
    - logical_disk
    - net
    - os
    - service
    - system
    - textfile
    - process
    - memory
    - tcp
    - iis

# Collector-specific configuration
collector:
  service:
    services-where: "Name='windows_exporter' OR Name='prometheus'"
  process:
    processes-where: "Name LIKE 'chrome%' OR Name = 'firefox'"
"#;

        let config_path = format!("{}\\windows_exporter.yml", self.install_path);
        let mut file = fs::File::create(&config_path)?;
        file.write_all(config_content.as_bytes())?;

        println!("Configuration file created at: {}", config_path);
        Ok(())
    }
}
