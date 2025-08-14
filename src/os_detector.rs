use std::env;

#[derive(Debug, Clone, PartialEq)]
pub enum OsType {
    Linux,
    Windows,
    MacOs,
    Unknown,
}

pub fn detect_os() -> OsType {
    match env::consts::OS {
        "linux" => OsType::Linux,
        "windows" => OsType::Windows,
        "macos" => OsType::MacOs,
        _ => OsType::Unknown,
    }
}

pub fn get_arch() -> &'static str {
    env::consts::ARCH
}

pub fn is_64bit() -> bool {
    env::consts::ARCH == "x86_64" || env::consts::ARCH == "aarch64"
}
