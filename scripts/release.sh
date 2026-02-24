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

# 函数：提取 Unreleased 部分中指定分类下的条目
extract_section() {
    local section_name="$1"
    local changelog_file="$2"

    # 先获取 Unreleased 部分的行号范围
    local unreleased_start=$(grep -n '^## \[Unreleased\]' "$changelog_file" | cut -d: -f1)
    local next_version_start=$(grep -n '^## \[' "$changelog_file" | grep -v "Unreleased" | head -1 | cut -d: -f1)

    if [ -z "$unreleased_start" ]; then
        return
    fi

    # 提取 Unreleased 部分的文本
    local unreleased_text
    if [ -n "$next_version_start" ]; then
        unreleased_text=$(sed -n "$((unreleased_start + 1)),$((next_version_start - 1))p" "$changelog_file")
    else
        unreleased_text=$(tail -n "+$((unreleased_start + 1))" "$changelog_file")
    fi

    # 使用 awk 从 Unreleased 部分提取指定分类下的条目
    echo "$unreleased_text" | awk -v section="$section_name" '
        /^### / { current_section = $0; next }
        /^## / { current_section = ""; next }
        current_section ~ section && /^- / { print $0 }
        current_section ~ section && /^[[:space:]]+- / { print $0 }
    '
}

# 函数：检查 Unreleased 部分是否有内容
has_unreleased_content() {
    local changelog_file="$1"
    local unreleased_start=$(grep -n '^## \[Unreleased\]' "$changelog_file" | cut -d: -f1)
    local next_version_start=$(grep -n '^## \[' "$changelog_file" | grep -v "Unreleased" | head -1 | cut -d: -f1)

    if [ -z "$unreleased_start" ]; then
        return 1
    fi

    # 提取 Unreleased 部分的内容
    if [ -n "$next_version_start" ]; then
        local content=$(sed -n "$((unreleased_start + 1)),$((next_version_start - 1))p" "$changelog_file" | grep -c '^- ')
    else
        local content=$(tail -n "+$((unreleased_start + 1))" "$changelog_file" | grep -c '^- ')
    fi

    [ "$content" -gt 0 ]
}

# 函数：构建新版本条目
build_changelog_entry() {
    local version="$1"
    local date="$2"
    local notes="$3"
    local changelog_file="$4"

    local added_items=$(extract_section "Added" "$changelog_file")
    local changed_items=$(extract_section "Changed" "$changelog_file")
    local fixed_items=$(extract_section "Fixed" "$changelog_file")
    local removed_items=$(extract_section "Removed" "$changelog_file")

    local entry="## [${version}] - ${date}
"

    # Added 部分
    if [ -n "$added_items" ]; then
        entry+="
### Added
$added_items"
    elif [ -n "$notes" ]; then
        entry+="
### Added
- $notes"
    fi

    # Changed 部分
    if [ -n "$changed_items" ]; then
        entry+="

### Changed
$changed_items"
    fi

    # Fixed 部分
    if [ -n "$fixed_items" ]; then
        entry+="

### Fixed
$fixed_items"
    fi

    # Removed 部分
    if [ -n "$removed_items" ]; then
        entry+="

### Removed
$removed_items"
    fi

    # 输出条目，末尾添加两个换行符，确保版本之间有空行分隔
    printf "%s\n\n" "$entry"
}

if [ -f CHANGELOG.md ]; then
    # 检查是否存在 Unreleased 部分
    unreleased_line=$(grep -n '^## \[Unreleased\]' CHANGELOG.md | cut -d: -f1)

    if [ -n "$unreleased_line" ] && has_unreleased_content CHANGELOG.md; then
        echo "  从 Unreleased 部分提取内容..."

        # 构建新版本条目（使用 Unreleased 内容）
        NEW_ENTRY=$(build_changelog_entry "$NEW_VERSION" "$TODAY" "$RELEASE_NOTES" CHANGELOG.md)

        # 找到下一个版本条目的行号
        next_version_line=$(grep -n '^## \[' CHANGELOG.md | grep -v "Unreleased" | head -1 | cut -d: -f1)

        if [ -n "$next_version_line" ]; then
            # 创建新的 Unreleased 部分（清空内容）
            new_unreleased="## [Unreleased]

