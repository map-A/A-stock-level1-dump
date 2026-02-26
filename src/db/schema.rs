use crate::config::Config;
use anyhow::Result;
use tracing::info;

/// 初始化数据库（创建表）
pub async fn init_database(config: &Config) -> Result<()> {
    let client = clickhouse::Client::default()
        .with_url(&config.clickhouse.url)
        .with_database(&config.clickhouse.database)
        .with_user(&config.clickhouse.username)
        .with_password(&config.clickhouse.password);
    
    info!("创建数据库: {}", config.clickhouse.database);
    client
        .query(&format!("CREATE DATABASE IF NOT EXISTS {}", config.clickhouse.database))
        .execute()
        .await?;
    
    // 创建股票信息表
    info!("创建表: stock_info");
    let create_stock_info = r#"
        CREATE TABLE IF NOT EXISTS stock_info (
            code String,
            name String,
            industry_l1_code String,
            industry_l1_name String,
            industry_l2_code String,
            industry_l2_name String,
            industry_l3_code String,
            industry_l3_name String,
            industry_l4_code String,
            industry_l4_name String
        ) ENGINE = ReplacingMergeTree()
        ORDER BY code
    "#;
    
    client.query(create_stock_info).execute().await?;
    
    // 创建市场数据表
    info!("创建表: market_data");
    let create_market_data = r#"
        CREATE TABLE IF NOT EXISTS market_data (
            code String,
            trade_date Date,
            time_sec UInt32,
            avg_sell_price Nullable(Float64),
            cum_volume Nullable(UInt64),
            cum_amount Nullable(Float64),
            cum_trades Nullable(UInt64),
            high_price Nullable(Float64),
            low_price Nullable(Float64),
            sell5_price Nullable(Float64),
            sell5_volume Nullable(UInt64),
            sell4_price Nullable(Float64),
            sell4_volume Nullable(UInt64),
            sell3_price Nullable(Float64),
            sell3_volume Nullable(UInt64),
            sell2_price Nullable(Float64),
            sell2_volume Nullable(UInt64),
            sell1_price Nullable(Float64),
            sell1_volume Nullable(UInt64),
            buy1_price Nullable(Float64),
            buy1_volume Nullable(UInt64),
            buy2_price Nullable(Float64),
            buy2_volume Nullable(UInt64),
            buy3_price Nullable(Float64),
            buy3_volume Nullable(UInt64),
            buy4_price Nullable(Float64),
            buy4_volume Nullable(UInt64),
            buy5_price Nullable(Float64),
            buy5_volume Nullable(UInt64)
        ) ENGINE = MergeTree()
        PARTITION BY toYYYYMM(trade_date)
        ORDER BY (code, trade_date, time_sec)
    "#;
    
    client.query(create_market_data).execute().await?;
    
    Ok(())
}
