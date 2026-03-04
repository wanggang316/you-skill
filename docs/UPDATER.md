# Tauri 自动更新配置指南

Tauri 的更新程序需要签名来验证更新来自可信来源，**此要求无法禁用**。

需要两个密钥：
- **公钥**：配置在 `tauri.conf.json` 中，用于验证升级包，可安全公开
- **私钥**：用于对安装程序文件签名，**切勿泄露，且丢失后将无法向已安装用户推送更新**

---

## 1. 生成 Tauri 签名密钥

```bash
npm run tauri signer generate -- -w ~/.tauri/you-skill.key
```

执行过程中会提示输入私钥密码（可留空，但建议设置）。执行完成后生成两个文件：
- `~/.tauri/you-skill.key` — 私钥
- `~/.tauri/you-skill.key.pub` — 公钥

---

## 2. 配置本地环境变量

在项目根目录的 `.env` 文件中添加（如无则新建）：

```bash
export TAURI_SIGNING_PRIVATE_KEY="<~/.tauri/you-skill.key 的内容>"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<生成时输入的密码，留空则省略此行>"
```

查看私钥内容：

```bash
cat ~/.tauri/you-skill.key
```

> `.env` 文件已在 `.gitignore` 中忽略，请确保不要提交到版本库。

---

## 3. 配置 tauri.conf.json

编辑 `src-tauri/tauri.conf.json`，在 `plugins.updater` 中填入公钥和 endpoints：

```json
"plugins": {
  "updater": {
    "pubkey": "<~/.tauri/you-skill.key.pub 的内容>",
    "endpoints": [
      "https://github.com/wanggang316/you-skill/releases/latest/download/latest.json"
    ]
  }
}
```

查看公钥内容：

```bash
cat ~/.tauri/you-skill.key.pub
```

当前仓库地址为 `wanggang316/you-skill`（通过 `git remote -v` 确认）。

---

## 4. 配置 GitHub Actions Secrets

在仓库 **Settings** → **Secrets and variables** → **Actions** 中添加以下 Secrets：

| Secret                              | 说明                                  | 获取方式                                       |
| ----------------------------------- | ------------------------------------- | ---------------------------------------------- |
| `TAURI_SIGNING_PRIVATE_KEY`         | 私钥内容                              | `cat ~/.tauri/you-skill.key`                   |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`| 生成密钥时输入的密码（留空则值为空）  | 生成时手动输入的密码                           |

---

## 5. 本地测试构建

确保 `.env` 已配置后执行：

```bash
source .env
npm run build
```

构建成功即表示签名配置正确。

---

## 6. 远程升级测试

参考 [README → Release](../README.md#release) 发布一个测试版本，验证更新检测与升级流程是否正常：

1. 在 `CHANGELOG.md` 的 `[Unreleased]` 部分填写测试更新内容
2. 发布新版本：
   ```bash
   npm run release <新版本号>
   ```
3. 在已安装**旧版本**应用的设备上触发更新检查，确认：
   - 能检测到新版本
   - 点击升级后下载并安装成功
   - 安装后版本号更新正确

---

## 注意事项

- 私钥丢失后，**已安装旧版本的用户将永远无法收到更新**，请妥善备份 `~/.tauri/you-skill.key`
- 公钥一旦写入 `tauri.conf.json` 并发布，对应的私钥就必须长期保留
- 如需重新生成密钥对，所有已安装旧版本的用户需手动重新安装应用
