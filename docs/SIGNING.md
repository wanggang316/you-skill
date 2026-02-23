# 代码签名配置指南

## macOS 签名

### 1. 申请 Apple Developer 账号

- 访问 [Apple Developer](https://developer.apple.com) 注册
- 年费：$99 (约 ¥688)
- 等待审核通过

### 2. 生成证书

#### 开发者 ID 应用证书（用于分发）

1. 登录 [Apple Developer Portal](https://developer.apple.com/account/resources/certificates/list)
2. 选择 **Certificates** → **+**
3. 选择 **Developer ID Application**
4. 按提示生成 CSR 文件并上传
5. 下载证书 (.cer 文件)
6. 双击安装到 Keychain Access

#### 导出为 p12 格式

```bash
# 查找证书名称
security find-identity -v -p codesigning

# 导出证书（替换 YOUR_CERTIFICATE_NAME）
security export -k ~/Library/Keychains/login.keychain-db \
  -t identities \
  -f pkcs12 \
  -P "your_password" \
  -o certificate.p12
```

### 3. 生成 App 专用密码

1. 访问 [appleid.apple.com](https://appleid.apple.com)
2. 登录 → **Sign-In and Security** → **App-Specific Passwords**
3. 生成密码（格式如 `xxxx-xxxx-xxxx-xxxx`）

### 4. 配置 GitHub Secrets

在仓库 **Settings** → **Secrets and variables** → **Actions** 中添加：

| Secret                       | 说明                   | 获取方式                                                             |
| ---------------------------- | ---------------------- | -------------------------------------------------------------------- |
| `APPLE_CERTIFICATE`          | Base64 编码的 p12 证书 | `base64 -i certificate.p12`                                          |
| `APPLE_CERTIFICATE_PASSWORD` | p12 导出密码           | 导出时设置的密码                                                     |
| `APPLE_SIGNING_IDENTITY`     | 签名身份               | `security find-identity -v -p codesigning`                           |
| `APPLE_ID`                   | Apple ID 邮箱          | 你的 Apple ID                                                        |
| `APPLE_PASSWORD`             | App 专用密码           | 第3步生成的                                                          |
| `APPLE_TEAM_ID`              | 团队 ID                | [Developer Portal](https://developer.apple.com/account) → Membership |
| `KEYCHAIN_PASSWORD`          | 钥匙串密码             | 任意密码，CI 使用                                                    |

### 5. 公证（Notarization）

GitHub Actions 工作流已配置自动公证，无需手动操作。

---

## Windows 签名（可选）

如果不购买证书，Windows 版本会显示 SmartScreen 警告，但用户可以点击"更多信息" → "仍要运行"来使用。

### 购买代码签名证书

推荐供应商：

- [DigiCert](https://www.digicert.com/)（最可信，最贵）
- [Sectigo](https://sectigo.com/)（性价比高）
- [SSL.com](https://www.ssl.com/)（最便宜）

### 配置 GitHub Secrets

| Secret                         | 说明                   |
| ------------------------------ | ---------------------- |
| `WINDOWS_CERTIFICATE`          | Base64 编码的 pfx 证书 |
| `WINDOWS_CERTIFICATE_PASSWORD` | pfx 密码               |

---

## 本地测试签名

```bash
# macOS 本地构建并签名
npm run tauri build

# 验证签名
codesign -dv --verbose=4 src-tauri/target/release/bundle/macos/YouSkill.app

# 验证公证（发布后）
spctl --assess --type install --context context:primary-similarity src-tauri/target/release/bundle/macos/YouSkill.app
```

---

## 发布流程

1. 提交所有更改到 main 分支
2. 创建并推送标签：
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. GitHub Actions 自动构建并创建 Release
4. 在 [Releases](https://github.com/YOUR_USERNAME/you-skill/releases) 页面下载各平台安装包
