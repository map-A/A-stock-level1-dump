use anyhow::Result;
use crate::config::Config;
use clickhouse::Client;

/// 从数据库获取所有股票代码（只包含上证和深圳）
pub async fn get_all_stock_codes(config: &Config) -> Result<Vec<String>> {
    let client = Client::default()
        .with_url(&config.clickhouse.url)
        .with_user(&config.clickhouse.username)
        .with_password(&config.clickhouse.password)
        .with_database(&config.clickhouse.database);
    
    // 只查询上证（6xxxxx）和深圳（0xxxxx, 3xxxxx）股票
    let mut cursor = client
        .query("SELECT code FROM stock_info WHERE (code LIKE '6%' OR code LIKE '0%' OR code LIKE '3%') AND length(code) = 6 ORDER BY code")
        .fetch::<String>()?;
    
    let mut codes = Vec::with_capacity(5000);
    while let Some(code) = cursor.next().await? {
        codes.push(code);
    }
    
    Ok(codes)
}

/// 批量插入优化（使用原生协议）
pub async fn batch_insert_optimized(
    client: &crate::db::ClickHouseClient,
    data: Vec<crate::models::MarketData>,
    batch_size: usize,
) -> Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    
    // 分批插入
    for chunk in data.chunks(batch_size) {
        client.insert_market_data(chunk).await?;
    }
    
    Ok(())
}
