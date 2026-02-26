#!/bin/bash
# Stock Fetcher 一键部署脚本

set -e

echo "========================================"
echo "  A股Level1数据抓取系统 - 自动部署"
echo "========================================"
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查命令是否存在
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# 打印成功消息
success() {
    echo -e "${GREEN}✓${NC} $1"
}

# 打印错误消息
error() {
    echo -e "${RED}✗${NC} $1"
}

# 打印警告消息
warning() {
    echo -e "${YELLOW}!${NC} $1"
}

# 检查前置条件
echo "📋 检查前置条件..."

if ! command_exists docker; then
    error "Docker未安装，请先安装Docker"
    exit 1
fi
success "Docker已安装"

if ! command_exists docker-compose; then
    error "Docker Compose未安装，请先安装Docker Compose"
    exit 1
fi
success "Docker Compose已安装"

if ! command_exists python3; then
    warning "Python3未安装，将跳过交易日历生成"
else
    success "Python3已安装"
fi

echo ""

# 步骤1：生成交易日历
if [ -f "data/trading_calendar.bin" ]; then
    success "交易日历文件已存在，跳过生成"
else
    if command_exists python3; then
        echo "📅 生成交易日历..."
        python3 generate_calendar.py
        success "交易日历生成完成"
    else
        error "缺少交易日历文件，请手动运行: python3 generate_calendar.py"
        exit 1
    fi
fi

echo ""

# 步骤2：启动ClickHouse
echo "🐳 启动ClickHouse数据库..."
docker-compose up -d

echo "⏳ 等待ClickHouse启动（15秒）..."
sleep 15

# 检查ClickHouse是否就绪
MAX_RETRIES=5
RETRY=0
while [ $RETRY -lt $MAX_RETRIES ]; do
    if curl -s http://localhost:18123/?query=SELECT%201 > /dev/null 2>&1; then
        success "ClickHouse已就绪"
        break
    else
        RETRY=$((RETRY+1))
        if [ $RETRY -lt $MAX_RETRIES ]; then
            warning "ClickHouse未就绪，重试中... ($RETRY/$MAX_RETRIES)"
            sleep 5
        else
            error "ClickHouse启动失败，请检查日志: docker logs stock-clickhouse"
            exit 1
        fi
    fi
done

echo ""

# 步骤3：创建用户
echo "👤 创建ClickHouse用户..."
curl -s 'http://localhost:18123/?query=CREATE%20USER%20IF%20NOT%20EXISTS%20stock_user%20IDENTIFIED%20BY%20%27stock_pass%27' > /dev/null 2>&1
curl -s 'http://localhost:18123/?query=GRANT%20ALL%20ON%20*.*%20TO%20stock_user' > /dev/null 2>&1
success "用户创建完成"

echo ""

# 步骤4：初始化数据库
echo "🗄️  初始化数据库..."
if [ -f "./target/release/stock-fetcher" ]; then
    BINARY="./target/release/stock-fetcher"
elif [ -f "./stock-fetcher" ]; then
    BINARY="./stock-fetcher"
else
    error "找不到可执行文件 stock-fetcher"
    error "请先构建项目: cargo build --release"
    exit 1
fi

$BINARY --init-db
success "数据库初始化完成"

echo ""

# 步骤5：导入股票信息
echo "📊 导入股票信息..."
$BINARY --import-stocks
success "股票信息导入完成"

echo ""

# 验证部署
echo "🔍 验证部署..."
STOCK_COUNT=$(curl -s 'http://stock_user:stock_pass@localhost:18123/?query=SELECT%20COUNT(*)%20FROM%20stock_db.stock_info')

if [ "$STOCK_COUNT" -gt 0 ]; then
    success "部署验证成功！已导入 $STOCK_COUNT 只股票"
else
    error "部署验证失败，请检查日志"
    exit 1
fi

echo ""
echo "========================================"
echo "  🎉 部署完成！"
echo "========================================"
echo ""
echo "接下来你可以："
echo ""
echo "1. 测试抓取单只股票："
echo "   $BINARY --stocks 600519 --start-date 20260224 --end-date 20260224"
echo ""
echo "2. 批量抓取："
echo "   $BINARY --start-date 20260101 --end-date 20260131 -j 10"
echo ""
echo "3. 查看帮助："
echo "   $BINARY --help"
echo ""
echo "4. 查看数据："
echo "   curl 'http://stock_user:stock_pass@localhost:18123/?query=SELECT%20*%20FROM%20stock_db.stock_info%20LIMIT%205'"
echo ""
