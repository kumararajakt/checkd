#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD_DIR="build-appimage"
APP_DIR="${BUILD_DIR}/AppDir"
CACHE_DIR="${BUILD_DIR}/tools"

echo "==> Preparing build directories..."
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}" "${CACHE_DIR}"

echo "==> Compiling application for AppDir..."
cmake -B "${BUILD_DIR}/cmake" -G Ninja \
  -DCMAKE_BUILD_TYPE=RelWithDebInfo \
  -DCMAKE_INSTALL_PREFIX=/usr \
  -DBUILD_TESTING=OFF

cmake --build "${BUILD_DIR}/cmake"
DESTDIR="$(pwd)/${APP_DIR}" cmake --install "${BUILD_DIR}/cmake"

echo "==> Copying Kirigami & KDE QML plugins..."
QT_QML_DIR="$(qtpaths6 --locate-dirs GenericDataLocation qml 2>/dev/null || echo "/usr/lib/qt6/qml")"
if [ ! -d "${QT_QML_DIR}" ]; then
  QT_QML_DIR="/usr/lib/qt6/qml"
fi

mkdir -p "${APP_DIR}/usr/lib/qt6/qml/org/kde"
for mod in kirigami kirigamiaddons qqc2desktopstyle breeze desktop coreaddons i18n iconthemes; do
  if [ -d "${QT_QML_DIR}/org/kde/${mod}" ]; then
    cp -rn "${QT_QML_DIR}/org/kde/${mod}" "${APP_DIR}/usr/lib/qt6/qml/org/kde/" 2>/dev/null || true
  fi
done

mkdir -p "${APP_DIR}/usr/lib/qt6/plugins/styles"
QT_PLUGINS_DIR="$(qtpaths6 --locate-dirs GenericDataLocation plugins 2>/dev/null || echo "/usr/lib/qt6/plugins")"
cp -rn "${QT_PLUGINS_DIR}/styles/"* "${APP_DIR}/usr/lib/qt6/plugins/styles/" 2>/dev/null || true

echo "==> Downloading linuxdeploy tools if needed..."
pushd "${CACHE_DIR}" > /dev/null

if [ ! -d "linuxdeploy" ]; then
  echo "Downloading linuxdeploy..."
  curl -sLo linuxdeploy.AppImage https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
  chmod +x linuxdeploy.AppImage
  ./linuxdeploy.AppImage --appimage-extract > /dev/null
  mv squashfs-root linuxdeploy
fi

if [ ! -d "linuxdeploy-plugin-qt" ]; then
  echo "Downloading linuxdeploy-plugin-qt..."
  curl -sLo linuxdeploy-plugin-qt.AppImage https://github.com/linuxdeploy/linuxdeploy-plugin-qt/releases/download/continuous/linuxdeploy-plugin-qt-x86_64.AppImage
  chmod +x linuxdeploy-plugin-qt.AppImage
  ./linuxdeploy-plugin-qt.AppImage --appimage-extract > /dev/null
  mv squashfs-root linuxdeploy-plugin-qt
fi

if [ ! -d "linuxdeploy-plugin-appimage" ]; then
  echo "Downloading linuxdeploy-plugin-appimage..."
  curl -sLo linuxdeploy-plugin-appimage.AppImage https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-x86_64.AppImage
  chmod +x linuxdeploy-plugin-appimage.AppImage
  ./linuxdeploy-plugin-appimage.AppImage --appimage-extract > /dev/null
  mv squashfs-root linuxdeploy-plugin-appimage
fi
popd > /dev/null

echo "==> Preparing clean Qt plugins directory..."
CLEAN_PLUGINS="$(pwd)/${BUILD_DIR}/qt-plugins"
rm -rf "${CLEAN_PLUGINS}"
mkdir -p "${CLEAN_PLUGINS}"
cp -rs /usr/lib/qt6/plugins/* "${CLEAN_PLUGINS}/" 2>/dev/null || true

for plugin in "${CLEAN_PLUGINS}"/*/*.so; do
  if [ -L "$plugin" ] || [ -f "$plugin" ]; then
    if ldd "$plugin" 2>&1 | grep -q "not found"; then
      echo "  Excluding broken/orphaned plugin: $(basename "$plugin")"
      rm -f "$plugin"
    fi
  fi
