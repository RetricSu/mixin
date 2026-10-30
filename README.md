# 密信 (Mixin)

一个给普通人用的桌面端对端安全传递工具。打开后只需选择“发送”或“接收”，不必理解复杂密码学术语。

## 产品特色

- **现代桌面设计**：精心打造的现代化桌面端界面，提供沉浸式暗色主题与清晰明亮浅色主题，支持状态记忆。
- **发送文字与任意文件**：不仅支持文字密码与敏感信息，还支持任意文件格式（文档、图片、表格、压缩包等）的高强度封装加密。
- **拖拽交互 (Drag & Drop)**：支持直接把待发送文件拖入窗口；收到 `.mixin` 密信文件也可直接拖入解密。
- **双重安全加密模式**：
  - **约定口令 (AES-256-GCM)**：双方约定一个密码，适合临时、点对点沟通。内置随机安全口令生成器（如 `golden-falcon-7281`）。
  - **联系人专属 (RSA-OAEP-2048 + AES-256)**：基于公钥加密会话密钥，仅对方本机持有的私钥能解开，无需每次约定口令。
- **智能接收识别**：粘贴或打开密信时，系统自动识别加密方式（口令加密 or 专属加密），一键解密并提供完整预览。
- **安全身份与联系人簿**：
  - 自动生成 RSA-2048 本机密钥对，私钥保存在操作系统数据目录并受权限保护。
  - 公开身份卡附带可读性强的安全指纹（Fingerprint，如 `32B0·37CD·0127·6295`）与高清晰度二维码，支持一键导出 PNG。
  - 内置常用联系人簿，保存好友与同事的公钥，发信时一键指定，告别繁琐的密钥复制。
  - 支持本机身份私钥安全备份与恢复导入。

## 安全架构

- **口令路径**：使用 AES-256-GCM，密码通过 SHA-256 派生 32 字节密钥，每次加密生成全新随机 12 字节 Nonce，具备完整的认证加密与防篡改特性。
- **联系人路径**：为每次传输生成 32 字节高强度随机对称密钥和 12 字节 Nonce，通过 RSA-OAEP-SHA256 包裹对称密钥，正文使用 AES-256-GCM 进行认证加密。不受 RSA 载荷大小限制，能安全传递大载荷与二进制文件。
- **文件存储权限**：私钥文件仅写入应用数据目录，在 Unix 系统上强制设定为 `0o600`（仅当前用户可读写）。

## 下载与安装

密信提供跨平台官方编译安装包（支持 macOS、Windows、Linux 三大主流平台）：

### 🍎 macOS
- 前往 [Releases](https://github.com/RetricSu/mixin/releases) 页面下载对应芯片的安装包：
  - Apple Silicon (M1/M2/M3/M4): `Mixin-aarch64-apple-darwin.dmg` 或 `.zip`
  - Intel 处理器: `Mixin-x86_64-apple-darwin.dmg` 或 `.zip`
- 打开 `.dmg` 文件后，直接将 **Mixin.app** 拖拽入 **Applications (应用程序)** 目录即可使用。

### 🪟 Windows
- 前往 [Releases](https://github.com/RetricSu/mixin/releases) 下载 `Mixin-x86_64-pc-windows-msvc.zip`。
- 解压后双击 `双击我.bat` 或直接运行 `mixin.exe`。

### 🐧 Linux
- 前往 [Releases](https://github.com/RetricSu/mixin/releases) 下载 `Mixin-x86_64-unknown-linux-gnu.tar.gz` 或 `.deb` 包。
- Debian / Ubuntu 用户可直接安装 deb 包：
  ```sh
  sudo dpkg -i mixin_*.deb
  ```
- 或解压 tar.gz 运行一键安装脚本：
  ```sh
  tar -zxvf Mixin-x86_64-unknown-linux-gnu.tar.gz
  cd Mixin-x86_64-unknown-linux-gnu
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
