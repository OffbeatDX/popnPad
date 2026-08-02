use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default = "default_playfield_fit")]
    pub playfield_fit: f32,
    #[serde(default = "default_button_size")]
    pub button_size: f32,
    #[serde(default = "default_panel_pos")]
    pub panel_pos: f32,
    #[serde(default)]
    pub single_row: bool,
    #[serde(default)]
    pub diagnostics: bool,
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    #[serde(default)]
    pub flip_vertical: bool,
    #[serde(default)]
    pub windowed: bool,
    #[serde(default = "default_mon_auto")]
    pub pad_monitor: i32,
    #[serde(default = "default_fit_sub_screen")]
    pub fit_sub_screen: bool,
}

fn default_mon_auto() -> i32 {
    -1
}

fn default_fit_sub_screen() -> bool {
    true
}

fn default_playfield_fit() -> f32 {
    1.0
}

fn default_button_size() -> f32 {
    1.0
}

fn default_panel_pos() -> f32 {
    0.5
}

fn default_api_port() -> u16 {
    crate::spiceapi::DEFAULT_PORT
}

impl Default for Config {
    fn default() -> Self {
        Config {
            playfield_fit: default_playfield_fit(),
            button_size: default_button_size(),
            panel_pos: default_panel_pos(),
            single_row: false,
            diagnostics: false,
            api_port: default_api_port(),
            flip_vertical: false,
            windowed: false,
            pad_monitor: -1,
            fit_sub_screen: default_fit_sub_screen(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        p.set_file_name("popnpad.json");
        p
    }

    pub fn load() -> Self {
        load_file(&Self::path()).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Err(e) = save_to_path(self, &Self::path()) {
            log::warn!("could not write settings: {e}");
        }
    }
}

fn load_file(path: &Path) -> Option<Config> {
    match std::fs::read_to_string(path) {
        Ok(s) => match serde_json::from_str::<Config>(&s) {
            Ok(c) => {
                log::info!("loaded settings from {}", path.display());
                Some(c)
            }
            Err(e) => {
                log::warn!("settings parse failed ({e}); using defaults");
                None
            }
        },
        Err(_) => None,
    }
}

fn save_to_path(cfg: &Config, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(cfg).map_err(|e| format!("serialize: {e}"))?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("rename {}: {e}", path.display())
    })?;
    log::info!("saved settings to {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_dir(label: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("popnpad-config-{label}-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn defaults_are_sane() {
        let c = Config::default();
        assert_eq!(c.api_port, crate::spiceapi::DEFAULT_PORT);
        assert_eq!(c.pad_monitor, -1);
        assert_eq!(c.button_size, 1.0);
        assert!(c.fit_sub_screen, "the Sub Screen is fitted out of the box");
    }

    #[test]
    fn a_pchord_era_file_still_loads_and_keeps_what_carries_over() {
        let dir = temp_dir("legacy");
        let path = dir.join("popnpad.json");
        let legacy = serde_json::json!({
            "relative_faders": true,
            "fader_decay": 0.7,
            "fader_curve": 1.0,
            "key_height": 0.66,
            "light_keys": true,
            "playfield_fit": 0.8,
            "api_port": 4242,
            "windowed": true,
            "nav_monitor": 1,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&legacy).unwrap()).unwrap();

        let cfg = load_file(&path).expect("load");
        assert_eq!(cfg.api_port, 4242, "shared settings survive");
        assert_eq!(cfg.playfield_fit, 0.8);
        assert!(cfg.windowed);
        assert_eq!(
            cfg.button_size, 1.0,
            "dropped fader fields fall to defaults"
        );
        assert!(
            cfg.fit_sub_screen,
            "a file written before the Sub Screen switch opts into it"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_empty_object_loads_as_defaults() {
        let dir = temp_dir("empty");
        let path = dir.join("popnpad.json");
        std::fs::write(&path, "{}").unwrap();
        assert_eq!(load_file(&path), Some(Config::default()));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_file_uses_defaults() {
        let dir = temp_dir("missing");
        let path = dir.join("popnpad.json");
        assert!(!path.exists());
        assert_eq!(load_file(&path), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn malformed_input_falls_back_to_defaults() {
        let dir = temp_dir("bad");
        let path = dir.join("popnpad.json");
        std::fs::write(&path, "{not-json").unwrap();
        assert!(load_file(&path).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn save_is_atomic_and_reports_failed_writes() {
        let dir = temp_dir("atomic");
        let path = dir.join("popnpad.json");
        let cfg = Config::default();
        save_to_path(&cfg, &path).unwrap();
        assert!(path.exists());
        assert!(!path.with_extension("json.tmp").exists());

        let blocked = dir.join("blocked");
        std::fs::write(&blocked, "file").unwrap();
        let err = save_to_path(&cfg, &blocked.join("popnpad.json")).expect_err("mkdir fails");
        assert!(err.contains("mkdir"), "{err}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
