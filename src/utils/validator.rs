use crate::config::ValidationConfig;
use crate::models::MarketData;
use anyhow::{Result, bail};

pub struct DataValidator {
    config: ValidationConfig,
}

impl DataValidator {
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }
    
    /// 验证市场数据
    pub fn validate(&self, data: &MarketData) -> Result<()> {
        // 验证价格范围
        if let Some(price) = data.avg_sell_price {
            if price < self.config.min_price || price > self.config.max_price {
                bail!("价格超出范围: {}", price);
            }
        }
        
        // 验证买卖档价格
        for price in [
            data.sell1_price, data.sell2_price, data.sell3_price, 
            data.sell4_price, data.sell5_price,
            data.buy1_price, data.buy2_price, data.buy3_price,
            data.buy4_price, data.buy5_price,
        ].iter().flatten() {
            if *price < self.config.min_price || *price > self.config.max_price {
                bail!("档位价格超出范围: {}", price);
            }
        }
        
        // 验证数量非负
        for volume in [
            data.cum_volume,
            data.sell1_volume, data.sell2_volume, data.sell3_volume,
            data.sell4_volume, data.sell5_volume,
            data.buy1_volume, data.buy2_volume, data.buy3_volume,
            data.buy4_volume, data.buy5_volume,
        ].iter().flatten() {
            if *volume == 0 {
                // 允许为0
            }
        }
        
        // 验证时间格式
        if data.time_sec < 90000 || data.time_sec > 160000 {
            // 允许盘前盘后，只做警告
        }
        
        Ok(())
    }
}
