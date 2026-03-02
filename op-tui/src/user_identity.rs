use std::path::{Path, PathBuf};

use openplay_client::identity::{
    default_user_dir, ensure_user_dir, identity_filename, list_identities, KeyPair,
};

#[derive(Debug, Clone)]
pub struct IdentityProfile {
    pub nickname: String,
    pub user_id: String,
    pub path: PathBuf,
}

fn to_profile(path: PathBuf, key_pair: KeyPair) -> IdentityProfile {
    IdentityProfile {
        nickname: key_pair.card.nickname.clone(),
        user_id: key_pair.user_id().to_string(),
        path,
    }
}

pub fn load_identities() -> Result<Vec<IdentityProfile>, String> {
    let dir = default_user_dir().map_err(|e| e.to_string())?;
    ensure_user_dir(&dir).map_err(|e| e.to_string())?;
    let mut entries = list_identities(&dir).map_err(|e| e.to_string())?;
    let mut profiles: Vec<IdentityProfile> = entries
        .drain(..)
        .map(|(path, kp)| to_profile(path, kp))
        .collect();
    profiles.sort_by(|a, b| a.nickname.cmp(&b.nickname).then(a.user_id.cmp(&b.user_id)));
    Ok(profiles)
}

pub fn load_identity_from_path(path: &Path) -> Result<IdentityProfile, String> {
    let key_pair = KeyPair::load(path).map_err(|e| e.to_string())?;
    Ok(to_profile(path.to_path_buf(), key_pair))
}

pub fn create_identity(nickname: &str) -> Result<IdentityProfile, String> {
    let dir = default_user_dir().map_err(|e| e.to_string())?;
    ensure_user_dir(&dir).map_err(|e| e.to_string())?;
    let key_pair = KeyPair::generate(nickname);
    let filename = identity_filename(&key_pair.user_id());
    let path = dir.join(&filename);
    key_pair.save(&path).map_err(|e| e.to_string())?;
    Ok(to_profile(path, key_pair))
}

pub fn delete_identity(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| e.to_string())
}
