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

# 检查标签是否已存在
if git rev-parse -q --verify "refs/tags/v${NEW_VERSION}" > /dev/null; then
    echo -e "${RED}错误: 标签 v${NEW_VERSION} 已存在${NC}"
    exit 1
fi

# 检查 CHANGELOG 中是否已存在该版本条目（防止重复发布同一版本）
if [ -f CHANGELOG.md ] && grep -q "^## \[${NEW_VERSION}\]" CHANGELOG.md; then
    echo -e "${RED}错误: CHANGELOG.md 中已存在 [${NEW_VERSION}] 条目${NC}"
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

# 更新 package.json 与 package-lock.json
npm version "${NEW_VERSION}" --no-git-tag-version --allow-same-version > /dev/null
echo "  ✓ package.json / package-lock.json: ${CURRENT_VERSION} → ${NEW_VERSION}"

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

# 函数：提取 Unreleased 部分的正文（不含 ## 标题行）
extract_unreleased_body() {
    local changelog_file="$1"

    awk '
        /^## \[Unreleased\]/ { in_unreleased = 1; next }
        in_unreleased && /^## \[/ { exit }
        in_unreleased { print }
    ' "$changelog_file"
}

# 函数：检查 Unreleased 部分是否有条目
has_unreleased_content() {
    local changelog_file="$1"

    extract_unreleased_body "$changelog_file" | grep -qE '^[[:space:]]*- '
}

# 函数：构建新版本条目
# 保留 Unreleased 中出现的所有分类（含 Breaking Changes、Deprecated、Security 等），
# 按原有顺序输出，并丢弃没有任何条目的空分类。
build_changelog_entry() {
    local version="$1"
    local date="$2"
    local changelog_file="$3"

    printf '## [%s] - %s\n' "$version" "$date"
    extract_unreleased_body "$changelog_file" | awk '
        function flush() {
            if (header != "" && has_content) {
                sub(/\n+$/, "\n", buf)
                printf "%s", buf
            }
            header = ""
            buf = ""
            has_content = 0
        }
        /^### / { flush(); header = $0; buf = "\n" $0 "\n"; next }
        {
            if (header == "") next
            buf = buf $0 "\n"
            if ($0 ~ /[^[:space:]]/) has_content = 1
        }
        END { flush() }
    '
}

if [ ! -f CHANGELOG.md ]; then
    echo -e "${RED}错误: 未找到 CHANGELOG.md${NC}"
    exit 1
fi

unreleased_line=$(grep -n '^## \[Unreleased\]' CHANGELOG.md | cut -d: -f1)

if [ -z "$unreleased_line" ]; then
    echo -e "${RED}错误: CHANGELOG.md 缺少 [Unreleased] 部分${NC}"
    exit 1
fi

if ! has_unreleased_content CHANGELOG.md; then
    echo -e "${RED}错误: CHANGELOG.md 的 [Unreleased] 部分没有任何条目${NC}"
    echo "请先补充本次发布的变更内容，再运行发布脚本"
    exit 1
fi

echo "  从 Unreleased 部分提取内容..."

NEW_ENTRY=$(build_changelog_entry "$NEW_VERSION" "$TODAY" CHANGELOG.md)

# 找到下一个版本条目的行号
next_version_line=$(grep -n '^## \[' CHANGELOG.md | grep -v "Unreleased" | head -1 | cut -d: -f1)

# 创建新的 Unreleased 部分（清空内容，保留项目使用的全部分类）
new_unreleased="## [Unreleased]

### Breaking Changes

### Added

### Changed

### Fixed

### Removed
"

# 组合新文件：头部 + 新 Unreleased + 新版本条目 + 剩余内容
head -n $((unreleased_line - 1)) CHANGELOG.md > CHANGELOG.md.tmp
echo "$new_unreleased" >> CHANGELOG.md.tmp
echo "$NEW_ENTRY" >> CHANGELOG.md.tmp
echo "" >> CHANGELOG.md.tmp
if [ -n "$next_version_line" ]; then
    tail -n +"$next_version_line" CHANGELOG.md >> CHANGELOG.md.tmp
fi
mv CHANGELOG.md.tmp CHANGELOG.md

echo "  ✓ CHANGELOG.md 已更新（Unreleased 内容已迁移到 ${NEW_VERSION}）"

echo ""
echo -e "${GREEN}🔨 步骤 3/6: 更新 Cargo.lock...${NC}"
# 仅刷新 workspace 成员的版本号，不重新解析第三方依赖
cargo update --manifest-path src-tauri/Cargo.toml --workspace --offline
echo "  ✓ Cargo.lock 已更新"

echo ""
echo -e "${GREEN}📤 步骤 4/6: 提交版本更新...${NC}"

# 添加所有修改的文件
git add package.json
git add package-lock.json
git add src-tauri/tauri.conf.json
git add src-tauri/Cargo.toml
git add src-tauri/Cargo.lock
git add CHANGELOG.md

git commit -m "chore: bump version to ${NEW_VERSION}

${RELEASE_NOTES}"

echo "  ✓ 版本更新已提交"

echo ""
echo -e "${GREEN}🏷️ 步骤 5/6: 创建标签...${NC}"

# 从 CHANGELOG.md 提取指定版本的内容（包含分类标题和变更条目）
extract_version_changelog() {
    local version="$1"
    local changelog_file="$2"

    # 找到版本条目的起始行和下一个版本条目的起始行
    local version_line=$(grep -n "^## \[$version\]" "$changelog_file" | cut -d: -f1)
    local next_version_line=$(grep -n "^## \[" "$changelog_file" | grep -v "$version" | grep -v "Unreleased" | sort -n | head -1 | cut -d: -f1)

    if [ -z "$version_line" ]; then
        return 1
    fi

    # 提取版本完整内容（跳过标题行）
    if [ -n "$next_version_line" ]; then
        sed -n "$((version_line + 1)),$((next_version_line - 1))p" "$changelog_file"
    else
        tail -n "+$((version_line + 1))" "$changelog_file"
    fi
}

# 提取版本变更内容
TAG_MESSAGE=$(extract_version_changelog "$NEW_VERSION" CHANGELOG.md)

# 构建完整的 tag message
TAG_MESSAGE="Release v${NEW_VERSION}

${TAG_MESSAGE}"

git tag --cleanup=verbatim -a "v${NEW_VERSION}" -m "$TAG_MESSAGE"

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
