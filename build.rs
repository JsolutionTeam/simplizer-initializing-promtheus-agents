use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const NODE_EXPORTER_VERSION: &str = "1.7.0";
const WINDOWS_EXPORTER_VERSION: &str = "0.25.1";

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed=PROCESS_CPU_AGENT_BUILD_FILE");
    println!("cargo:rerun-if-env-changed=PROCESS_CPU_AGENT_BUILD_URL");
    println!("cargo:rerun-if-env-changed=NODE_EXPORTER_BUILD_FILE");
    println!("cargo:rerun-if-env-changed=NODE_EXPORTER_BUILD_URL");
    println!("cargo:rerun-if-env-changed=WINDOWS_EXPORTER_BUILD_FILE");
    println!("cargo:rerun-if-env-changed=WINDOWS_EXPORTER_BUILD_URL");

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let target = TargetInfo::from_triple(&env::var("TARGET")?);

    for artifact in artifacts_for(&target) {
        artifact.ensure(&target, &out_dir)?;
    }

    Ok(())
}

#[derive(Clone)]
struct Artifact {
    kind: ArtifactKind,
    output_name: &'static str,
    env_file: &'static str,
    env_url: &'static str,
}

#[derive(Clone, Copy, PartialEq)]
enum ArtifactKind {
    ProcessCpuAgent,
    NodeExporter,
    WindowsExporter,
}

impl Artifact {
    fn ensure(&self, target: &TargetInfo, out_dir: &Path) -> Result<(), Box<dyn Error>> {
        let dest = out_dir.join(self.output_name);
        if dest.exists() {
            return Ok(());
        }

        if let Ok(path) = env::var(self.env_file) {
            copy_to(Path::new(&path), &dest)?;
            return Ok(());
        }

        if let Some(bundled) = self.bundle_path()? {
            copy_to(&bundled, &dest)?;
            return Ok(());
        }

        if let Ok(url) = env::var(self.env_url) {
            download_to(&url, &dest)?;
            return Ok(());
        }

        let url = self.default_url(target)?;
        download_to(&url, &dest)?;
        Ok(())
    }

    fn bundle_path(&self) -> Result<Option<PathBuf>, Box<dyn Error>> {
        if self.kind != ArtifactKind::ProcessCpuAgent {
            return Ok(None);
        }
        let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
        let binary = if cfg!(windows) {
            manifest.join("lib").join("process-cpu-agent.exe")
        } else {
            manifest.join("lib").join("process-cpu-agent")
        };
        Ok(binary.exists().then_some(binary))
    }

    fn default_url(&self, target: &TargetInfo) -> Result<String, Box<dyn Error>> {
        let url = match self.kind {
            ArtifactKind::ProcessCpuAgent => default_process_cpu_agent_url(target),
            ArtifactKind::NodeExporter => default_node_exporter_url(target),
            ArtifactKind::WindowsExporter => default_windows_exporter_url(target),
        };
        url.ok_or_else(|| format!("No default URL for target {}", target.triple).into())
    }
}

struct TargetInfo {
    triple: String,
    os: TargetOs,
    arch: TargetArch,
}

enum TargetOs {
    Linux,
    Windows,
    Other,
}

enum TargetArch {
    X86_64,
    X86,
    Aarch64,
    Other,
}

impl TargetInfo {
    fn from_triple(triple: &str) -> Self {
        let os = if triple.contains("windows") {
            TargetOs::Windows
        } else if triple.contains("linux") {
            TargetOs::Linux
        } else {
            TargetOs::Other
        };

        let arch = if triple.contains("x86_64") || triple.contains("amd64") {
            TargetArch::X86_64
        } else if triple.contains("i686") || triple.contains("i586") || triple.contains("386") {
            TargetArch::X86
        } else if triple.contains("aarch64") || triple.contains("arm64") {
            TargetArch::Aarch64
        } else {
            TargetArch::Other
        };

        Self {
            triple: triple.to_string(),
            os,
            arch,
        }
    }

    fn is_linux(&self) -> bool {
        matches!(self.os, TargetOs::Linux)
    }

    fn is_windows(&self) -> bool {
        matches!(self.os, TargetOs::Windows)
    }
}

fn artifacts_for(target: &TargetInfo) -> Vec<Artifact> {
    let mut list = vec![Artifact {
        kind: ArtifactKind::ProcessCpuAgent,
        output_name: "process_cpu_agent.bin",
        env_file: "PROCESS_CPU_AGENT_BUILD_FILE",
        env_url: "PROCESS_CPU_AGENT_BUILD_URL",
    }];

    if target.is_linux() {
        list.push(Artifact {
            kind: ArtifactKind::NodeExporter,
            output_name: "node_exporter.tar.gz",
            env_file: "NODE_EXPORTER_BUILD_FILE",
            env_url: "NODE_EXPORTER_BUILD_URL",
        });
    }

    if target.is_windows() {
        list.push(Artifact {
            kind: ArtifactKind::WindowsExporter,
            output_name: "windows_exporter.msi",
            env_file: "WINDOWS_EXPORTER_BUILD_FILE",
            env_url: "WINDOWS_EXPORTER_BUILD_URL",
        });
    }

    list
}

fn copy_to(src: &Path, dest: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dest)?;
    println!(
        "cargo:warning=Copied {} to {}",
        src.display(),
        dest.display()
    );
    Ok(())
}

fn download_to(url: &str, dest: &Path) -> Result<(), Box<dyn Error>> {
    println!("cargo:warning=Downloading artifact from {url}");
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let response = client.get(url).send()?;
    if !response.status().is_success() {
        return Err(format!("Failed to download {url}: HTTP {}", response.status()).into());
    }
    let bytes = response.bytes()?;
    fs::write(dest, &bytes)?;
    Ok(())
}

fn default_process_cpu_agent_url(target: &TargetInfo) -> Option<String> {
    let os = match target.os {
        TargetOs::Windows => "windows",
        TargetOs::Linux => "linux",
        TargetOs::Other => return None,
    };

    let arch = match target.arch {
        TargetArch::X86_64 => "amd64",
        TargetArch::X86 => "386",
        TargetArch::Aarch64 => "arm64",
        TargetArch::Other => "amd64",
    };

    let suffix = if os == "windows" {
        format!("process-cpu-agent-{os}-{arch}.exe")
    } else {
        format!("process-cpu-agent-{os}-{arch}")
    };

    Some(format!(
        "https://github.com/your-org/process-cpu-agent/releases/latest/download/{suffix}"
    ))
}

fn default_node_exporter_url(target: &TargetInfo) -> Option<String> {
    if !target.is_linux() {
        return None;
    }

    let arch = match target.arch {
        TargetArch::X86_64 => "amd64",
        TargetArch::X86 => "386",
        TargetArch::Aarch64 => "arm64",
        TargetArch::Other => "amd64",
    };

    Some(format!(
        "https://github.com/prometheus/node_exporter/releases/download/v{ver}/node_exporter-{ver}.linux-{arch}.tar.gz",
        ver = NODE_EXPORTER_VERSION
    ))
}

fn default_windows_exporter_url(target: &TargetInfo) -> Option<String> {
    if !target.is_windows() {
        return None;
    }

    let arch = match target.arch {
        TargetArch::X86_64 => "amd64",
        _ => "386",
    };

    Some(format!(
        "https://github.com/prometheus-community/windows_exporter/releases/download/v{ver}/windows_exporter-{ver}-{arch}.msi",
        ver = WINDOWS_EXPORTER_VERSION
    ))
}
