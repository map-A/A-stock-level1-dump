#!/bin/bash
# 测试脚本 - v2.0验证

echo "🧪 A股Level1数据抓取工具 v2.0 - 测试脚本"
echo "================================================"
echo ""

# 检查编译
echo "1️⃣ 检查编译状态..."
if [ -f "target/release/bulk_download" ]; then
    echo "✅ bulk_download 可执行文件存在"
    ls -lh target/release/bulk_download | awk '{print "   大小:", $5}'
else
    echo "❌ 找不到 bulk_download，正在编译..."
    cargo build --release --bin bulk_download
fi
echo ""


# 检查ClickHouse
echo "3️⃣ 检查ClickHouse状态..."
if docker ps | grep -q clickhouse; then
    echo "✅ ClickHouse 容器运行中"
else
    echo "⚠️  ClickHouse 未运行，启动中..."
    docker-compose up -d
    sleep 5
fi
echo ""

# 测试命令格式
echo "4️⃣ 测试命令帮助..."
./target/release/bulk_download 2>&1 | head -3
echo ""


echo "✅ 测试完成！"
echo ""
echo "🚀 开始使用:"
echo "   ./target/release/bulk_download 20260224 50"
echo ""
