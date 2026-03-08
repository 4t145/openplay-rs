use std::path::PathBuf;

use openplay_client::{KeyPair, default_user_dir, identity::identity_filename};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user_dir = default_user_dir()?;
    std::fs::create_dir_all(&user_dir)?;

    let key_pair = KeyPair::generate("test-player");
    let filename = identity_filename(&key_pair.user_id());
    let path: PathBuf = user_dir.join(filename);

    key_pair.save(&path)?;

    println!("identity written: {}", path.display());
    println!("user_id: {}", key_pair.user_id());
    Ok(())
}
