mod node_exporter;
mod os_detector;
mod process_cpu_agent;
mod windows_exporter;

use node_exporter::NodeExporterSetup;
use os_detector::{OsType, detect_os};
use process_cpu_agent::ProcessCpuAgentSetup;
use windows_exporter::WindowsExporterSetup;

fn main() {
    println!("Prometheus Exporters Setup Tool");
    println!("================================\n");

    let os_type = detect_os();
    let arch = os_detector::get_arch();

    println!("Detected OS: {:?}", os_type);
    println!("Architecture: {}", arch);
    println!("64-bit: {}\n", os_detector::is_64bit());

    let process_agent_setup = ProcessCpuAgentSetup::new();

    let result = match os_type {
        OsType::Linux => {
            println!("Setting up exporters for Linux...\n");

            println!("1. Setting up Node Exporter...");
            let node_setup = NodeExporterSetup::new();
            if let Err(e) = node_setup.setup() {
                eprintln!("Node Exporter setup failed: {}", e);
            }

            println!("\n2. Setting up Process CPU Agent...");
            match process_agent_setup.setup() {
                Ok(_) => process_agent_setup.create_config_file(),
                Err(e) => Err(e),
            }
        }
        OsType::Windows => {
            println!("Setting up exporters for Windows...\n");

            println!("1. Setting up Windows Exporter...");
            let windows_setup = WindowsExporterSetup::new();
            if let Err(e) = windows_setup.setup() {
                eprintln!("Windows Exporter setup failed: {}", e);
            }
            windows_setup.create_config_file().ok();

            println!("\n2. Setting up Process CPU Agent...");
            match process_agent_setup.setup() {
                Ok(_) => process_agent_setup.create_config_file(),
                Err(e) => Err(e),
            }
        }
        OsType::MacOs => {
            println!("Setting up exporters for macOS...\n");
            println!("Note: macOS support uses Node Exporter with limited collectors");

            println!("1. Setting up Node Exporter...");
            let node_setup = NodeExporterSetup::new();
            if let Err(e) = node_setup.setup() {
                eprintln!("Node Exporter setup failed: {}", e);
            }

            println!("\n2. Setting up Process CPU Agent...");
            match process_agent_setup.setup() {
                Ok(_) => process_agent_setup.create_config_file(),
                Err(e) => Err(e),
            }
        }
        OsType::Unknown => Err("Unsupported operating system".into()),
    };

    match result {
        Ok(_) => {
            println!("\n✓ Exporter setup completed successfully!");
            println!("Next steps:");
            match os_type {
                OsType::Linux => {
                    println!("1. Start Node Exporter: sudo systemctl enable --now node_exporter");
                    println!(
                        "2. Start Process CPU Agent: sudo systemctl enable --now process-cpu-agent"
                    );
                    println!("3. Check Node Exporter metrics: http://localhost:9100/metrics");
                    println!("4. Check Process CPU Agent metrics: http://localhost:9256/metrics");
                }
                OsType::Windows => {
                    println!("1. Check Windows Exporter: sc query windows_exporter");
                    println!("2. Start Process CPU Agent: sc start ProcessCpuAgent");
                    println!("3. Check Windows Exporter metrics: http://localhost:9182/metrics");
                    println!("4. Check Process CPU Agent metrics: http://localhost:9256/metrics");
                }
                OsType::MacOs => {
                    println!("1. Start Node Exporter manually from /opt/prometheus/node_exporter");
                    println!("2. Start Process CPU Agent from /opt/prometheus/process-cpu-agent");
                    println!("3. Check Node Exporter metrics: http://localhost:9100/metrics");
                    println!("4. Check Process CPU Agent metrics: http://localhost:9256/metrics");
                }
                _ => {}
            }
            println!("5. Configure Prometheus to scrape these exporters");
        }
        Err(e) => {
            eprintln!("\n✗ Setup failed: {}", e);
            eprintln!("Please check permissions and try again");
            std::process::exit(1);
        }
    }
}
