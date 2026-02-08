#!/bin/bash

# YouSkill 发布脚本
# 用法: ./scripts/release.sh <版本号> [发布说明]
# 示例: ./scripts/release.sh 0.2.0 "添加新功能X"

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查参数
if [ $# -lt 1 ]; then
    echo -e "${RED}错误: 请提供版本号${NC}"
    echo "用法: ./scripts/release.sh <版本号> [发布说明]"
    echo "示例: ./scripts/release.sh 0.2.0"
    exit 1
fi

NEW_VERSION="$1"
RELEASE_NOTES="${2:-"Release v${NEW_VERSION}"}"

# 验证版本号格式 (semver)
if [[ ! "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}错误: 版本号格式不正确，应为 x.y.z${NC}"
    echo "示例: 0.2.0, 1.0.0"
    exit 1
fi

echo -e "${GREEN}🚀 开始发布 YouSkill v${NEW_VERSION}${NC}"
echo ""

# 获取当前版本
CURRENT_VERSION=$(grep -o '"version": "[^"]*"' package.json | head -1 | cut -d'"' -f4)
echo -e "${YELLOW}当前版本: ${CURRENT_VERSION}${NC}"
echo -e "${YELLOW}新版本: ${NEW_VERSION}${NC}"
echo ""

# 检查工作目录是否干净
if [ -n "$(git status --porcelain)" ]; then
    echo -e "${RED}错误: 工作目录有未提交的更改${NC}"
    git status --short
    echo ""
    echo "请先提交或暂存更改后再运行发布脚本"
    exit 1
fi

# 确认发布
echo -e "${YELLOW}是否确认发布? (y/N)${NC}"
read -r confirm
if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
    echo "已取消发布"
    exit 0
fi

echo ""
echo -e "${GREEN}📦 步骤 1/6: 更新版本号...${NC}"

# 更新 package.json
sed -i.bak "s/\"version\": \"${CURRENT_VERSION}\"/\"version\": \"${NEW_VERSION}\"/" package.json
rm -f package.json.bak
echo "  ✓ package.json: ${CURRENT_VERSION} → ${NEW_VERSION}"

# 更新 tauri.conf.json
sed -i.bak "s/\"version\": \"${CURRENT_VERSION}\"/\"version\": \"${NEW_VERSION}\"/" src-tauri/tauri.conf.json
rm -f src-tauri/tauri.conf.json.bak
echo "  ✓ src-tauri/tauri.conf.json: ${CURRENT_VERSION} → ${NEW_VERSION}"

# 更新 Cargo.toml
sed -i.bak "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" src-tauri/Cargo.toml
rm -f src-tauri/Cargo.toml.bak
echo "  ✓ src-tauri/Cargo.toml: ${CURRENT_VERSION} → ${NEW_VERSION}"

echo ""
echo -e "${GREEN}📝 步骤 2/6: 更新 CHANGELOG.md...${NC}"

# 获取当前日期
TODAY=$(date +%Y-%m-%d)

# 创建新的 changelog 条目
NEW_ENTRY="## [${NEW_VERSION}] - ${TODAY}

### Added
- ${RELEASE_NOTES}

"

# 插入到 CHANGELOG.md 的顶部（第一个 ## 之前）
if [ -f CHANGELOG.md ]; then
    # 找到第一个 ## 的位置并插入新条目
    awk -v entry="$NEW_ENTRY" 'NR==1{print} /^## \[/{print entry} {print}' CHANGELOG.md > CHANGELOG.md.tmp
    mv CHANGELOG.md.tmp CHANGELOG.md
    echo "  ✓ CHANGELOG.md 已更新"
else
    echo "# Changelog" > CHANGELOG.md
    echo "" >> CHANGELOG.md
    echo "$NEW_ENTRY" >> CHANGELOG.md
    echo "  ✓ CHANGELOG.md 已创建"
fi

echo ""
echo -e "${GREEN}🔨 步骤 3/6: 本地构建测试...${NC}"
echo "  (跳过构建测试，CI 会处理)"

echo ""
echo -e "${GREEN}📤 步骤 4/6: 提交版本更新...${NC}"

git add package.json
# 尝试添加 Cargo.lock，如果不存在也没关系
if [ -f src-tauri/Cargo.lock ]; then
    git add src-tauri/Cargo.lock
fi

git add src-tauri/tauri.conf.json
# 尝试添加 Cargo.toml
if [ -f src-tauri/Cargo.toml ]; then
    git add src-tauri/Cargo.toml
fi

git add CHANGELOG.md

git commit -m "chore: bump version to ${NEW_VERSION}

${RELEASE_NOTES}"

echo "  ✓ 版本更新已提交"

echo ""
echo -e "${GREEN}🏷️ 步骤 5/6: 创建标签...${NC}"

git tag -a "v${NEW_VERSION}" -m "Release v${NEW_VERSION}

${RELEASE_NOTES}"

echo "  ✓ 标签 v${NEW_VERSION} 已创建"

echo ""
echo -e "${GREEN}🚀 步骤 6/6: 推送到远程...${NC}"

echo -e "${YELLOW}是否推送到远程仓库? (y/N)${NC}"
read -r push_confirm
if [[ "$push_confirm" =~ ^[Yy]$ ]]; then
    git push origin main
    git push origin "v${NEW_VERSION}"
    echo "  ✓ 已推送到远程"
    echo ""
    echo -e "${GREEN}✅ 发布成功!${NC}"
    echo ""
    echo "GitHub Actions 将自动构建并创建 Release"
    echo "请前往 GitHub Releases 页面查看进度:"
    echo "  https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\([^/]*\/[^.]*\).*/\1/')/releases"
else
    echo "  ⚠️  已跳过推送"
    echo ""
    echo "手动推送命令:"
    echo "  git push origin main"
    echo "  git push origin v${NEW_VERSION}"
fi

echo ""
echo -e "${GREEN}🎉 完成!${NC}"