### Added

### Changed

### Fixed

### Removed
"
            # 组合新文件：头部 + 新 Unreleased + 新版本条目 + 剩余内容
            head -n $((unreleased_line - 1)) CHANGELOG.md > CHANGELOG.md.tmp
            echo "$new_unreleased" >> CHANGELOG.md.tmp
            echo "" >> CHANGELOG.md.tmp
            echo "$NEW_ENTRY" >> CHANGELOG.md.tmp
            echo "" >> CHANGELOG.md.tmp
            tail -n +$next_version_line CHANGELOG.md >> CHANGELOG.md.tmp
            mv CHANGELOG.md.tmp CHANGELOG.md
        fi
        echo "  ✓ CHANGELOG.md 已更新（从 Unreleased 迁移内容）"
    else
        # 没有 Unreleased 内容，使用传统方式
        NEW_ENTRY="## [${NEW_VERSION}] - ${TODAY}

### Added
- ${RELEASE_NOTES}

"

        # 插入到 Unreleased 部分之后（或第一个版本条目之前）
        first_version=$(grep -n '^## \[' CHANGELOG.md | grep -v "Unreleased" | head -1 | cut -d: -f1)

        if [ -n "$unreleased_line" ]; then
            # 在 Unreleased 后插入
            if [ -n "$first_version" ]; then
                head -n $((first_version - 1)) CHANGELOG.md > CHANGELOG.md.tmp
                echo "$NEW_ENTRY" >> CHANGELOG.md.tmp
                echo "" >> CHANGELOG.md.tmp
                tail -n +$first_version CHANGELOG.md >> CHANGELOG.md.tmp
                mv CHANGELOG.md.tmp CHANGELOG.md
            fi
        elif [ -n "$first_version" ]; then
            # 在第一个版本条目前插入
            head -n $((first_version - 1)) CHANGELOG.md > CHANGELOG.md.tmp
            echo "$NEW_ENTRY" >> CHANGELOG.md.tmp
            echo "" >> CHANGELOG.md.tmp
            tail -n +$first_version CHANGELOG.md >> CHANGELOG.md.tmp
            mv CHANGELOG.md.tmp CHANGELOG.md
        else
            echo "$NEW_ENTRY" >> CHANGELOG.md
        fi
        echo "  ✓ CHANGELOG.md 已更新"
    fi
else
    # 创建新的 CHANGELOG.md
    cat > CHANGELOG.md << 'EOF'
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

EOF
    echo "## [Unreleased]" >> CHANGELOG.md
    echo "" >> CHANGELOG.md
    echo "### Added" >> CHANGELOG.md
    echo "" >> CHANGELOG.md
    echo "### Changed" >> CHANGELOG.md
    echo "" >> CHANGELOG.md
    echo "### Fixed" >> CHANGELOG.md
    echo "" >> CHANGELOG.md
    echo "### Removed" >> CHANGELOG.md
    echo "" >> CHANGELOG.md
    echo "" >> CHANGELOG.md
    echo "## [${NEW_VERSION}] - ${TODAY}" >> CHANGELOG.md
    echo "" >> CHANGELOG.md
    echo "### Added" >> CHANGELOG.md
    echo "- ${RELEASE_NOTES}" >> CHANGELOG.md
    echo "  ✓ CHANGELOG.md 已创建"
fi

echo ""
echo -e "${GREEN}🔨 步骤 3/6: 更新 Cargo.lock...${NC}"
# 更新 Cargo.lock 以反映新版本
cd src-tauri && cargo generate-lockfile && cd ..
echo "  ✓ Cargo.lock 已更新"

echo ""
echo -e "${GREEN}📤 步骤 4/6: 提交版本更新...${NC}"

# 添加所有修改的文件
git add package.json
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
