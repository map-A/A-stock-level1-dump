use crate::models::MarketData;

/// 协议分隔符
const STX: u8 = 0x02;  // 字段分隔
const ETX: u8 = 0x03;  // 记录起始
const EOT: u8 = 0x04;  // 帧分隔

/// 用结构体代替 HashMap 保存原始字段（零哈希开销，直接字段访问）
#[derive(Default, Clone)]
struct RawRecord {
    code:        Option<String>,
    time:        Option<String>,
    price:       Option<String>,
    cum_volume:  Option<String>,
    cum_amount:  Option<String>,
    cum_trades:  Option<String>,
    high:        Option<String>,
    low:         Option<String>,
    sell5_price: Option<String>, sell5_volume: Option<String>,
    sell4_price: Option<String>, sell4_volume: Option<String>,
    sell3_price: Option<String>, sell3_volume: Option<String>,
    sell2_price: Option<String>, sell2_volume: Option<String>,
    sell1_price: Option<String>, sell1_volume: Option<String>,
    buy1_price:  Option<String>, buy1_volume:  Option<String>,
    buy2_price:  Option<String>, buy2_volume:  Option<String>,
    buy3_price:  Option<String>, buy3_volume:  Option<String>,
    buy4_price:  Option<String>, buy4_volume:  Option<String>,
    buy5_price:  Option<String>, buy5_volume:  Option<String>,
}

/// 迭代帧
fn iter_frames(data: &[u8]) -> Vec<&[u8]> {
    let mut frames = Vec::new();
    let mut start = 0;
    
    while start < data.len() {
        // 查找 [EOT, ETX]
        if let Some(pos) = data[start..].windows(2).position(|w| w == [EOT, ETX]) {
            let end = start + pos;
            if end > start {
                frames.push(&data[start..end]);
            }
            start = end + 2;
        } else {
            // 没找到边界，返回剩余数据
            if start < data.len() {
                frames.push(&data[start..]);
            }
            break;
        }
    }
    
    frames
}

/// Token化（按STX分割）
fn tokenize(record: &[u8]) -> Vec<&[u8]> {
    let mut rec = record;
    
    // 跳过前导ETX
    if !rec.is_empty() && rec[0] == ETX {
        rec = &rec[1..];
    }
    
    if rec.is_empty() {
        return vec![];
    }
    
    rec.split(|&b| b == STX)
        .filter(|t| !t.is_empty())
        .collect()
}

/// 字节转字符串
fn bytes_to_string(b: &[u8]) -> String {
    String::from_utf8_lossy(b).trim().to_string()
}

/// 安全地提取前缀和值（避免UTF-8字符边界问题）
fn extract_prefix_value(s: &str) -> Option<(String, String)> {
    let mut char_indices = s.char_indices();
    char_indices.next()?; // 第1个字符
    let (second_pos, second_char) = char_indices.next()?; // 第2个字符
    // 前缀截止于第2个字符结束处（正确处理多字节UTF-8）
    let end_of_prefix = second_pos + second_char.len_utf8();
    Some((s[..end_of_prefix].to_string(), s[end_of_prefix..].to_string()))
}

/// 解析浮点数
fn parse_float(s: &str) -> Option<f64> {
    s.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0)
}

/// 解析整数
fn parse_u64(s: &str) -> Option<u64> {
    s.parse::<u64>().ok()
}

/// 解析时间（91401.937 -> 91401）
fn parse_time(s: &str) -> Option<u32> {
    // 提取小数点前的部分
    let time_str = s.split('.').next()?;
    time_str.parse::<u32>().ok()
}

