use std::fs;
use std::io::Write;
use std::process::Command;

const WINDOWS_EXPORTER_VERSION: &str = "0.25.1";
const WINDOWS_EXPORTER_PORT: u16 = 31415;

#[cfg(target_os = "windows")]
const EMBEDDED_WINDOWS_EXPORTER: Option<&[u8]> = Some(include_bytes!(concat!(
    env!("OUT_DIR"),
    "/windows_exporter.msi"
)));
#[cfg(not(target_os = "windows"))]
const EMBEDDED_WINDOWS_EXPORTER: Option<&[u8]> = None;

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
        if self.version == WINDOWS_EXPORTER_VERSION
            && let Some(bytes) = EMBEDDED_WINDOWS_EXPORTER
        {
            self.write_installer(bytes)?;
            return Ok(());
        }

        let url = self.download_url(arch);
        println!("Downloading from: {url}");

        let response = reqwest::blocking::get(&url)?;

        if !response.status().is_success() {
            return Err(format!("Failed to download: HTTP {}", response.status()).into());
        }

        let bytes = response.bytes()?;
        self.write_installer(&bytes)?;
        Ok(())
    }

    fn write_installer(&self, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let installer_path = format!("{}\\windows_exporter.msi", self.install_path);
        let mut file = fs::File::create(&installer_path)?;
        file.write_all(bytes)?;
        println!("Windows Exporter installer prepared at: {installer_path}");
        Ok(())
    }

    fn install_msi(&self) -> Result<(), Box<dyn std::error::Error>> {
        let installer_path = format!("{}\\windows_exporter.msi", self.install_path);

        println!("Installing Windows Exporter...");

        // Follow upstream MSI semantics: properties are passed as
        //   ENABLED_COLLECTORS=...
        //   LISTEN_PORT=...
        // without extra quoting, matching README examples.
        let collectors_arg = "ENABLED_COLLECTORS=cpu,cs,logical_disk,net,os,service,system,textfile,process,memory,thermalzone";

        let output = Command::new("msiexec")
            .args([
                "/i",
                &installer_path,
                "/quiet",
                "/norestart",
                &format!("LISTEN_PORT={}", WINDOWS_EXPORTER_PORT),
                collectors_arg,
            ])
            .output()?;

        if output.status.success() {
            println!("Windows Exporter installed successfully");
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Installation failed: {error}").into());
        }

        Ok(())
    }

    fn configure_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Configuring Windows Exporter service...");

        Command::new("sc")
            .args(["config", "windows_exporter", "start=auto"])
            .output()?;

        let output = Command::new("sc")
            .args(["start", "windows_exporter"])
            .output()?;

        if output.status.success() {
            println!("Windows Exporter service started successfully");
            println!("Metrics available at: http://localhost:{WINDOWS_EXPORTER_PORT}/metrics");
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
    - thermalzone
    - tcp
    - iis

# Collector-specific configuration
collector:
  service:
    services-where: "Name='windows_exporter' OR Name='prometheus'"
  process:
    processes-where: "Name LIKE 'chrome%' OR Name = 'firefox'"
"#;

        // Use proper path separator based on OS
        let config_path = if cfg!(windows) {
            format!("{}\\windows_exporter.yml", self.install_path)
        } else {
            format!("{}/windows_exporter.yml", self.install_path)
        };

        let mut file = fs::File::create(&config_path)?;
        file.write_all(config_content.as_bytes())?;

        println!("Configuration file created at: {config_path}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_windows_exporter_creation() {
        let setup = WindowsExporterSetup::new();
        assert_eq!(setup.version, "0.25.1");
        assert_eq!(setup.install_path, "C:\\Program Files\\prometheus");
    }

    #[test]
    fn test_download_url_generation() {
        let setup = WindowsExporterSetup::new();

        let url_amd64 = setup.download_url("x86_64");
        assert!(url_amd64.contains("windows_exporter-0.25.1-amd64.msi"));
        assert!(url_amd64.starts_with(
            "https://github.com/prometheus-community/windows_exporter/releases/download/"
        ));

        let url_386 = setup.download_url("x86");
        assert!(url_386.contains("windows_exporter-0.25.1-386.msi"));
    }

    #[test]
    fn test_arch_mapping() {
        let setup = WindowsExporterSetup::new();

        // Test x86_64 maps to amd64
        let url = setup.download_url("x86_64");
        assert!(url.contains("amd64"));

        // Test other architectures map to 386
        let url = setup.download_url("x86");
        assert!(url.contains("386"));
    }

    #[test]
    fn test_create_directories() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");

        let mut setup = WindowsExporterSetup::new();
        setup.install_path = test_path.to_str().unwrap().to_string();

        let result = setup.create_directories();
        assert!(result.is_ok());
        assert!(test_path.exists());
    }

    #[test]
    fn test_version_constant() {
        assert_eq!(WINDOWS_EXPORTER_VERSION, "0.25.1");
    }

    #[test]
    fn test_port_constant() {
        assert_eq!(WINDOWS_EXPORTER_PORT, 31415);
    }

    #[test]
    fn test_create_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");
        fs::create_dir_all(&test_path).unwrap();

        let mut setup = WindowsExporterSetup::new();
        setup.install_path = test_path.to_str().unwrap().to_string();

        let result = setup.create_config_file();
        assert!(result.is_ok());

        // Check file exists with proper path separator
        let config_path = test_path.join("windows_exporter.yml");
        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("Windows Exporter Configuration"));
        assert!(content.contains("collectors:"));
        assert!(content.contains("cpu"));
        assert!(content.contains("memory"));
        assert!(content.contains("logical_disk"));
    }

    #[test]
    fn test_installer_path() {
        let setup = WindowsExporterSetup::new();
        let installer_path = format!("{}\\windows_exporter.msi", setup.install_path);

        assert!(installer_path.contains("windows_exporter.msi"));
        assert!(installer_path.contains(&setup.install_path));
    }

    #[test]
    fn test_download_installer_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");
        fs::create_dir_all(&test_path).unwrap();

        let mut setup = WindowsExporterSetup::new();
        setup.install_path = test_path.to_str().unwrap().to_string();
        // Use an invalid version that will cause 404
        setup.version = "99.99.99".to_string();

        let result = setup.download_installer("x86_64");
        // GitHub will return 404 for non-existent version
        assert!(result.is_err());
    }
}
