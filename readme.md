# A股Level1数据抓取工具 

**Rust高性能实现**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![ClickHouse](https://img.shields.io/badge/clickhouse-24.1-green.svg)](https://clickhouse.com/)

## ✨ 特性

- 🚀 **极致性能**: 5589只股票从15.5小时降至3分钟(mac mini4 测试)
- 🔄 **断点续传**: 中断后自动继续，无需重新下载
- ✅ **数据验证**: 自动过滤无效数据（时间、价格、成交量）
- 🔒 **稳定可靠**: UTF-8边界问题修复，自动重试机制
- 📊 **实时监控**: 进度追踪、速度统计、ETA预测
- 🎯 **生产就绪**: ClickHouse批量操作，连接池优化

## 🚀 快速开始

```bash
# 1. 启动数据库
docker-compose up -d

# 2. 编译项目
cargo build --release


# 3. 导入股票代码数据，然后下载对应股票的数据
./target/release/bulk_download 20260224 100
```

## 📊 性能对比

| 方案 | 单股耗时 | 全量耗时(5183股) |
|------|---------|------------------|
| Python | 10秒 | 15.5小时 |
| **Rust v2.0** | **1.4秒** | **3分钟** |

## 🆕 v2.0 更新（2026-02-25）

### 关键修复
- ✅ **UTF-8崩溃修复**: 安全处理非ASCII字符，不再随机崩溃
- ✅ **断点续传**: 自动跳过已下载股票
- ✅ **自动重试**: 失败重试3次，提升成功率
- ✅ **失败追踪**: 记录并汇总所有失败股票

### 易用性提升
- 📈 进度百分比和ETA显示
- 🔍 数据过滤统计（显示过滤的记录数）
- ⚠️ 无效数据警告
- 📝 更清晰的日志输出


## 使用说明

### 1. 环境要求

- Rust 1.70+
- Docker（运行ClickHouse）

### 2. 启动ClickHouse

```bash
docker-compose up -d
```
这里数据库初始化完成以后需要等待一段时间，直接进行下面下载步骤可能会遇到网络连接失败

### 3. 编译项目

```bash
cargo build --release
```


### 4. 下载全量数据

```bash
# 基本用法：日期 + 并发数
./target/release/bulk_download 20260224 50

# 断点续传（中断后继续）
./target/release/bulk_download 20260224 50

# 强制重新下载
./target/release/bulk_download 20260224 50 --force

```

### 6. 查看数据

```bash
# 查看下载的股票数
curl -s 'http://localhost:8123/?user=stock_user&password=stock_pass&database=stock_db' \
  --data "SELECT count(DISTINCT code) FROM market_data WHERE trade_date='2026-02-24'"

# 查看某只股票的数据
curl -s 'http://localhost:8123/?user=stock_user&password=stock_pass&database=stock_db' \
  --data "SELECT * FROM market_data WHERE code='600519' AND trade_date='2026-02-24' LIMIT 10"
```
# 清空旧数据（如需要）
curl 'http://localhost:8123/?user=stock_user&password=stock_pass&database=stock_db' \
  --data "DELETE FROM market_data WHERE trade_date='2026-02-24'"


### 配置文件 (config.toml)

```toml
[fetcher]
max_concurrent = 50      # 最大并发数
request_delay_ms = 10    # 请求延迟（避免限流）
max_retries = 3          # 最大重试次数

[clickhouse]
batch_size = 2000        # 批量插入大小

[server]
timeout_secs = 5         # 连接超时
```


### 某些股票总是失败？

程序会在最后汇总失败的股票：
```
⚠️  39 只股票失败:
  1: 000004
  2: 000636
  ...
```

可以：
1. 检查这些股票是否停牌
2. 单独重试这些股票
3. 查看详细错误日志

## 🛠️ 技术栈

- **语言**: Rust 2021
- **异步运行时**: Tokio
- **数据库**: ClickHouse 24.1
- **协议**: TongDaXin TCP (原生实现)
- **压缩**: Zlib (flate2)
- **并发**: Connection Pool + buffer_unordered

## 📄 License

MIT License

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

---
