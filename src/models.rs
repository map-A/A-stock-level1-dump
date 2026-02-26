use serde::{Deserialize, Serialize};

/// 股票信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInfo {
    pub code: String,
    pub name: String,
    pub industry_l1_code: String,
    pub industry_l1_name: String,
    pub industry_l2_code: String,
    pub industry_l2_name: String,
    pub industry_l3_code: String,
    pub industry_l3_name: String,
    pub industry_l4_code: String,
    pub industry_l4_name: String,
}

/// 市场数据（单条tick）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub code: String,
    pub trade_date: u32,          // YYYYMMDD
    pub time_sec: u32,             // 时间秒数，如91500
    pub avg_sell_price: Option<f64>,
    pub cum_volume: Option<u64>,
    pub cum_amount: Option<f64>,
    pub cum_trades: Option<u64>,
    pub high_price: Option<f64>,
    pub low_price: Option<f64>,
    pub sell5_price: Option<f64>,
    pub sell5_volume: Option<u64>,
    pub sell4_price: Option<f64>,
    pub sell4_volume: Option<u64>,
    pub sell3_price: Option<f64>,
    pub sell3_volume: Option<u64>,
    pub sell2_price: Option<f64>,
    pub sell2_volume: Option<u64>,
    pub sell1_price: Option<f64>,
    pub sell1_volume: Option<u64>,
    pub buy1_price: Option<f64>,
    pub buy1_volume: Option<u64>,
    pub buy2_price: Option<f64>,
    pub buy2_volume: Option<u64>,
    pub buy3_price: Option<f64>,
    pub buy3_volume: Option<u64>,
    pub buy4_price: Option<f64>,
    pub buy4_volume: Option<u64>,
    pub buy5_price: Option<f64>,
    pub buy5_volume: Option<u64>,
}

/// 抓取任务
#[derive(Debug, Clone)]
pub struct FetchTask {
    pub stock_code: String,
    pub trade_date: u32,
}
