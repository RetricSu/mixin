#!/bin/sh
set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/512x512/apps"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

# Install binary
cp "$DIR/waxseal" "$BIN_DIR/waxseal"
chmod +x "$BIN_DIR/waxseal"

# Install icon
if [ -f "$DIR/icon.png" ]; then
    cp "$DIR/icon.png" "$ICON_DIR/waxseal.png"
fi

# Install desktop entry
if [ -f "$DIR/waxseal.desktop" ]; then
    sed -e "s|Exec=waxseal|Exec=$BIN_DIR/waxseal|" "$DIR/waxseal.desktop" > "$APP_DIR/waxseal.desktop"
    chmod +x "$APP_DIR/waxseal.desktop"
fi

echo "========================================"
echo "火漆 (Waxseal) 安装完成！"
echo "已安装到: $BIN_DIR/waxseal"
echo "桌面启动项: $APP_DIR/waxseal.desktop"
echo "直接在终端输入 waxseal 或在应用菜单中点击即可打开。"
echo "========================================"
