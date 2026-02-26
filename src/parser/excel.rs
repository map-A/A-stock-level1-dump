use anyhow::{Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use crate::models::StockInfo;
use tracing::info;

/// 解析股票列表Excel文件
pub fn parse_stock_list(path: &str) -> Result<Vec<StockInfo>> {
    info!("正在解析Excel文件: {}", path);
    
    let mut workbook: Xlsx<_> = open_workbook(path)
        .context(format!("无法打开Excel文件: {}", path))?;
    
    // 获取第一个sheet
    let sheet_name = workbook.sheet_names()[0].clone();
    let range = workbook
        .worksheet_range(&sheet_name)
        .context("无法读取工作表")?;
    
    let mut stocks = Vec::new();
    
    // 跳过表头，从第2行开始
    for row in range.rows().skip(1) {
        if row.len() < 10 {
            continue;
        }
        
        let stock = StockInfo {
            code: row[0].to_string().trim().to_string(),
            name: row[1].to_string().trim().to_string(),
            industry_l1_code: row[2].to_string().trim().to_string(),
            industry_l1_name: row[3].to_string().trim().to_string(),
            industry_l2_code: row[4].to_string().trim().to_string(),
            industry_l2_name: row[5].to_string().trim().to_string(),
            industry_l3_code: row[6].to_string().trim().to_string(),
            industry_l3_name: row[7].to_string().trim().to_string(),
            industry_l4_code: row[8].to_string().trim().to_string(),
            industry_l4_name: row[9].to_string().trim().to_string(),
        };
        
        // 验证股票代码格式
        if stock.code.len() == 6 && stock.code.chars().all(|c| c.is_numeric()) {
            stocks.push(stock);
        }
    }
    
    info!("✓ 成功解析 {} 只股票", stocks.len());
    Ok(stocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_stock_list() {
        // 需要实际的Excel文件
        if let Ok(stocks) = parse_stock_list("行业分类.xlsx") {
            assert!(stocks.len() > 0);
            assert_eq!(stocks[0].code.len(), 6);
        }
    }
}
