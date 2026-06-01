use anyhow::Result;
use config::{Config, File};
use directories::ProjectDirs;
use rust_embed::RustEmbed;
use serde::Deserialize;

#[derive(RustEmbed)]
#[folder = "resources/themes/"]
struct ThemeAsset;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)] // allow dead code for sub alt, right now i don't have button yet so i save this for the future
pub struct Theme {
    pub bg: String,
    pub main: String,
    pub caret: String,
    pub text: String,
    pub sub: String,
    #[serde(alias = "subAlt")]
    pub sub_alt: String,
    pub error: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: "#2c2e34".to_string(),
            main: "#e2b714".to_string(),
            caret: "#e2b714".to_string(),
            text: "#d1d0c5".to_string(),
            sub: "#646669".to_string(),
            sub_alt: "#45474d".to_string(),
            error: "#ca4754".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_theme_name")]
    pub theme: String,
    pub custom_theme: Option<Theme>,
}

fn default_theme_name() -> String {
    "default".to_string()
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let mut builder = Config::builder()
            .set_default("theme", "default")?;

        if let Some(proj_dirs) = ProjectDirs::from("", "", "typa") {
            let config_path = proj_dirs.config_dir().join("config.toml");
            if config_path.exists() {
                builder = builder.add_source(File::from(config_path));
            }
        }

        let cfg = builder.build()?;
        let app_config: AppConfig = cfg.try_deserialize()?;
        Ok(app_config)
    }

    pub fn resolved_theme(&self) -> Theme {
        // prioritize custom theme
        if let Some(ref t) = self.custom_theme {
            return t.clone();
        }
        get_builtin_theme(&self.theme).unwrap_or_default()
    }
}

pub fn get_builtin_theme(name: &str) -> Option<Theme> {
    let filename = format!("{}.json", name);
    let file = ThemeAsset::get(&filename)?;
    let s = std::str::from_utf8(file.data.as_ref()).ok()?;
    serde_json::from_str(s).ok()
}
