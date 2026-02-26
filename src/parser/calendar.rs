use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use tracing::info;

/// 交易日历
pub struct TradingCalendar {
    trading_days: HashSet<u32>,
    sorted_days: Vec<u32>,
}

impl TradingCalendar {
    /// 从二进制文件加载交易日历
    /// 格式：4字节数量(u32) + N个4字节日期(u32, YYYYMMDD)
    pub fn load(path: &str) -> Result<Self> {
        info!("正在加载交易日历: {}", path);
        
        let mut file = File::open(path)
            .context(format!("无法打开交易日历文件: {}", path))?;
        
        // 读取日期数量
        let mut count_buf = [0u8; 4];
        file.read_exact(&mut count_buf)
            .context("读取日期数量失败")?;
        let count = u32::from_le_bytes(count_buf) as usize;
        
        // 读取所有日期
        let mut trading_days = HashSet::with_capacity(count);
        let mut sorted_days = Vec::with_capacity(count);
        
        for _ in 0..count {
            let mut date_buf = [0u8; 4];
            file.read_exact(&mut date_buf)
                .context("读取日期数据失败")?;
            let date = u32::from_le_bytes(date_buf);
            trading_days.insert(date);
            sorted_days.push(date);
        }
        
        sorted_days.sort_unstable();
        
        info!("✓ 加载了 {} 个交易日 ({} - {})", 
            count, sorted_days[0], sorted_days[sorted_days.len() - 1]);
        
        Ok(Self {
            trading_days,
            sorted_days,
        })
    }
    
    /// 检查是否为交易日
    pub fn is_trading_day(&self, date: u32) -> bool {
        self.trading_days.contains(&date)
    }
    
    /// 获取日期范围内的所有交易日
    pub fn get_trading_days(&self, start: u32, end: u32) -> Vec<u32> {
        self.sorted_days
            .iter()
            .filter(|&&d| d >= start && d <= end)
            .copied()
            .collect()
    }
    
    /// 验证日期格式
    pub fn validate_date(date: u32) -> Result<()> {
        if date < 10000000 || date > 99999999 {
            bail!("无效的日期格式: {}", date);
        }
        
        let year = date / 10000;
        let month = (date / 100) % 100;
        let day = date % 100;
        
        if year < 2015 || year > 2030 {
            bail!("年份超出范围: {}", year);
        }
        if month < 1 || month > 12 {
            bail!("月份超出范围: {}", month);
        }
        if day < 1 || day > 31 {
            bail!("日期超出范围: {}", day);
        }
        
        Ok(())
    }
    
    /// 获取所有交易日
    pub fn all_trading_days(&self) -> &[u32] {
        &self.sorted_days
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_date() {
        assert!(TradingCalendar::validate_date(20250101).is_ok());
        assert!(TradingCalendar::validate_date(20251231).is_ok());
        assert!(TradingCalendar::validate_date(1234).is_err());
        assert!(TradingCalendar::validate_date(20251301).is_err());
    }
}
