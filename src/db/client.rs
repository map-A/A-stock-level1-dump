use crate::config::Config;
use crate::models::{StockInfo, MarketData};
use anyhow::Result;
use clickhouse::Row;
use serde::{Deserialize, Serialize};
use tracing::info;

/// ClickHouse客户端封装
pub struct ClickHouseClient {
    client: clickhouse::Client,
    batch_size: usize,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
struct StockInfoRow {
    code: String,
    name: String,
    industry_l1_code: String,
    industry_l1_name: String,
    industry_l2_code: String,
    industry_l2_name: String,
    industry_l3_code: String,
    industry_l3_name: String,
    industry_l4_code: String,
    industry_l4_name: String,
}

impl From<StockInfo> for StockInfoRow {
    fn from(s: StockInfo) -> Self {
        Self {
            code: s.code,
            name: s.name,
            industry_l1_code: s.industry_l1_code,
            industry_l1_name: s.industry_l1_name,
            industry_l2_code: s.industry_l2_code,
            industry_l2_name: s.industry_l2_name,
            industry_l3_code: s.industry_l3_code,
            industry_l3_name: s.industry_l3_name,
            industry_l4_code: s.industry_l4_code,
            industry_l4_name: s.industry_l4_name,
        }
    }
}

/// ClickHouse 原生行类型：trade_date 使用 time::Date 直接映射 Date 列
#[derive(Row, Serialize)]
struct MarketDataRow {
    code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    trade_date: time::Date,
    time_sec: u32,
    avg_sell_price: Option<f64>,
    cum_volume:     Option<u64>,
    cum_amount:     Option<f64>,
    cum_trades:     Option<u64>,
    high_price:     Option<f64>,
    low_price:      Option<f64>,
    sell5_price: Option<f64>, sell5_volume: Option<u64>,
    sell4_price: Option<f64>, sell4_volume: Option<u64>,
    sell3_price: Option<f64>, sell3_volume: Option<u64>,
    sell2_price: Option<f64>, sell2_volume: Option<u64>,
    sell1_price: Option<f64>, sell1_volume: Option<u64>,
    buy1_price:  Option<f64>, buy1_volume:  Option<u64>,
    buy2_price:  Option<f64>, buy2_volume:  Option<u64>,
    buy3_price:  Option<f64>, buy3_volume:  Option<u64>,
    buy4_price:  Option<f64>, buy4_volume:  Option<u64>,
    buy5_price:  Option<f64>, buy5_volume:  Option<u64>,
}

/// YYYYMMDD u32 → time::Date
fn u32_to_date(d: u32) -> Result<time::Date> {
    let year  = (d / 10000) as i32;
    let month = time::Month::try_from(((d / 100) % 100) as u8)
        .map_err(|_| anyhow::anyhow!("invalid month in date {}", d))?;
    let day   = (d % 100) as u8;
    time::Date::from_calendar_date(year, month, day)
        .map_err(|e| anyhow::anyhow!("invalid date {}: {}", d, e))
}

impl ClickHouseClient {
    pub fn new(config: &Config) -> Self {
        let client = clickhouse::Client::default()
            .with_url(&config.clickhouse.url)
            .with_database(&config.clickhouse.database)
            .with_user(&config.clickhouse.username)
            .with_password(&config.clickhouse.password);
        
        Self {
            client,
            batch_size: config.clickhouse.batch_size,
        }
    }
    
    /// 批量插入股票信息
    pub async fn insert_stock_info(&self, stocks: &[StockInfo]) -> Result<()> {
        let mut insert = self.client.insert("stock_info")?;
        
        for stock in stocks {
            let row: StockInfoRow = stock.clone().into();
            insert.write(&row).await?;
        }
        
        insert.end().await?;
        Ok(())
    }
    
    /// 批量插入市场数据（ClickHouse 原生二进制协议，无 SQL 字符串拼接）
    pub async fn insert_market_data(&self, data: &[MarketData]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        let mut insert = self.client.insert("market_data")?;
        for item in data {
            let row = MarketDataRow {
                code:           item.code.clone(),
                trade_date:     u32_to_date(item.trade_date)?,
                time_sec:       item.time_sec,
                avg_sell_price: item.avg_sell_price,
                cum_volume:     item.cum_volume,
                cum_amount:     item.cum_amount,
                cum_trades:     item.cum_trades,
                high_price:     item.high_price,
                low_price:      item.low_price,
                sell5_price: item.sell5_price, sell5_volume: item.sell5_volume,
                sell4_price: item.sell4_price, sell4_volume: item.sell4_volume,
                sell3_price: item.sell3_price, sell3_volume: item.sell3_volume,
                sell2_price: item.sell2_price, sell2_volume: item.sell2_volume,
                sell1_price: item.sell1_price, sell1_volume: item.sell1_volume,
                buy1_price:  item.buy1_price,  buy1_volume:  item.buy1_volume,
                buy2_price:  item.buy2_price,  buy2_volume:  item.buy2_volume,
                buy3_price:  item.buy3_price,  buy3_volume:  item.buy3_volume,
                buy4_price:  item.buy4_price,  buy4_volume:  item.buy4_volume,
                buy5_price:  item.buy5_price,  buy5_volume:  item.buy5_volume,
            };
            insert.write(&row).await?;
        }
        insert.end().await?;
        Ok(())
    }
    
    /// 检查数据是否已存在
    pub async fn data_exists(&self, code: &str, date: u32) -> Result<bool> {
        let year = date / 10000;
        let month = (date / 100) % 100;
        let day = date % 100;
        let date_str = format!("{:04}-{:02}-{:02}", year, month, day);
        
        let sql = format!(
            "SELECT COUNT(*) FROM market_data WHERE code = '{}' AND trade_date = '{}'",
            code.replace("'", "''"),
            date_str
        );
        
        let count: u64 = self.client
            .query(&sql)
            .fetch_one::<u64>()
            .await
            .unwrap_or(0);
        
        Ok(count > 0)
    }
}

/// 导入股票信息到数据库
pub async fn import_stock_info(config: &Config, stocks: &[StockInfo]) -> Result<()> {
    let client = ClickHouseClient::new(config);
    
    info!("开始导入股票信息，共 {} 条", stocks.len());
    
    // 分批插入
    for chunk in stocks.chunks(config.clickhouse.batch_size) {
        client.insert_stock_info(chunk).await?;
    }
    
    info!("✓ 股票信息导入完成");
    Ok(())
}
