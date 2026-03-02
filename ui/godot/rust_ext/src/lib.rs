use godot::prelude::*;
use openplay_client::identity::{
    default_user_dir, ensure_user_dir, identity_filename, list_identities, KeyPair,
};

struct OpenPlaySdk;

#[gdextension]
unsafe impl ExtensionLibrary for OpenPlaySdk {}

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct OpenPlaySdkRef {
    #[base]
    base: Base<RefCounted>,
}

#[godot_api]
impl OpenPlaySdkRef {
    #[func]
    fn sdk_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[func]
    fn list_identities(&self) -> Array<Dictionary> {
        let mut result = Array::new();
        let Ok(dir) = default_user_dir() else {
            return result;
        };
        if ensure_user_dir(&dir).is_err() {
            return result;
        }
        let Ok(entries) = list_identities(&dir) else {
            return result;
        };

        for (path, kp) in entries {
            let mut dict = Dictionary::new();
            let _ = dict.insert("nickname", kp.card.nickname.clone());
            let _ = dict.insert("user_id", kp.user_id().to_string());
            let _ = dict.insert("path", path.to_string_lossy().to_string());
            result.push(&dict);
        }

        result
    }

    #[func]
    fn create_identity(&self, nickname: String) -> Dictionary {
        let mut dict = Dictionary::new();
        let Ok(dir) = default_user_dir() else {
            return dict;
        };
        if ensure_user_dir(&dir).is_err() {
            return dict;
        }
        let kp = KeyPair::generate(&nickname);
        let filename = identity_filename(&kp.user_id());
        let path = dir.join(&filename);
        if kp.save(&path).is_ok() {
            let _ = dict.insert("nickname", kp.card.nickname.clone());
            let _ = dict.insert("user_id", kp.user_id().to_string());
            let _ = dict.insert("path", path.to_string_lossy().to_string());
        }
        dict
    }

    #[func]
    fn delete_identity(&self, path: String) -> bool {
        std::fs::remove_file(std::path::Path::new(&path)).is_ok()
    }
}

#[godot_api]
impl IRefCounted for OpenPlaySdkRef {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
    }
}
