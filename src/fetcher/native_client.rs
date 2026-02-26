use anyhow::{Result, Context, anyhow};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};
use tracing::{info, debug};
use crate::models::MarketData;
use super::protocol::{
    parse_hexdump, replace_date_code, write_u32_le, read_u32_le,
    DEFAULT_HELLO, DEFAULT_REQUEST, OFFSET_POS1, OFFSET_POS2, DEFAULT_STEP
};
use super::extract::extract_payloads;
use super::parser::parse_payload;

pub struct NativeTcpClient {
    host: String,
    port: u16,
    timeout_secs: u64,
}

impl NativeTcpClient {
    pub fn new(host: String, port: u16, timeout_secs: u64) -> Self {
        Self {
            host,
            port,
            timeout_secs,
        }
    }
    
    /// 接收数据直到静默
    async fn recv_until_quiet(stream: &mut TcpStream, quiet_ms: u64) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        let mut chunk = vec![0u8; 8192];
        
        loop {
            match timeout(Duration::from_millis(quiet_ms), stream.read(&mut chunk)).await {
                Ok(Ok(n)) if n > 0 => {
                    buffer.extend_from_slice(&chunk[..n]);
                    debug!("Received {} bytes, total: {}", n, buffer.len());
                }
                Ok(Ok(_)) => {
                    // 连接关闭
                    break;
                }
                Ok(Err(e)) => {
                    return Err(anyhow!("Read error: {}", e));
                }
                Err(_) => {
                    // 超时，认为数据接收完毕
                    break;
                }
            }
        }
        
        Ok(buffer)
    }
    
    /// 抓取数据
    pub async fn fetch(&self, code: &str, date: u32) -> Result<Vec<MarketData>> {
        let date_str = date.to_string();
        
        info!("正在抓取: {} {}", code, date_str);
        
        // 连接服务器
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = timeout(
            Duration::from_secs(self.timeout_secs),
            TcpStream::connect(&addr)
        ).await
            .context("Connection timeout")?
            .context("Failed to connect")?;
        
        // 准备HELLO请求
        let mut hello_payload = parse_hexdump(DEFAULT_HELLO)
            .context("Failed to parse HELLO")?;
        replace_date_code(&mut hello_payload, &date_str, code)
            .context("Failed to replace date/code in HELLO")?;
        
        // 发送HELLO
        stream.write_all(&hello_payload).await
            .context("Failed to send HELLO")?;
        
        // 接收HELLO响应
        let _hello_resp = Self::recv_until_quiet(&mut stream, 1200).await
            .context("Failed to receive HELLO response")?;
        
        debug!("HELLO response: {} bytes", _hello_resp.len());
        
        // 准备请求模板
        let mut template = parse_hexdump(DEFAULT_REQUEST)
            .context("Failed to parse REQUEST template")?;
        replace_date_code(&mut template, &date_str, code)
            .context("Failed to replace date/code in REQUEST")?;
        
        let mut all_responses = Vec::new();
        let mut baseline_size = None;
        
        // 分页循环（最多4页）
        // 注意：只更新pos1，pos2保持模板中的固定值不变（与Python版本行为一致）
        for page in 0..999u32 {
            // 更新请求中的offset（仅更新pos1）
            let mut request = template.clone();
            write_u32_le(&mut request, OFFSET_POS1, DEFAULT_STEP * page)
                .context("Failed to write offset1")?;
            
            // 发送请求
            stream.write_all(&request).await
                .context(format!("Failed to send request page {}", page))?;
            
            // 接收响应
            let response = Self::recv_until_quiet(&mut stream, 1200).await
                .context(format!("Failed to receive response page {}", page))?;
            
            let got = response.len();
            debug!("Page {}: received {} bytes", page, got);
            
            if got == 0 {
                break;
            }
            
            // 短页停判定
            if let Some(baseline) = baseline_size {
                let threshold = std::cmp::max((baseline as f64 * 0.6) as usize, baseline - 4096);
                if got < threshold {
                    debug!("Short page detected: {} < {}, stopping", got, threshold);
                    all_responses.push(response);
                    break;
                }
            } else {
                baseline_size = Some(got);
            }
            
            all_responses.push(response);
        }
        
        // 接收尾部数据
        let _tail = Self::recv_until_quiet(&mut stream, 1500).await.ok();
        
        // 合并响应：第0页完整保留；第1+页若以MAGIC开头则去掉前20字节的分片头
        let mut combined_response = Vec::new();
        for (i, resp) in all_responses.iter().enumerate() {
            if i == 0 {
                combined_response.extend_from_slice(resp);
            } else if resp.len() > 20 && resp.starts_with(super::protocol::MAGIC) {
                combined_response.extend_from_slice(&resp[20..]);
            } else {
                combined_response.extend_from_slice(resp);
            }
        }
        
        debug!("Total response size: {} bytes", combined_response.len());
        
        // 提取payload
        let payloads = extract_payloads(&combined_response)
            .context("Failed to extract payloads")?;
        
        debug!("Extracted {} payloads", payloads.len());
        
        // 解析所有payload
        let mut all_records = Vec::new();
        for payload in &payloads {
            let records = parse_payload(payload, date);
            all_records.extend(records);
        }
        
        info!("抓取完成: {} {} - {} 条记录", code, date_str, all_records.len());
        
        Ok(all_records)
    }
}

#[cfg(test)]
mod tests {    
    #[tokio::test]
    async fn test_native_client() {
        // 这个测试需要真实的服务器连接
        // 仅用于手动测试
    }
}