done

cat << EOF > "${CACHE_DIR}/qmake"
#!/bin/bash
if [ "\$1" = "-query" ]; then
  if [ -z "\${2:-}" ]; then
    /usr/bin/qmake6 -query | sed "s|^QT_INSTALL_PLUGINS:.*|QT_INSTALL_PLUGINS:${CLEAN_PLUGINS}|"
    exit 0
  elif [ "\$2" = "QT_INSTALL_PLUGINS" ]; then
    echo "${CLEAN_PLUGINS}"
    exit 0
  fi
fi
exec /usr/bin/qmake6 "\$@"
EOF
chmod +x "${CACHE_DIR}/qmake"
ln -sf qmake "${CACHE_DIR}/qmake6"

export QMAKE="$(pwd)/${CACHE_DIR}/qmake"
export PATH="$(pwd)/${CACHE_DIR}:$(pwd)/${CACHE_DIR}/linuxdeploy/usr/bin:$(pwd)/${CACHE_DIR}/linuxdeploy-plugin-qt/usr/bin:$(pwd)/${CACHE_DIR}/linuxdeploy-plugin-appimage/usr/bin:${PATH}"
export APPIMAGE_EXTRACT_AND_RUN=1
export NO_STRIP=1
export DEPLOY_PLATFORM_THEMES=1
export EXTRA_PLATFORM_PLUGINS="libqwayland.so;libqoffscreen.so"
export QML_SOURCES_PATHS="$(pwd)/src/qml"
export EXTRA_QT_MODULES="core;gui;quick;widgets;sql;svg;quickcontrols2;network"

cat << 'EOF' > "${BUILD_DIR}/AppRun"
#!/bin/bash
HERE="$(dirname "$(readlink -f "${0}")")"
export APPDIR="${HERE}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${HERE}/usr/lib/qt6:${LD_LIBRARY_PATH:-}"
export QT_PLUGIN_PATH="${HERE}/usr/plugins:${QT_PLUGIN_PATH:-}"
export QML_IMPORT_PATH="${HERE}/usr/qml:${HERE}/usr/lib/qt6/qml:${QML_IMPORT_PATH:-}"
export QML2_IMPORT_PATH="${HERE}/usr/qml:${HERE}/usr/lib/qt6/qml:${QML2_IMPORT_PATH:-}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
export QT_QUICK_CONTROLS_STYLE="${QT_QUICK_CONTROLS_STYLE:-org.kde.desktop}"
exec "${HERE}/usr/bin/checkd" "$@"
EOF
chmod +x "${BUILD_DIR}/AppRun"

echo "==> Packaging AppImage with linuxdeploy..."
"${CACHE_DIR}/linuxdeploy/AppRun" \
  --appdir "${APP_DIR}" \
  --executable "${APP_DIR}/usr/bin/checkd" \
  --desktop-file "${APP_DIR}/usr/share/applications/com.kumar.checkd.desktop" \
  --icon-file "${APP_DIR}/usr/share/icons/hicolor/scalable/apps/com.kumar.checkd.svg" \
  --custom-apprun "${BUILD_DIR}/AppRun" \
  --plugin qt \
  --output appimage

if [ -f "CheckD-x86_64.AppImage" ]; then
  echo ""
  echo "==> Done! AppImage created at: ./CheckD-x86_64.AppImage"
  echo "    You can run it directly with:"
  echo "    ./CheckD-x86_64.AppImage"
elif [ -n "${OUTPUT_APPIMAGE:-}" ]; then
  mv "${OUTPUT_APPIMAGE}" CheckD-x86_64.AppImage
  echo ""
  echo "==> Done! AppImage created at: ./CheckD-x86_64.AppImage"
  echo "    You can run it directly with:"
  echo "    ./CheckD-x86_64.AppImage"
else
  echo "==> Error: AppImage was not produced."
  exit 1
fi
