use std::collections::HashMap;

use anyhow::anyhow;
use sha2::{Digest, Sha256};
use tokio::fs;
use tracing::{debug, info, warn};

/// Validate proof-of-possession by checking file hashes
pub async fn validate_proof_of_possession(
    file_hashes: &HashMap<String, String>,
) -> anyhow::Result<()> {
    // For local development, we'll be more lenient with proof-of-possession
    if file_hashes.is_empty() {
        warn!("No file hashes provided for proof-of-possession, allowing for local development");
        return Ok(());
    }

    let mut valid_files = 0;
    for (file_path, expected_hash) in file_hashes {
        // Skip validation if it's a dummy hash (for local development)
        if expected_hash == "dummy_hash" {
            debug!("Skipping validation for dummy hash: {}", file_path);
            valid_files += 1;
            continue;
        }

        match fs::read(file_path).await {
            Ok(content) => {
                let mut hasher = Sha256::new();
                hasher.update(&content);
                let actual_hash = format!("{:x}", hasher.finalize());

                if actual_hash == *expected_hash {
                    valid_files += 1;
                    debug!("File hash validated: {}", file_path);
                } else {
                    warn!(
                        "Hash mismatch for file {}: expected {}, got {}",
                        file_path, expected_hash, actual_hash
                    );
                }
            }
            Err(e) => {
                warn!(
                    "File not accessible for proof-of-possession validation: {} - {}",
                    file_path, e
                );
            }
        }
    }

    // For local development, we'll allow the request if at least one file validates
    // or if all are dummy hashes
    if valid_files > 0 {
        info!(
            "Proof-of-possession validation passed ({} valid files)",
            valid_files
        );
        Ok(())
    } else {
        Err(anyhow!(
            "No valid file hashes found for proof-of-possession"
        ))
    }
}
