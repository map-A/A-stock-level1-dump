use crate::models::MarketData;

/// 数据验证器
pub struct DataValidator {
}

impl DataValidator {
    pub fn new() -> Self {
        DataValidator {}
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
    
    
    /// 验证MarketData
    pub fn validate(&self, data: &MarketData) -> bool {
        // 验证时间
        if !self.validate_time(data.time_sec) {
            return false;
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
        let validator = DataValidator::new();
        
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
    
}
