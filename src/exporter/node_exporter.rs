use crate::exporter::downloader;
use std::path::Path;
use std::process::Command;

const NODE_EXPORTER_VERSION: &str = "1.7.0";
const NODE_EXPORTER_PORT: u16 = 31415;

#[cfg(target_os = "linux")]
const EMBEDDED_NODE_EXPORTER_ARCHIVE: Option<&[u8]> = Some(include_bytes!(concat!(
    env!("OUT_DIR"),
    "/node_exporter.tar.gz"
)));
#[cfg(not(target_os = "linux"))]
const EMBEDDED_NODE_EXPORTER_ARCHIVE: Option<&[u8]> = None;

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
        self.create_systemd_service(arch)?;

        Ok(())
    }

    fn create_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        downloader::ensure_directory_exists(&self.install_path)
    }

    fn download_and_extract(&self, arch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let extract_path = format!("{}/node_exporter", self.install_path);

        if self.version == NODE_EXPORTER_VERSION
            && let Some(bytes) = EMBEDDED_NODE_EXPORTER_ARCHIVE {
                downloader::extract_tar_gz(bytes, &extract_path)?;
                return Ok(());
            }

        let url = self.download_url(arch);
        downloader::download_and_extract_tar_gz(&url, &extract_path)?;

        Ok(())
    }

    fn create_systemd_service(&self, arch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let service_content =
            create_systemd_service_content(&self.install_path, &self.version, arch);
        let service_path = "/etc/systemd/system/node_exporter.service";

        if Path::new("/etc/systemd/system").exists() {
            downloader::write_file(service_path, service_content.as_bytes())?;
            println!("Systemd service created at: {service_path}");

            Command::new("systemctl").args(["daemon-reload"]).output()?;

            println!("Run 'systemctl enable --now node_exporter' to start the service");
        } else {
            println!("Systemd not found. Please manually configure the service.");
        }

        Ok(())
    }
}

/// Create systemd service content for Node Exporter
pub fn create_systemd_service_content(install_path: &str, version: &str, arch: &str) -> String {
    format!(
        r#"[Unit]
Description=Prometheus Node Exporter
After=network.target

[Service]
Type=simple
ExecStart={install_path}/node_exporter/node_exporter-{version}.linux-{arch}/node_exporter --web.listen-address=:31415
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
"#
    )
}

/// Generate download URL for Node Exporter
pub fn generate_download_url(version: &str, arch: &str) -> String {
    format!(
        "https://github.com/prometheus/node_exporter/releases/download/v{version}/node_exporter-{version}.linux-{arch}.tar.gz"
    )
}

/// Get architecture string for Node Exporter
pub fn get_node_exporter_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "arm" | "armv7l" => "armv7",
        "i686" | "i586" | "x86" => "386",
        _ => {
            if crate::os_detector::is_64bit() {
                "amd64"
            } else {
                "386"
            }
        }
    }
}

/// Setup Node Exporter with custom parameters
pub fn setup_node_exporter(
    version: &str,
    install_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Setting up Node Exporter v{version}");

    let arch = get_node_exporter_arch();
    let extract_path = format!("{install_path}/node_exporter");

    // Create directories
    downloader::ensure_directory_exists(install_path)?;

    if version == NODE_EXPORTER_VERSION {
        if let Some(bytes) = EMBEDDED_NODE_EXPORTER_ARCHIVE {
            downloader::extract_tar_gz(bytes, &extract_path)?;
        } else {
            let url = generate_download_url(version, arch);
            downloader::download_and_extract_tar_gz(&url, &extract_path)?;
        }
    } else {
        let url = generate_download_url(version, arch);
        downloader::download_and_extract_tar_gz(&url, &extract_path)?;
    }

    // Create systemd service
    let service_content = create_systemd_service_content(install_path, version, arch);
    let service_path = "/etc/systemd/system/node_exporter.service";

    if Path::new("/etc/systemd/system").exists() {
        downloader::write_file(service_path, service_content.as_bytes())?;
        println!("Systemd service created at: {service_path}");

        Command::new("systemctl").args(["daemon-reload"]).output()?;
        println!("Run 'systemctl enable --now node_exporter' to start the service");
    } else {
        println!("Systemd not found. Please manually configure the service.");
    }

    Ok(())
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
        assert!(
            url_amd64.starts_with("https://github.com/prometheus/node_exporter/releases/download/")
        );

        let url_386 = setup.download_url("386");
        assert!(url_386.contains("node_exporter-1.7.0.linux-386.tar.gz"));
    }

    #[test]
    fn test_generate_download_url_function() {
        let url = generate_download_url("1.7.0", "amd64");
        assert!(url.contains("node_exporter-1.7.0.linux-amd64.tar.gz"));
        assert!(url.starts_with("https://github.com/prometheus/node_exporter/releases/download/"));
    }

    #[test]
    fn test_arch_selection() {
        let arch = get_node_exporter_arch();

        match std::env::consts::ARCH {
            "x86_64" => assert_eq!(arch, "amd64"),
            "aarch64" => assert_eq!(arch, "arm64"),
            "arm" | "armv7l" => assert_eq!(arch, "armv7"),
            _ => {
                if crate::os_detector::is_64bit() {
                    assert_eq!(arch, "amd64");
                } else {
                    assert_eq!(arch, "386");
                }
            }
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
        assert_eq!(NODE_EXPORTER_PORT, 31415);
    }

    #[test]
    fn test_systemd_service_content_function() {
        let content = create_systemd_service_content("/opt/prometheus", "1.7.0", "amd64");

        assert!(content.contains("Description=Prometheus Node Exporter"));
        assert!(content.contains("1.7.0"));
        assert!(content.contains("WantedBy=multi-user.target"));
        assert!(content.contains("/opt/prometheus"));
    }

    #[test]
    fn test_setup_node_exporter_invalid_version() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_prometheus");

        let result = setup_node_exporter("99.99.99", test_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_systemd_service_content() {
        let setup = NodeExporterSetup::new();
        let service_content =
            create_systemd_service_content(&setup.install_path, &setup.version, "amd64");

        assert!(service_content.contains("Description=Prometheus Node Exporter"));
        assert!(service_content.contains(&setup.version));
        assert!(service_content.contains("WantedBy=multi-user.target"));
    }
}
