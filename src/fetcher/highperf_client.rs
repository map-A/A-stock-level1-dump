use anyhow::{Result, Context, anyhow};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};
use std::sync::Arc;
use parking_lot::Mutex;
use crate::models::MarketData;
use super::protocol::{
    parse_hexdump, replace_date_code, write_u32_le, read_u32_le,
    DEFAULT_HELLO, DEFAULT_REQUEST, OFFSET_POS1, OFFSET_POS2, DEFAULT_STEP
};
use super::extract::extract_payloads;
use super::parser::parse_payload;

/// 已认证的连接（HELLO已完成）
pub struct AuthenticatedStream {
    stream: TcpStream,
}

/// 连接池
pub struct ConnectionPool {
    host: String,
    port: u16,
    timeout_secs: u64,
    pool: Arc<Mutex<Vec<AuthenticatedStream>>>,
    max_size: usize,
    hello_template: Vec<u8>,
}

impl ConnectionPool {
    pub fn new(host: String, port: u16, timeout_secs: u64, max_size: usize, hello_template: Vec<u8>) -> Self {
        Self {
            host,
            port,
            timeout_secs,
            pool: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            max_size,
            hello_template,
        }
    }
    
    /// 获取已认证的连接（从池中或新建+HELLO）
    async fn get_connection(&self) -> Result<AuthenticatedStream> {
        // 尝试从池中获取
        if let Some(conn) = self.pool.lock().pop() {
            return Ok(conn);
        }
        
        // 创建新连接并完成HELLO握手
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = timeout(
            Duration::from_secs(self.timeout_secs),
            TcpStream::connect(&addr)
        ).await
            .context("Connection timeout")?
            .context("Failed to connect")?;
        
        // 发送HELLO握手（只做一次）
        stream.write_all(&self.hello_template).await
            .context("Failed to send HELLO")?;
        
        // 接收HELLO响应
        let mut buf = vec![0u8; 1024];
        timeout(
            Duration::from_millis(1200),
            stream.read(&mut buf)
        ).await
            .context("HELLO response timeout")?
            .context("Failed to read HELLO response")?;
        
        Ok(AuthenticatedStream { stream })
    }
    
    /// 归还连接到池
    fn return_connection(&self, conn: AuthenticatedStream) {
        let mut pool = self.pool.lock();
        if pool.len() < self.max_size {
            pool.push(conn);
        }
        // 否则连接被drop自动关闭
    }
}

/// 高性能TCP客户端（带连接池+HELLO复用）
pub struct HighPerfTcpClient {
    pool: Arc<ConnectionPool>,
    // 预解析的请求模板
    request_template: Vec<u8>,
}

impl HighPerfTcpClient {
    pub fn new(host: String, port: u16, timeout_secs: u64, pool_size: usize) -> Result<Self> {
        // 预解析模板
        let hello_template = parse_hexdump(DEFAULT_HELLO)
            .context("Failed to parse HELLO template")?;
        let request_template = parse_hexdump(DEFAULT_REQUEST)
            .context("Failed to parse REQUEST template")?;
        
        let pool = Arc::new(ConnectionPool::new(host, port, timeout_secs, pool_size, hello_template));
        
        Ok(Self {
            pool,
            request_template,
        })
    }
    
    /// 接收数据直到静默
    /// - first_byte_ms: 等待服务器开始响应的最长时间（含查询时间）
    /// - quiet_ms: 收到数据后，判断传输结束的静默超时（只需覆盖TCP分段间隔）
    async fn recv_until_quiet(stream: &mut TcpStream, first_byte_ms: u64, quiet_ms: u64) -> Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(256 * 1024);
        let mut chunk = vec![0u8; 16384];
        
        // 第一次读：用较长的超时等待服务器开始响应
        match timeout(Duration::from_millis(first_byte_ms), stream.read(&mut chunk)).await {
            Ok(Ok(n)) if n > 0 => buffer.extend_from_slice(&chunk[..n]),
            Ok(Ok(_)) => return Ok(buffer),
            Ok(Err(e)) => return Err(anyhow!("Read error: {}", e)),
            Err(_) => return Ok(buffer), // 服务器无响应，返回空
        }
        
        // 后续读：用短超时检测静默（数据已在传输中，只需等TCP分段间隔）
        loop {
            match timeout(Duration::from_millis(quiet_ms), stream.read(&mut chunk)).await {
                Ok(Ok(n)) if n > 0 => buffer.extend_from_slice(&chunk[..n]),
                Ok(Ok(_)) => break,
                Ok(Err(e)) => return Err(anyhow!("Read error: {}", e)),
                Err(_) => break,
            }
        }
        
        Ok(buffer)
    }
    
    /// 抓取数据（优化版：复用HELLO认证）
    pub async fn fetch(&self, code: &str, date: u32) -> Result<Vec<MarketData>> {
        let date_str = date.to_string();
        
        // 获取已认证的连接（HELLO已完成）
        let mut conn = self.pool.get_connection().await?;
        
        // 复制并原地替换请求模板（无额外分配）
        let mut request = self.request_template.clone();
        replace_date_code(&mut request, &date_str, code)
            .context("Failed to replace date/code in REQUEST")?;
        
        // 读取初始offset
        let initial_offset = read_u32_le(&request, OFFSET_POS2)?;
        
        let mut all_responses = Vec::with_capacity(4);
        let mut baseline_size: Option<usize> = None;
        
        // 分页循环（最多4页）
        for page in 0..4 {
            let offset = initial_offset + DEFAULT_STEP * page;
            
            // 更新offset（直接修改）
            write_u32_le(&mut request, OFFSET_POS1, DEFAULT_STEP * page)?;
            write_u32_le(&mut request, OFFSET_POS2, offset)?;
            
            // 发送请求
            conn.stream.write_all(&request).await?;
            
            // 接收响应：1000ms等待首字节（服务器查询时间），80ms检测后续静默
            let response = Self::recv_until_quiet(&mut conn.stream, 1000, 80).await?;
            let got = response.len();
            
            if got == 0 {
                break;
            }
            
            // 短页停判定
            if let Some(baseline) = baseline_size {
                let threshold = std::cmp::max((baseline as f64 * 0.6) as usize, baseline.saturating_sub(4096));
                if got < threshold {
                    all_responses.push(response);
                    break;
                }
            } else {
                baseline_size = Some(got);
            }
            
            all_responses.push(response);
        }
        
        // 接收尾部数据：已收到数据，80ms静默即可排空
        let _tail = Self::recv_until_quiet(&mut conn.stream, 80, 80).await.ok();
        
        // 归还连接（仍然保持HELLO认证状态）
        self.pool.return_connection(conn);
        
        // 合并响应（优化：预分配）
        let total_size: usize = all_responses.iter().map(|r| r.len()).sum();
        let mut combined_response = Vec::with_capacity(total_size);
        for resp in all_responses {
            combined_response.extend(resp);
        }
        
        // 提取payload
        let payloads = extract_payloads(&combined_response)?;
        
        // 解析所有payload（优化：预分配）
        let mut all_records = Vec::with_capacity(payloads.len() * 100);
        for payload in &payloads {
            let records = parse_payload(payload, date);
            all_records.extend(records);
        }
        
        Ok(all_records)
    }
}
