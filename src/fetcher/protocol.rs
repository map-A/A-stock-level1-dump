use anyhow::{Result, Context, anyhow};

/// TongDaXin协议常量
pub const MAGIC: &[u8] = &[0xB1, 0xCB, 0x74, 0x00];  // 0x0074CBB1
pub const ZLIB_HEADERS: &[&[u8]] = &[
    &[0x78, 0x01],
    &[0x78, 0x5E],
    &[0x78, 0x9C],
    &[0x78, 0xDA],
];

/// 默认HELLO请求包
pub const DEFAULT_HELLO: &str = r#"
00 00 00 00  00 00 2a 00  2a 00 c5 02  68 69 73 68
66 2f 64 61  74 65 2f 32  30 32 35 30  36 31 32 2f
73 68 36 30  35 35 59 38  2e 69 6d 67  00 00 00 00
00 00 00 00
"#;

/// 默认请求包模板
pub const DEFAULT_REQUEST: &str = r#"
00 00 00 00  00 00 36 01  36 01 b9 06  00 00 00 00
30 75 00 00  68 69 73 68  66 2f 64 61  74 65 2f 32
30 32 35 30  36 31 32 2f  73 68 36 30  35 35 39 38
2e 69 6d 67  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
"#;

/// offset字段位置
pub const OFFSET_POS1: usize = 0x0C;
pub const OFFSET_POS2: usize = 0x10;

/// 默认步长
pub const DEFAULT_STEP: u32 = 0x7530;  // 30000

/// 路径前缀和固定长度（"hishf/date/YYYYMMDD/shNNNNNN.img" = 32字节）
const PATH_PREFIX: &[u8] = b"hishf/date/";
const PATH_LEN: usize = 32;

/// 解析hexdump字符串为字节
pub fn parse_hexdump(text: &str) -> Result<Vec<u8>> {
    let hex_str: String = text
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();
    
    if hex_str.len() % 2 != 0 {
        return Err(anyhow!("Invalid hex string length"));
    }
    
    (0..hex_str.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex_str[i..i + 2], 16)
                .context("Failed to parse hex byte")
        })
        .collect()
}

/// 原地替换请求包中的日期和股票代码（无额外内存分配）
pub fn replace_date_code(payload: &mut [u8], date: &str, code: &str) -> Result<()> {
    if date.len() != 8 || code.len() != 6 {
        return Err(anyhow!("Invalid date or code format"));
    }
    
    let market = if code.starts_with('6') { "sh" } else { "sz" };
    let new_path = format!("hishf/date/{}/{}{}.img", date, market, code);
    debug_assert_eq!(new_path.len(), PATH_LEN);
    
    if let Some(pos) = payload.windows(PATH_PREFIX.len()).position(|w| w == PATH_PREFIX) {
        if pos + PATH_LEN <= payload.len() {
            payload[pos..pos + PATH_LEN].copy_from_slice(new_path.as_bytes());
            return Ok(());
        }
    }
    
    Err(anyhow!("Path not found in payload"))
}

/// 读取小端序u32
pub fn read_u32_le(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(anyhow!("Offset out of bounds"));
    }
    
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/// 写入小端序u32
pub fn write_u32_le(data: &mut [u8], offset: usize, value: u32) -> Result<()> {
    if offset + 4 > data.len() {
        return Err(anyhow!("Offset out of bounds"));
    }
    
    let bytes = value.to_le_bytes();
    data[offset..offset + 4].copy_from_slice(&bytes);
    Ok(())
}

/// 查找所有匹配的位置
pub fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut start = 0;
    
    while let Some(pos) = haystack[start..].windows(needle.len()).position(|w| w == needle) {
        positions.push(start + pos);
        start += pos + 1;
    }
    
    positions
}

/// 按Magic分割数据块
pub fn slice_blocks(data: &[u8]) -> Vec<&[u8]> {
    let positions = find_all(data, MAGIC);
    
    if positions.is_empty() {
        return vec![];
    }
    
    let mut blocks = Vec::new();
    for (i, &start) in positions.iter().enumerate() {
        let end = positions.get(i + 1).copied().unwrap_or(data.len());
        blocks.push(&data[start..end]);
    }
    
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_hexdump() {
        let hex = "00 01 ff";
        let result = parse_hexdump(hex).unwrap();
        assert_eq!(result, vec![0x00, 0x01, 0xFF]);
    }
    
    #[test]
    fn test_replace_date_code() {
        let mut payload = b"hishf/date/20250612/sh605598.img".to_vec();
        replace_date_code(&mut payload, "20260224", "600519").unwrap();
        assert_eq!(&payload, b"hishf/date/20260224/sh600519.img");
    }
    
    #[test]
    fn test_read_write_u32_le() {
        let mut data = vec![0u8; 8];
        write_u32_le(&mut data, 2, 0x12345678).unwrap();
        assert_eq!(read_u32_le(&data, 2).unwrap(), 0x12345678);
    }
}
