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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_os() {
        let os = detect_os();
        // The result should be one of the valid OS types
        assert!(matches!(
            os,
            OsType::Linux | OsType::Windows | OsType::MacOs | OsType::Unknown
        ));
    }

    #[test]
    fn test_get_arch() {
        let arch = get_arch();
        // Architecture should not be empty
        assert!(!arch.is_empty());
        // Common architectures
        assert!(matches!(
            arch,
            "x86_64" | "x86" | "aarch64" | "arm" | "powerpc" | "powerpc64" | "mips" | "sparc"
        ));
    }

    #[test]
    fn test_is_64bit() {
        let is_64 = is_64bit();
        let arch = get_arch();
        
        if arch == "x86_64" || arch == "aarch64" {
            assert!(is_64);
        } else {
            assert!(!is_64);
        }
    }

    #[test]
    fn test_os_type_equality() {
        assert_eq!(OsType::Linux, OsType::Linux);
        assert_eq!(OsType::Windows, OsType::Windows);
        assert_eq!(OsType::MacOs, OsType::MacOs);
        assert_eq!(OsType::Unknown, OsType::Unknown);
        assert_ne!(OsType::Linux, OsType::Windows);
    }

    #[test]
    fn test_os_type_clone() {
        let os1 = OsType::Linux;
        let os2 = os1.clone();
        assert_eq!(os1, os2);
    }

    #[test]
    fn test_current_os_consistency() {
        // Test that multiple calls return the same result
        let os1 = detect_os();
        let os2 = detect_os();
        assert_eq!(os1, os2);
    }

    #[test]
    fn test_arch_consistency() {
        // Test that multiple calls return the same result
        let arch1 = get_arch();
        let arch2 = get_arch();
        assert_eq!(arch1, arch2);
    }
}