/// 解析单个帧，将字段填入 RawRecord（从 last_record 继承缺失字段）
fn parse_frame(frame: &[u8], last_record: &Option<RawRecord>) -> Option<RawRecord> {
    let tokens = tokenize(frame);
    if tokens.is_empty() {
        return None;
    }

    // 继承上一条记录的字段值
    let mut rec = last_record.clone().unwrap_or_default();

    for token in tokens {
        let s = bytes_to_string(token);
        // 字段ID固定为2字节ASCII，值为剩余部分
        if let Some((prefix, value)) = extract_prefix_value(&s) {
            let v = Some(value);
            match prefix.as_str() {
                "01" => rec.code        = v,
                "0T" => rec.time        = v,
                "08" => rec.price       = v,
                "10" => rec.cum_volume  = v,
                "1A" => rec.cum_amount  = v,
                "09" => rec.cum_trades  = v,
                "06" => rec.high        = v,
                "07" => rec.low         = v,
                "44" => rec.sell5_price  = v, "54" => rec.sell5_volume = v,
                "43" => rec.sell4_price  = v, "53" => rec.sell4_volume = v,
                "42" => rec.sell3_price  = v, "52" => rec.sell3_volume = v,
                "41" => rec.sell2_price  = v, "51" => rec.sell2_volume = v,
                "40" => rec.sell1_price  = v, "50" => rec.sell1_volume = v,
                "20" => rec.buy1_price   = v, "30" => rec.buy1_volume  = v,
                "21" => rec.buy2_price   = v, "31" => rec.buy2_volume  = v,
                "22" => rec.buy3_price   = v, "32" => rec.buy3_volume  = v,
                "23" => rec.buy4_price   = v, "33" => rec.buy4_volume  = v,
                "24" => rec.buy5_price   = v, "34" => rec.buy5_volume  = v,
                _ => {}
            }
        }
    }

    Some(rec)
}

/// 解析payload为MarketData列表
pub fn parse_payload(payload: &[u8], trade_date: u32) -> Vec<MarketData> {
    let frames = iter_frames(payload);
    
    let mut records = Vec::new();
    let mut last_record: Option<RawRecord> = None;
    
    // 跳过前两帧（通常是头部信息）
    for (i, frame) in frames.iter().enumerate() {
        if i < 2 {
            continue;
        }
        
        if let Some(record) = parse_frame(frame, &last_record) {
            if let Some(data) = raw_to_market_data(&record, trade_date) {
                records.push(data);
            }
            last_record = Some(record);
        }
    }
    
    records
}

/// RawRecord 转 MarketData
fn raw_to_market_data(r: &RawRecord, trade_date: u32) -> Option<MarketData> {
    let code = r.code.as_ref()?.clone();
    let time_sec = r.time.as_ref().and_then(|s| parse_time(s))?;

    Some(MarketData {
        code,
        trade_date,
        time_sec,
        avg_sell_price: r.price.as_deref().and_then(parse_float),
        cum_volume:     r.cum_volume.as_deref().and_then(parse_u64),
        cum_amount:     r.cum_amount.as_deref().and_then(parse_float),
        cum_trades:     r.cum_trades.as_deref().and_then(parse_u64),
        high_price:     r.high.as_deref().and_then(parse_float),
        low_price:      r.low.as_deref().and_then(parse_float),
        sell5_price:  r.sell5_price.as_deref().and_then(parse_float),
        sell5_volume: r.sell5_volume.as_deref().and_then(parse_u64),
        sell4_price:  r.sell4_price.as_deref().and_then(parse_float),
        sell4_volume: r.sell4_volume.as_deref().and_then(parse_u64),
        sell3_price:  r.sell3_price.as_deref().and_then(parse_float),
        sell3_volume: r.sell3_volume.as_deref().and_then(parse_u64),
        sell2_price:  r.sell2_price.as_deref().and_then(parse_float),
        sell2_volume: r.sell2_volume.as_deref().and_then(parse_u64),
        sell1_price:  r.sell1_price.as_deref().and_then(parse_float),
        sell1_volume: r.sell1_volume.as_deref().and_then(parse_u64),
        buy1_price:  r.buy1_price.as_deref().and_then(parse_float),
        buy1_volume: r.buy1_volume.as_deref().and_then(parse_u64),
        buy2_price:  r.buy2_price.as_deref().and_then(parse_float),
        buy2_volume: r.buy2_volume.as_deref().and_then(parse_u64),
        buy3_price:  r.buy3_price.as_deref().and_then(parse_float),
        buy3_volume: r.buy3_volume.as_deref().and_then(parse_u64),
        buy4_price:  r.buy4_price.as_deref().and_then(parse_float),
        buy4_volume: r.buy4_volume.as_deref().and_then(parse_u64),
        buy5_price:  r.buy5_price.as_deref().and_then(parse_float),
        buy5_volume: r.buy5_volume.as_deref().and_then(parse_u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time("91401.937"), Some(91401));
        assert_eq!(parse_time("150000.000"), Some(150000));
        assert_eq!(parse_time("invalid"), None);
    }
    
    #[test]
    fn test_tokenize() {
        let data = vec![ETX, b'0', b'1', STX, b'0', b'2'];
        let tokens = tokenize(&data);
        assert_eq!(tokens.len(), 2);
    }
}
