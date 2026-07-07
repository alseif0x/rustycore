use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub account: String,
    pub password: String,
    pub character_guid: u64,
    pub account_id: u32,
    pub lfg_role: u8,
    pub class: String,
    pub enabled: bool,
    pub session_key_bnet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestsConfig {
    pub lfg_join: bool,
    pub lfg_proposal: bool,
    pub bg_join: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub dungeon_id: u32,
    pub wait_for_proposal_timeout_secs: u64,
    pub launch_delay_ms: u64,
    #[serde(default)]
    pub auto_teleport: bool,
    #[serde(default)]
    pub cleanup_groups: bool,
    #[serde(default)]
    pub require_group: bool,
    pub tests: TestsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub bots: Vec<BotConfig>,
    pub test_config: TestConfig,
}

impl AppConfig {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;
        let config: AppConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path))?;
        Ok(())
    }

    pub fn get_enabled_bots(&self) -> Vec<&BotConfig> {
        self.bots.iter().filter(|b| b.enabled).collect()
    }

    pub fn default_config() -> Self {
        AppConfig {
            bots: vec![BotConfig {
                account: "TESTBOT@bot.local".to_string(),
                password: String::new(),
                character_guid: 13,
                account_id: 6,
                lfg_role: 8,
                class: "warrior".to_string(),
                enabled: true,
                session_key_bnet: String::new(),
            }],
            test_config: TestConfig {
                dungeon_id: 259,
                wait_for_proposal_timeout_secs: 120,
                launch_delay_ms: 2000,
                auto_teleport: false,
                cleanup_groups: false,
                require_group: false,
                tests: TestsConfig {
                    lfg_join: true,
                    lfg_proposal: true,
                    bg_join: true,
                },
            },
        }
    }

    pub fn load_or_create(path: &str) -> Result<Self> {
        if Path::new(path).exists() {
            Self::from_file(path)
        } else {
            let config = Self::default_config();
            config.save_to_file(path)?;
            Ok(config)
        }
    }
}
