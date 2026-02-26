/// 全量下载工具 - 极致性能版本
/// 
/// 特性：
/// - 断点续传：自动跳过已下载的股票
/// - 并发下载：支持高并发抓取
/// - 错误重试：失败自动重试3次
/// - 流水线：下载/解析与DB写入并行，跨股批量写入

use anyhow::Result;
use tracing::{info, error, warn};
use futures::stream::{self, StreamExt};
use std::time::Instant;
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::mpsc;

// 引用主程序的模块
use stock_fetcher::{config, db, fetcher, utils};
use stock_fetcher::models::MarketData;

/// DB写入批次大小：跨股累积更多行再写入，减少 insert 次数
const DB_BATCH_SIZE: usize = 10_000;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("🚀 全量下载工具 v3.0");
    
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("用法: {} <YYYYMMDD> [并发数] [--force]", args[0]);
        eprintln!("示例: {} 20260224 50", args[0]);
        eprintln!("      {} 20260224 50 --force  (强制重新下载)", args[0]);
        std::process::exit(1);
    }
    
    let date: u32 = args[1].parse()?;
    let concurrent = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(50);
    let force = args.get(3).map(|s| s == "--force").unwrap_or(false);
    
    info!("日期: {}, 并发: {}, 模式: {}", date, concurrent, if force { "强制" } else { "增量" });
    
    let config = config::Config::load("config.toml")?;
    
    info!("读取股票代码...");
    let mut codes = db::get_all_stock_codes(&config).await?;
    info!("共 {} 只股票", codes.len());
    
    // 断点续传：查询已下载的股票
    if !force {
        info!("检查已下载股票...");
        let downloaded = get_downloaded_codes(&config, date).await?;
        if !downloaded.is_empty() {
            codes.retain(|code| !downloaded.contains(code));
            info!("已下载 {} 只，剩余 {} 只", downloaded.len(), codes.len());
        }
        
        if codes.is_empty() {
            info!("✅ 所有股票已下载完成！");
            return Ok(());
        }
    }
    
    let client = Arc::new(fetcher::HighPerfTcpClient::new(
        config.server.host.clone(),
        config.server.port,
        config.server.timeout_secs,
        concurrent * 2,
    )?);
    
    let db_client = Arc::new(db::ClickHouseClient::new(&config));
    let validator = Arc::new(utils::DataValidator::new(
        config.validation.min_price,
        config.validation.max_price,
    ));
    
    let total = codes.len();
    let start = Instant::now();

    // ── 流水线：下载任务 → channel → DB写入任务 ──────────────────────────
    // channel 容量：每只股票约 1000 条，512 个缓冲足以解耦下载与 DB 写入
    let (tx, mut rx) = mpsc::channel::<Vec<MarketData>>(512);

    // 独立 DB 写入任务：跨股累积 DB_BATCH_SIZE 条后统一写入
    let db_writer = {
        let db = db_client.clone();
        tokio::spawn(async move {
            let mut total_records = 0u64;
            let mut batch: Vec<MarketData> = Vec::with_capacity(DB_BATCH_SIZE);

            while let Some(data) = rx.recv().await {
                batch.extend(data);
                if batch.len() >= DB_BATCH_SIZE {
                    match db.insert_market_data(&batch).await {
                        Ok(_) => total_records += batch.len() as u64,
                        Err(e) => error!("DB批量写入失败: {}", e),
                    }
                    batch.clear();
                }
            }
            // 刷新剩余数据
            if !batch.is_empty() {
                match db.insert_market_data(&batch).await {
                    Ok(_) => total_records += batch.len() as u64,
                    Err(e) => error!("DB末尾写入失败: {}", e),
                }
            }
            total_records
        })
    };

    // 下载任务：只负责抓取/解析/过滤，结果发送到 channel
    let mut success = 0usize;
    let mut failed = Vec::new();

    {
        // tx_dl 是 stream 闭包内使用的发送端；drop(tx_dl) 发生在 stream 结束后
        let tx_dl = tx.clone();

        let mut stream = stream::iter(codes.into_iter().enumerate())
            .map(|(i, code)| {
                let c = client.clone();
                let v = validator.clone();
                let tx = tx_dl.clone();
                let max_retries = 3;
                
                async move {
                    for retry in 0..max_retries {
                        match c.fetch(&code, date).await {
                            Ok(mut data) => {
                                let raw_count = data.len();
                                data = utils::filter_valid_data(data, &v);
                                let valid_count = data.len();
                                
                                if !data.is_empty() {
                                    if (i + 1) % 10 == 0 {
                                        info!("[{}] {} - {} 条 (过滤{})", 
                                            i+1, code, valid_count, raw_count - valid_count);
                                    }
                                    let _ = tx.send(data).await;
                                    return (1usize, code);
                                } else {
                                    warn!("{} 无有效数据 (原始{}条)", code, raw_count);
                                    return (1, code);
                                }
                            }
                            Err(e) => {
                                if retry < max_retries - 1 {
                                    warn!("{} 失败(重试{}/{}): {}", code, retry+1, max_retries-1, e);
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                } else {
                                    error!("{} 失败(放弃): {}", code, e);
                                }
                            }
                        }
                    }
                    (0usize, code)
                }
            })
            .buffer_unordered(concurrent);

        let mut done = 0usize;
        while let Some((ok, code)) = stream.next().await {
            success += ok;
            done += 1;
            if ok == 0 {
                failed.push(code);
            }

            if done % 100 == 0 {
                let progress = done as f64 / total as f64 * 100.0;
                let elapsed = start.elapsed().as_secs_f64();
                let speed = done as f64 / elapsed;
                let eta = (total - done) as f64 / speed;
                info!("📊 进度: {:.1}% ({}/{}), 速度: {:.1}股/秒, 预计剩余: {:.0}秒",
                    progress, done, total, speed, eta);
            }
        }
        // tx_dl 在此处 drop，stream 也在此处 drop
    }

    // 关闭 channel，让 DB 写入任务知晓下载已全部完成
    drop(tx);
    let records = db_writer.await.unwrap_or(0);

    let dur = start.elapsed();
    info!("✅ 完成! 成功{}/{}, 记录{}, 耗时{:.1}秒, 速度{:.1}股/秒",
        success, total, records, dur.as_secs_f64(), total as f64 / dur.as_secs_f64());
    
    if !failed.is_empty() {
        warn!("⚠️  {} 只股票失败:", failed.len());
        for (i, code) in failed.iter().take(20).enumerate() {
            warn!("  {}: {}", i+1, code);
        }
        if failed.len() > 20 {
            warn!("  ... 还有 {} 只", failed.len() - 20);
        }
    }
    
    Ok(())
}

/// 查询已下载的股票代码
async fn get_downloaded_codes(config: &config::Config, date: u32) -> Result<HashSet<String>> {
    let date_str = format!("{}-{:02}-{:02}", date/10000, (date/100)%100, date%100);
    
    let client = clickhouse::Client::default()
        .with_url(&config.clickhouse.url)
        .with_user(&config.clickhouse.username)
        .with_password(&config.clickhouse.password)
        .with_database(&config.clickhouse.database);
    
    let query = format!(
        "SELECT DISTINCT code FROM market_data WHERE trade_date = '{}'",
        date_str
    );
    
    let result: Vec<String> = client.query(&query).fetch_all::<String>().await?;
    Ok(result.into_iter().collect())
}
