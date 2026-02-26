use crate::models::MarketData;

/// 数据验证器
pub struct DataValidator {
    min_price: f64,
    max_price: f64,
}

impl DataValidator {
    pub fn new(min_price: f64, max_price: f64) -> Self {
        Self {
            min_price,
            max_price,
        }
    }
    
    /// 验证时间是否在交易时段
    pub fn validate_time(&self, time_sec: u32) -> bool {
        // 转换为时分秒
        let hour = time_sec / 10000;
        let minute = (time_sec / 100) % 100;
        let _second = time_sec % 100;
        
        // 早盘：09:15 - 11:30
        if hour == 9 && minute >= 15 {
            return true;
        }
        if hour == 10 {
            return true;
        }
        if hour == 11 && minute <= 30 {
            return true;
        }
        
        // 午盘：13:00 - 15:00
        if hour == 13 {
            return true;
        }
        if hour == 14 {
            return true;
        }
        if hour == 15 && minute == 0 {
            return true;
        }
        
        false
    }
    
    /// 验证价格
    pub fn validate_price(&self, price: Option<f64>) -> bool {
        match price {
            Some(p) => p.is_finite() && p >= self.min_price && p <= self.max_price,
            None => true,  // 允许None
        }
    }
    
    /// 验证成交量
    pub fn validate_volume(&self, volume: Option<u64>) -> bool {
        match volume {
            Some(_v) => true,
            None => true,
        }
    }
    
    /// 验证MarketData
    pub fn validate(&self, data: &MarketData) -> bool {
        // 验证时间
        if !self.validate_time(data.time_sec) {
            return false;
        }
        
        // 验证价格字段
        let price_fields = [
            data.avg_sell_price,
            data.high_price,
            data.low_price,
            data.sell5_price,
            data.sell4_price,
            data.sell3_price,
            data.sell2_price,
            data.sell1_price,
            data.buy1_price,
            data.buy2_price,
            data.buy3_price,
            data.buy4_price,
            data.buy5_price,
        ];
        
        for price in &price_fields {
            if !self.validate_price(*price) {
                return false;
            }
        }
        
        true
    }
}

/// 过滤MarketData列表
pub fn filter_valid_data(data: Vec<MarketData>, validator: &DataValidator) -> Vec<MarketData> {
    data.into_iter()
        .filter(|d| validator.validate(d))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_time() {
        let validator = DataValidator::new(0.01, 10000.0);
        
        // 早盘
        assert!(validator.validate_time(91500));  // 09:15:00
        assert!(validator.validate_time(113000)); // 11:30:00
        assert!(!validator.validate_time(91400)); // 09:14:00
        assert!(!validator.validate_time(113100)); // 11:31:00
        
        // 午盘
        assert!(validator.validate_time(130000)); // 13:00:00
        assert!(validator.validate_time(150000)); // 15:00:00
        assert!(!validator.validate_time(125900)); // 12:59:00
        assert!(!validator.validate_time(150100)); // 15:01:00
    }
    
    #[test]
    fn test_validate_price() {
        let validator = DataValidator::new(0.01, 10000.0);
        
        assert!(validator.validate_price(Some(100.0)));
        assert!(!validator.validate_price(Some(0.0)));
        assert!(!validator.validate_price(Some(-1.0)));
        assert!(!validator.validate_price(Some(f64::NAN)));
        assert!(!validator.validate_price(Some(f64::INFINITY)));
        assert!(validator.validate_price(None));
    }
}
