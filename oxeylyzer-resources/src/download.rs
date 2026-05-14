use std::io::Read;
use std::path::Path;

use crate::ResourceError;

const DATA_URL: &str =
    "https://github.com/o-x-e-y/oxeylyzer/archive/refs/heads/data-files.zip";

// GitHub archive zip puts all files under a root folder named "{repo}-{branch}/"
const ZIP_INNER_PREFIX: &str = "oxeylyzer-data-files/";

pub enum DownloadProgress {
    Connecting,
    Downloading { bytes_done: u64, bytes_total: Option<u64> },
    Extracting,
    Done,
}

pub fn download_and_extract<F>(dest: &Path, progress: F) -> Result<(), ResourceError>
where
    F: Fn(DownloadProgress) + Send + 'static,
{
    progress(DownloadProgress::Connecting);

    let resp = ureq::get(DATA_URL)
        .call()
        .map_err(|e| ResourceError::Download(e.to_string()))?;

    let content_length = resp
        .header("content-length")
        .and_then(|v| v.parse::<u64>().ok());

    let mut body = Vec::new();
    let mut bytes_done: u64 = 0;
    let mut reader = resp.into_reader();
    let mut buf = [0u8; 16384];

    loop {
        let n = reader.read(&mut buf).map_err(ResourceError::Io)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
        bytes_done += n as u64;
        progress(DownloadProgress::Downloading { bytes_done, bytes_total: content_length });
    }

    progress(DownloadProgress::Extracting);

    let cursor = std::io::Cursor::new(body);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| ResourceError::Extraction(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file =
            archive.by_index(i).map_err(|e| ResourceError::Extraction(e.to_string()))?;

        let raw_name = file.name().to_string();
        let stripped = match raw_name.strip_prefix(ZIP_INNER_PREFIX) {
            Some(s) => s,
            None => &raw_name,
        };
        if stripped.is_empty() {
            continue;
        }

        let out_path = dest.join(stripped);

        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out)?;
        }
    }

    std::fs::write(dest.join(".data-version"), "1.0.0")?;

    progress(DownloadProgress::Done);
    Ok(())
}
