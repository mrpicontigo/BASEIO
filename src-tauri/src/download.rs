//! Verified, resumable, cancellable file downloads (used for the image engine
//! and image models). A file only appears at its final path after its size and
//! sha256 both match the published values.

use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::watch;

pub fn sha256_file(path: &Path) -> AppResult<String> {
    let mut f = std::fs::File::open(path).map_err(|e| AppError::Internal(format!("open {}: {e}", path.display())))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).map_err(|e| AppError::Internal(e.to_string()))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}

/// True if `path` exists with the expected size (cheap check, no hashing).
pub fn present(path: &Path, size: u64) -> bool {
    std::fs::metadata(path).map(|m| m.is_file() && m.len() == size).unwrap_or(false)
}

fn part_path(dest: &Path) -> PathBuf {
    let mut p = dest.as_os_str().to_owned();
    p.push(".part");
    PathBuf::from(p)
}

/// Download `url` to `dest`, resuming a previous `.part` file when possible.
/// `on_progress(downloaded, total)` is called as bytes arrive.
pub async fn fetch_verified(
    http: &reqwest::Client,
    url: &str,
    dest: &Path,
    size: u64,
    sha256: &str,
    cancel: &mut watch::Receiver<bool>,
    mut on_progress: impl FnMut(u64, u64),
) -> AppResult<()> {
    if present(dest, size) {
        on_progress(size, size);
        return Ok(());
    }
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| AppError::Internal(format!("create {}: {e}", dir.display())))?;
    }
    let part = part_path(dest);
    let mut have = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    if have > size {
        let _ = std::fs::remove_file(&part);
        have = 0;
    }

    if have < size {
        let mut req = http.get(url).timeout(Duration::from_secs(60 * 60 * 6));
        if have > 0 {
            req = req.header("Range", format!("bytes={have}-"));
        }
        let resp = req.send().await.map_err(|e| AppError::Network(format!("Download failed to start: {e}")))?;
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(AppError::Network("Access denied by the server. The model may require accepting its licence on the publisher's site.".into()));
        }
        if !status.is_success() {
            return Err(AppError::Network(format!("Download failed: HTTP {status}")));
        }
        // Server ignored the Range header: start over.
        if have > 0 && status != reqwest::StatusCode::PARTIAL_CONTENT {
            have = 0;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(have > 0)
            .truncate(have == 0)
            .open(&part)
            .map_err(|e| AppError::Internal(format!("open {}: {e}", part.display())))?;
        let mut stream = resp.bytes_stream();
        on_progress(have, size);
        loop {
            let chunk = tokio::select! {
                _ = cancel.changed() => return Err(AppError::Cancelled),
                c = tokio::time::timeout(Duration::from_secs(120), stream.next()) => match c {
                    Err(_) => return Err(AppError::Network("Download stalled: no data for 2 minutes".into())),
                    Ok(None) => break,
                    Ok(Some(Err(e))) => return Err(AppError::Network(format!("Download interrupted: {e}"))),
                    Ok(Some(Ok(b))) => b,
                },
            };
            file.write_all(&chunk).map_err(|e| {
                if e.raw_os_error() == Some(112) { AppError::Validation("Not enough disk space".into()) } else { AppError::Internal(e.to_string()) }
            })?;
            have += chunk.len() as u64;
            if have > size {
                drop(file);
                let _ = std::fs::remove_file(&part);
                return Err(AppError::Network("Downloaded file is larger than expected; discarded".into()));
            }
            on_progress(have, size);
        }
        file.flush().map_err(|e| AppError::Internal(e.to_string()))?;
    }

    if have != size {
        return Err(AppError::Network(format!("Download incomplete ({have} of {size} bytes). Retry to resume.")));
    }
    let p = part.clone();
    let digest = tauri::async_runtime::spawn_blocking(move || sha256_file(&p))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
    if !digest.eq_ignore_ascii_case(sha256) {
        let _ = std::fs::remove_file(&part);
        return Err(AppError::Network("Downloaded file failed its checksum and was deleted. Please retry.".into()));
    }
    std::fs::rename(&part, dest).map_err(|e| AppError::Internal(format!("finalise download: {e}")))?;
    Ok(())
}

/// Extract a zip into `dest` safely (rejects paths escaping `dest`).
pub fn extract_zip(zip_path: &Path, dest: &Path) -> AppResult<()> {
    let f = std::fs::File::open(zip_path).map_err(|e| AppError::Internal(e.to_string()))?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| AppError::Internal(format!("bad archive: {e}")))?;
    std::fs::create_dir_all(dest).map_err(|e| AppError::Internal(e.to_string()))?;
    for i in 0..z.len() {
        let mut entry = z.by_index(i).map_err(|e| AppError::Internal(e.to_string()))?;
        let rel = entry
            .enclosed_name()
            .ok_or_else(|| AppError::Validation(format!("unsafe path in archive: {}", entry.name())))?;
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| AppError::Internal(e.to_string()))?;
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(|e| AppError::Internal(e.to_string()))?;
        }
        let mut w = std::fs::File::create(&out).map_err(|e| AppError::Internal(e.to_string()))?;
        std::io::copy(&mut entry, &mut w).map_err(|e| AppError::Internal(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_value() {
        let p = std::env::temp_dir().join(format!("baseio-sha-{}", std::process::id()));
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(sha256_file(&p).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert!(present(&p, 3));
        assert!(!present(&p, 4));
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn zip_slip_rejected() {
        let dir = std::env::temp_dir().join(format!("baseio-zip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let zp = dir.join("evil.zip");
        {
            let f = std::fs::File::create(&zp).unwrap();
            let mut z = zip::ZipWriter::new(f);
            z.start_file("../escape.txt", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"x").unwrap();
            z.finish().unwrap();
        }
        assert!(extract_zip(&zp, &dir.join("out")).is_err());
        assert!(!dir.join("escape.txt").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
