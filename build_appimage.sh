#!/bin/bash
set -e

echo "BUILDING OPENNAV APPIMAGE..."

# 0. Clean previous artifacts
rm -rf AppDir
rm -f OpenNav*.AppImage

# 1. Build Release Binary
echo "[1/4] Compiling Rust binary..."
~/.cargo/bin/cargo build --release

# 2. Download linuxdeploy if not present
if [ ! -f linuxdeploy-x86_64.AppImage ]; then
    echo "[2/4] Downloading linuxdeploy..."
    wget -q https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
    chmod +x linuxdeploy-x86_64.AppImage
fi

# 3. Prepare Environment
echo "[3/4] Preparing resources..."
# We need a desktop file without absolute paths for the AppImage
cp opennav.desktop opennav-appimage.desktop
sed -i 's|^Exec=.*|Exec=opennav %u|' opennav-appimage.desktop
sed -i 's|^Icon=.*|Icon=opennav|' opennav-appimage.desktop

# 4. Generate AppImage
echo "[4/4] Generating AppImage..."

# We set NO_STRIP to avoid issues with some libraries, though not strictly necessary
export NO_STRIP=true

./linuxdeploy-x86_64.AppImage \
  --appdir AppDir \
  --executable target/release/opennav \
  --desktop-file opennav-appimage.desktop \
  --icon-file resources/opennav.png \
  --output appimage

# Cleanup
rm opennav-appimage.desktop
rm -rf AppDir

echo "SUCCESS! AppImage created:"
ls -lh OpenNav*.AppImage
