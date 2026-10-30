#!/bin/sh
set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/512x512/apps"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

# Install binary
cp "$DIR/mixin" "$BIN_DIR/mixin"
chmod +x "$BIN_DIR/mixin"

# Install icon
if [ -f "$DIR/icon.png" ]; then
    cp "$DIR/icon.png" "$ICON_DIR/mixin.png"
fi

# Install desktop entry
if [ -f "$DIR/mixin.desktop" ]; then
    sed -e "s|Exec=mixin|Exec=$BIN_DIR/mixin|" "$DIR/mixin.desktop" > "$APP_DIR/mixin.desktop"
    chmod +x "$APP_DIR/mixin.desktop"
fi

echo "========================================"
echo "密信 (Mixin) 安装完成！"
echo "已安装到: $BIN_DIR/mixin"
echo "桌面启动项: $APP_DIR/mixin.desktop"
echo "直接在终端输入 mixin 或在应用菜单中点击即可打开。"
echo "========================================"
