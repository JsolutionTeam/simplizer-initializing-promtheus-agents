use reqwest::blocking::Client;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Download a file from URL to the specified path
pub fn download_file(url: &str, dest_path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("Downloading from: {url}");

    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(dest_path).parent() {
        fs::create_dir_all(parent)?;
    }

    // Download the file
    let client = Client::new();
    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(format!("Failed to download: HTTP {}", response.status()).into());
    }

    let bytes = response.bytes()?.to_vec();

    // Write to file
    write_file(dest_path, &bytes)?;

    println!("Downloaded to: {dest_path}");

    // Set executable permissions on Unix
    #[cfg(unix)]
    set_executable_permissions(dest_path)?;

    Ok(bytes)
}

/// Write bytes to a file
pub fn write_file(path: &str, content: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(path)?;
    file.write_all(content)?;
    Ok(())
}

/// Set executable permissions on Unix systems
#[cfg(unix)]
pub fn set_executable_permissions(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

/// Download content from URL
pub fn download_content(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(format!("Failed to download: HTTP {}", response.status()).into());
    }

    Ok(response.bytes()?.to_vec())
}

/// Extract tar.gz archive to specified path
pub fn extract_tar_gz(
    archive_bytes: &[u8],
    extract_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    fs::create_dir_all(extract_path)?;

    let tar_gz = GzDecoder::new(archive_bytes);
    let mut archive = Archive::new(tar_gz);
    archive.unpack(extract_path)?;

    Ok(())
}

/// Extract zip archive to specified path
pub fn extract_zip(
    archive_bytes: &[u8],
    extract_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Cursor;
    use zip::ZipArchive;

    fs::create_dir_all(extract_path)?;

    // Create a cursor from bytes for zip archive
    let cursor = Cursor::new(archive_bytes);
    let mut archive = ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(extract_path).join(file.name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;

            // Set executable permissions for binaries on Unix
            #[cfg(unix)]
            {
                if file.name().contains("exporter") || file.name().contains("agent") {
                    set_executable_permissions(outpath.to_str().unwrap())?;
                }
            }
        }
    }

    Ok(())
}

/// Download and extract a tar.gz file
pub fn download_and_extract_tar_gz(
    url: &str,
    extract_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Downloading tar.gz from: {url}");

    let bytes = download_content(url)?;
    extract_tar_gz(&bytes, extract_path)?;

    println!("Extracted to: {extract_path}");
    Ok(())
}

/// Download and extract a zip file
pub fn download_and_extract_zip(
    url: &str,
    extract_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Downloading zip from: {url}");

    let bytes = download_content(url)?;
    extract_zip(&bytes, extract_path)?;

    println!("Extracted to: {extract_path}");
    Ok(())
}

/// Create directory if it doesn't exist
pub fn ensure_directory_exists(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(path)?;
    Ok(())
}

/// Check if path exists
pub fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// Get parent directory of a path
pub fn get_parent_directory(path: &str) -> Option<String> {
    Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = b"Hello, World!";

        let result = write_file(file_path.to_str().unwrap(), content);
        assert!(result.is_ok());

        let read_content = fs::read(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_ensure_directory_exists() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("level1").join("level2");

        let result = ensure_directory_exists(nested_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(nested_path.exists());
    }

    #[test]
    fn test_path_exists() {
        let temp_dir = TempDir::new().unwrap();
        let existing_path = temp_dir.path();
        let non_existing_path = temp_dir.path().join("nonexistent");

        assert!(path_exists(existing_path.to_str().unwrap()));
        assert!(!path_exists(non_existing_path.to_str().unwrap()));
    }

    #[test]
    fn test_get_parent_directory() {
        let path = "/home/user/file.txt";
        let parent = get_parent_directory(path);
        assert_eq!(parent, Some("/home/user".to_string()));

        let root_path = "/";
        let root_parent = get_parent_directory(root_path);
        assert_eq!(root_parent, None);
    }

    #[test]
    fn test_download_content_invalid_url() {
        let result = download_content("http://192.0.2.1:9999/nonexistent/file.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_download_file_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let dest_path = temp_dir.path().join("test.txt");

        let result = download_file(
            "http://192.0.2.1:9999/nonexistent/file.txt",
            dest_path.to_str().unwrap(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_extract_tar_gz_with_invalid_data() {
        let temp_dir = TempDir::new().unwrap();
        let extract_path = temp_dir.path().join("extract");
        let invalid_data = b"not a tar.gz file";

        let result = extract_tar_gz(invalid_data, extract_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_zip_with_invalid_data() {
        let temp_dir = TempDir::new().unwrap();
        let extract_path = temp_dir.path().join("extract");
        let invalid_data = b"not a zip file";

        let result = extract_zip(invalid_data, extract_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_download_and_extract_tar_gz_invalid_url() {
        let temp_dir = TempDir::new().unwrap();

        let result = download_and_extract_tar_gz(
            "http://192.0.2.1:9999/archive.tar.gz",
            temp_dir.path().to_str().unwrap(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_download_and_extract_zip_invalid_url() {
        let temp_dir = TempDir::new().unwrap();

        let result = download_and_extract_zip(
            "http://192.0.2.1:9999/archive.zip",
            temp_dir.path().to_str().unwrap(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_create_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested = temp_dir.path().join("a").join("b").join("c");

        ensure_directory_exists(nested.to_str().unwrap()).unwrap();
        assert!(nested.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_set_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("executable");

        // Create a file
        write_file(file_path.to_str().unwrap(), b"#!/bin/sh\necho test").unwrap();

        // Set executable permissions
        set_executable_permissions(file_path.to_str().unwrap()).unwrap();

        // Check permissions
        let metadata = fs::metadata(&file_path).unwrap();
        let permissions = metadata.permissions();
        assert_eq!(permissions.mode() & 0o111, 0o111); // Check execute bits
    }
}
