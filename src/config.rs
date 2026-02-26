use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub fetcher: FetcherConfig,
    pub clickhouse: ClickHouseConfig,
    pub data: DataConfig,
    pub validation: ValidationConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub timeout_secs: u64,
    pub retry_count: u32,
    pub retry_delay_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FetcherConfig {
    pub max_concurrent: usize,
    pub batch_size: usize,
    pub request_delay_ms: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
}

fn default_max_retries() -> usize {
    3
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClickHouseConfig {
    pub url: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataConfig {
    pub stock_list: String,
    pub trading_calendar: String,
    pub output_dir: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValidationConfig {
    pub min_price: f64,
    pub max_price: f64,
    pub max_price_change_pct: f64,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context(format!("无法读取配置文件: {}", path))?;
        
        let config: Config = toml::from_str(&content)
            .context("配置文件解析失败")?;
        
        Ok(config)
    }
}
