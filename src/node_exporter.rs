use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

const NODE_EXPORTER_VERSION: &str = "1.7.0";
const NODE_EXPORTER_PORT: u16 = 9100;

pub struct NodeExporterSetup {
    version: String,
    install_path: String,
}

impl NodeExporterSetup {
    pub fn new() -> Self {
        Self {
            version: NODE_EXPORTER_VERSION.to_string(),
            install_path: "/opt/prometheus".to_string(),
        }
    }

    pub fn download_url(&self, arch: &str) -> String {
        format!(
            "https://github.com/prometheus/node_exporter/releases/download/v{}/node_exporter-{}.linux-{}.tar.gz",
            self.version, self.version, arch
        )
    }

    pub fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Setting up Node Exporter v{}", self.version);

        let arch = if crate::os_detector::is_64bit() {
            "amd64"
        } else {
            "386"
        };

        self.create_directories()?;
        self.download_and_extract(arch)?;
        self.create_systemd_service()?;

        Ok(())
    }

    fn create_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&self.install_path)?;
        Ok(())
    }

    fn download_and_extract(&self, arch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let url = self.download_url(arch);
        println!("Downloading from: {url}");

        let response = reqwest::blocking::get(&url)?;
        let bytes = response.bytes()?;

        let tar_gz = flate2::read::GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(tar_gz);

        let extract_path = format!("{}/node_exporter", self.install_path);
        fs::create_dir_all(&extract_path)?;
        archive.unpack(&extract_path)?;

        println!("Node Exporter extracted to: {extract_path}");
        Ok(())
    }

    fn create_systemd_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let service_content = format!(
            r#"[Unit]
Description=Prometheus Node Exporter
After=network.target

[Service]
Type=simple
ExecStart={}/node_exporter/node_exporter-{}.linux-amd64/node_exporter
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
"#,
            self.install_path, self.version
        );

        let service_path = "/etc/systemd/system/node_exporter.service";

        if Path::new("/etc/systemd/system").exists() {
            let mut file = fs::File::create(service_path)?;
            file.write_all(service_content.as_bytes())?;
            println!("Systemd service created at: {service_path}");

            Command::new("systemctl").args(["daemon-reload"]).output()?;

            println!("Run 'systemctl enable --now node_exporter' to start the service");
        } else {
            println!("Systemd not found. Please manually configure the service.");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_node_exporter_creation() {
        let setup = NodeExporterSetup::new();
        assert_eq!(setup.version, "1.7.0");
        assert_eq!(setup.install_path, "/opt/prometheus");
    }

    #[test]
    fn test_download_url_generation() {
        let setup = NodeExporterSetup::new();
        
        let url_amd64 = setup.download_url("amd64");
        assert!(url_amd64.contains("node_exporter-1.7.0.linux-amd64.tar.gz"));
        assert!(url_amd64.starts_with("https://github.com/prometheus/node_exporter/releases/download/"));
        
        let url_386 = setup.download_url("386");
        assert!(url_386.contains("node_exporter-1.7.0.linux-386.tar.gz"));
    }

    #[test]
    fn test_arch_selection() {
        let setup = NodeExporterSetup::new();
        
        // Test URL generation for 64-bit
        if crate::os_detector::is_64bit() {
            let url = setup.download_url("amd64");
            assert!(url.contains("amd64"));
        } else {
            let url = setup.download_url("386");
            assert!(url.contains("386"));
        }
    }

    #[test]
    fn test_create_directories() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");
        
        let mut setup = NodeExporterSetup::new();
        setup.install_path = test_path.to_str().unwrap().to_string();
        
        let result = setup.create_directories();
        assert!(result.is_ok());
        assert!(test_path.exists());
    }

    #[test]
    fn test_version_constant() {
        assert_eq!(NODE_EXPORTER_VERSION, "1.7.0");
    }

    #[test]
    fn test_port_constant() {
        assert_eq!(NODE_EXPORTER_PORT, 9100);
    }

    #[test]
    fn test_systemd_service_content() {
        let setup = NodeExporterSetup::new();
        let service_content = format!(
            r#"[Unit]
Description=Prometheus Node Exporter
After=network.target

[Service]
Type=simple
ExecStart={}/node_exporter/node_exporter-{}.linux-amd64/node_exporter
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
"#,
            setup.install_path, setup.version
        );
        
        assert!(service_content.contains("Description=Prometheus Node Exporter"));
        assert!(service_content.contains(&setup.version));
        assert!(service_content.contains("WantedBy=multi-user.target"));
    }

    #[test]
    fn test_download_and_extract_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");
        
        let mut setup = NodeExporterSetup::new();
        setup.install_path = test_path.to_str().unwrap().to_string();
        setup.version = "invalid.version".to_string();
        
        let result = setup.download_and_extract("amd64");
        assert!(result.is_err());
    }
}
