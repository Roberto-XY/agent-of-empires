//! Gemini CLI integration - session detection and resume

use anyhow::Result;
use std::fs;

pub fn detect_session_id(project_path: &str) -> Result<Option<String>> {
    let gemini_dir = get_gemini_chats_dir(project_path)?;

    if !gemini_dir.exists() {
        return Ok(None);
    }

    // Find the most recently modified session file
    let mut latest: Option<(String, std::time::SystemTime)> = None;

    if let Ok(entries) = fs::read_dir(&gemini_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let session_id = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string());

                        if let Some(id) = session_id {
                            if latest.is_none() || modified > latest.as_ref().unwrap().1 {
                                latest = Some((id, modified));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(latest.map(|(id, _)| id))
}

fn get_gemini_chats_dir(project_path: &str) -> Result<std::path::PathBuf> {
    // Check custom config dir first
    if let Some(custom_dir) = super::get_gemini_config_dir() {
        let project_hash = hash_project_path(project_path);
        return Ok(custom_dir.join("tmp").join(project_hash).join("chats"));
    }

    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    let project_hash = hash_project_path(project_path);

    // Default Gemini config location
    Ok(home.join(".gemini/tmp").join(project_hash).join("chats"))
}

fn hash_project_path(path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
