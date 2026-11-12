# 火漆 (Waxseal)

一个给普通人用的桌面端对端安全传递工具。打开后只需选择“发送”或“接收”，不必理解复杂密码学术语。

## 产品特色

- **现代桌面设计**：精心打造的现代化桌面端界面，提供沉浸式暗色主题与清晰明亮浅色主题，支持状态记忆。
- **发送文字与任意文件**：不仅支持文字密码与敏感信息，还支持任意文件格式（文档、图片、表格、压缩包等）的高强度封装加密。
- **拖拽交互 (Drag & Drop)**：支持直接把待发送文件拖入窗口；收到 `.seal`（并兼容旧版 `.mixin`）密信文件也可直接拖入解密。
- **双重安全加密模式**：
  - **约定口令 (AES-256-GCM)**：双方约定一个密码，适合临时、点对点沟通。内置随机安全口令生成器（如 `golden-falcon-7281`）。
  - **联系人专属 (X25519 + Ed25519 + AES-256-GCM)**：基于现代椭圆曲线 X25519 临时密钥交换并派生 AES-256 会话密钥，结合 Ed25519 发件人数字签名，确保仅对方能解密且真实发件人身份无法冒充与篡改，无需每次约定口令。
- **智能接收识别**：粘贴或打开密信时，系统自动识别加密方式（口令密信 `MX1.` 或专属密信 `MXID2.`），一键解密并自动验证发件人签名，展示发件人身份。
- **安全身份与联系人簿**：
  - 自动生成 X25519 密钥交换与 Ed25519 签名本机身份，私钥保存在操作系统数据目录并受权限保护。
  - 公开身份卡附带可读性强的安全指纹（Fingerprint，如 `32B0·37CD·0127·6295`）与轻量高清二维码（公钥由数百字节压缩至 64 字节，大幅提升扫码速度与识别率），支持一键导出 PNG。
  - 内置常用联系人簿，保存好友与同事的公开身份，发信时一键指定，解密时自动匹配联系人姓名并确认签名有效。
  - 支持本机身份私钥安全备份与恢复导入。

## 安全架构

- **口令路径**：使用 AES-256-GCM，口令经强内存硬化算法 Argon2id（内存 19 MiB、2 轮迭代、单线程并发，符合 RFC 9106 与 OWASP 推荐标准）与每次全新生成的 16 字节随机 Salt 派生 32 字节高强度密钥，结合随机 12 字节 Nonce 进行认证加密，具备高强度的抗 GPU/ASIC 离线暴力破解能力与防篡改特性。密文封装格式为 `MX1.` + base64(salt[16] || nonce[12] || ciphertext || tag[16])。
- **联系人路径**：为每次传输生成全新随机临时 X25519 密钥对，与接收方 X25519 公钥完成 Diffie-Hellman 密钥交换，通过 HKDF-SHA256 派生 32 字节高强度会话密钥；正文使用 AES-256-GCM 进行认证加密；发件人使用 Ed25519 强签名绑定协议域标签、接收方公钥、发件人公钥、临时公钥、IV 及密文。密文封装格式为 `MXID2.` + base64(sender_public_key[64] || ephemeral_x25519_pk[32] || nonce[12] || signature[64] || ciphertext || tag[16])。具备前向保密、防重放伪造、发件人真实性保证与抗篡改特性。
- **文件存储权限**：私钥文件仅写入应用数据目录，在 Unix 系统上强制设定为 `0o600`（仅当前用户可读写）。

## 下载与安装

火漆提供跨平台官方编译安装包（支持 macOS、Windows、Linux 三大主流平台）：

### 🍎 macOS
- 前往 [Releases](https://github.com/RetricSu/waxseal/releases) 页面下载对应芯片的安装包：
  - Apple Silicon (M1/M2/M3/M4): `Waxseal-aarch64-apple-darwin.dmg` 或 `.zip`
  - Intel 处理器: `Waxseal-x86_64-apple-darwin.dmg` 或 `.zip`
- 打开 `.dmg` 文件后，直接将 **Waxseal.app** 拖拽入 **Applications (应用程序)** 目录即可使用。

### 🪟 Windows
- 前往 [Releases](https://github.com/RetricSu/waxseal/releases) 下载 `Waxseal-x86_64-pc-windows-msvc.zip`。
- 解压后双击 `双击我.bat` 或直接运行 `waxseal.exe`（或 `火漆.exe`）。

### 🐧 Linux
- 前往 [Releases](https://github.com/RetricSu/waxseal/releases) 下载 `Waxseal-x86_64-unknown-linux-gnu.tar.gz` 或 `.deb` 包。
- Debian / Ubuntu 用户可直接安装 deb 包：
  ```sh
  sudo dpkg -i waxseal_*.deb
  ```
- 或解压 tar.gz 运行一键安装脚本：
  ```sh
  tar -zxvf Waxseal-x86_64-unknown-linux-gnu.tar.gz
  cd Waxseal-x86_64-unknown-linux-gnu
  ./install.sh
  ```

## 编译与测试

```sh
# 运行单元测试
cargo test

# 构建本地版本
cargo build --release

# 打包多平台二进制（需对应 toolchain）
./build.sh
```
