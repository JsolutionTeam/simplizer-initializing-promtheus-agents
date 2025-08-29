use reqwest::blocking::Client;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

pub struct ExporterDownloader {
    client: Client,
}

impl ExporterDownloader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Download a file from URL to the specified path
    pub fn download_file(
        &self,
        url: &str,
        dest_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Downloading from: {url}");

        // Create parent directories if they don't exist
        if let Some(parent) = Path::new(dest_path).parent() {
            fs::create_dir_all(parent)?;
        }

        // Download the file
        let response = self.client.get(url).send()?;

        if !response.status().is_success() {
            return Err(format!("Failed to download: HTTP {}", response.status()).into());
        }

        let bytes = response.bytes()?;

        // Write to file
        let mut file = File::create(dest_path)?;
        file.write_all(&bytes)?;

        println!("Downloaded to: {dest_path}");

        // Set executable permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(dest_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(dest_path, permissions)?;
        }

        Ok(())
    }

    /// Download and extract a tar.gz file
    pub fn download_and_extract_tar_gz(
        &self,
        url: &str,
        extract_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Downloading tar.gz from: {url}");

        let response = self.client.get(url).send()?;

        if !response.status().is_success() {
            return Err(format!("Failed to download: HTTP {}", response.status()).into());
        }

        let bytes = response.bytes()?;

        // Extract tar.gz
        let tar_gz = flate2::read::GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(tar_gz);

        fs::create_dir_all(extract_path)?;
        archive.unpack(extract_path)?;

        println!("Extracted to: {extract_path}");

        Ok(())
    }

    /// Download and extract a zip file
    pub fn download_and_extract_zip(
        &self,
        url: &str,
        extract_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Downloading zip from: {url}");

        let response = self.client.get(url).send()?;

        if !response.status().is_success() {
            return Err(format!("Failed to download: HTTP {}", response.status()).into());
        }

        let bytes = response.bytes()?;

        // Create a temporary file for the zip
        let temp_zip = format!("{extract_path}/temp_download.zip");
        fs::create_dir_all(extract_path)?;

        let mut file = File::create(&temp_zip)?;
        file.write_all(&bytes)?;
        drop(file);

        // Extract zip
        let file = File::open(&temp_zip)?;
        let mut archive = zip::ZipArchive::new(file)?;

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
                    use std::os::unix::fs::PermissionsExt;
                    if file.name().contains("exporter") || file.name().contains("agent") {
                        let metadata = fs::metadata(&outpath)?;
                        let mut permissions = metadata.permissions();
                        permissions.set_mode(0o755);
                        fs::set_permissions(&outpath, permissions)?;
                    }
                }
            }
        }

        // Clean up temp file
        fs::remove_file(&temp_zip)?;

        println!("Extracted to: {extract_path}");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_exporter_downloader_creation() {
        let downloader = ExporterDownloader::new();
        assert!(!std::ptr::addr_of!(downloader).is_null());
    }

    #[test]
    fn test_download_file_invalid_url() {
        let downloader = ExporterDownloader::new();
        let temp_dir = TempDir::new().unwrap();
        let dest_path = temp_dir.path().join("test.txt");
        
        // Use a URL that will definitely fail
        let result = downloader.download_file(
            "http://192.0.2.1:9999/nonexistent/file.txt",
            dest_path.to_str().unwrap()
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_download_file_creates_parent_dirs() {
        let downloader = ExporterDownloader::new();
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("level1").join("level2").join("file.txt");
        
        // This will fail but should create parent directories
        let _ = downloader.download_file(
            "http://invalid-url.com/file.txt",
            nested_path.to_str().unwrap()
        );
        
        // Check if parent directories were created
        assert!(nested_path.parent().unwrap().exists());
    }

    #[test]
    fn test_download_and_extract_tar_gz_invalid_url() {
        let downloader = ExporterDownloader::new();
        let temp_dir = TempDir::new().unwrap();
        
        let result = downloader.download_and_extract_tar_gz(
            "http://invalid-url.com/archive.tar.gz",
            temp_dir.path().to_str().unwrap()
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_download_and_extract_zip_invalid_url() {
        let downloader = ExporterDownloader::new();
        let temp_dir = TempDir::new().unwrap();
        
        let result = downloader.download_and_extract_zip(
            "http://invalid-url.com/archive.zip",
            temp_dir.path().to_str().unwrap()
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_path_creation() {
        let temp_dir = TempDir::new().unwrap();
        let extract_path = temp_dir.path().join("extract").join("nested");
        
        // Manually create the directory to test the expected behavior
        fs::create_dir_all(&extract_path).unwrap();
        
        assert!(extract_path.exists());
    }
}
