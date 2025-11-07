#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_BIN="${PROJECT_ROOT}/src-tauri/target/release/chatgpt-desktop"
INSTALL_DIR="${HOME}/.local/opt/chatgpt-desktop"
LAUNCHER="${INSTALL_DIR}/chatgpt-desktop"
DESKTOP_FILE="${HOME}/.local/share/applications/chatgpt-desktop.desktop"
ICON_BASE_DIR="${HOME}/.local/share/icons/hicolor"
ICONS_SOURCE_DIR="${PROJECT_ROOT}/src-tauri/icons"

printf '→ Building ChatGPT Desktop (release binary)...\n'
cargo build --release --manifest-path "${PROJECT_ROOT}/src-tauri/Cargo.toml"

if pgrep -x "chatgpt-desktop" > /dev/null; then
    printf '\n⚠ ChatGPT Desktop is currently running.\n'
    printf 'Please close the application before installing.\n'
    printf '\nTo kill the process, run:\n'
    printf '  pkill chatgpt-desktop\n\n'
    exit 1
fi

mkdir -p "${INSTALL_DIR}"
mkdir -p "${INSTALL_DIR}/icons"
cp "${TARGET_BIN}" "${INSTALL_DIR}/chatgpt-desktop"
chmod +x "${INSTALL_DIR}/chatgpt-desktop"

# Copy icons to the installation directory for runtime access
cp "${ICONS_SOURCE_DIR}/icon-light-32x32.png" "${INSTALL_DIR}/icons/"
cp "${ICONS_SOURCE_DIR}/32x32.png" "${INSTALL_DIR}/icons/"

cat > "${LAUNCHER}" <<'LAUNCH'
#!/usr/bin/env bash
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${DIR}/chatgpt-desktop" "$@"
LAUNCH
chmod +x "${LAUNCHER}"

printf '→ Installing icons...\n'
mkdir -p "${ICON_BASE_DIR}/32x32/apps"
mkdir -p "${ICON_BASE_DIR}/128x128/apps"
mkdir -p "${ICON_BASE_DIR}/256x256/apps"
mkdir -p "${ICON_BASE_DIR}/scalable/apps"

cp "${ICONS_SOURCE_DIR}/32x32.png" "${ICON_BASE_DIR}/32x32/apps/chatgpt-desktop.png"
cp "${ICONS_SOURCE_DIR}/128x128.png" "${ICON_BASE_DIR}/128x128/apps/chatgpt-desktop.png"
cp "${ICONS_SOURCE_DIR}/icon.png" "${ICON_BASE_DIR}/256x256/apps/chatgpt-desktop.png"

mkdir -p "${ICON_BASE_DIR}/32x32/apps"
cp "${ICONS_SOURCE_DIR}/icon-light-32x32.png" "${ICON_BASE_DIR}/32x32/apps/chatgpt-desktop-tray-light.png"

gtk-update-icon-cache -f -t "${ICON_BASE_DIR}" >/dev/null 2>&1 || true

cat > "${DESKTOP_FILE}" <<DESKTOP
[Desktop Entry]
Type=Application
Name=ChatGPT Desktop
Comment=Minimal desktop wrapper for chat.openai.com
Exec=${LAUNCHER}
Icon=chatgpt-desktop
Terminal=false
Categories=Utility;Network;
StartupNotify=true
DESKTOP

update-desktop-database "${HOME}/.local/share/applications" >/dev/null 2>&1 || true

printf '\n✔ ChatGPT Desktop installed locally.\n'
printf '  • Binary: %s\n' "${INSTALL_DIR}/chatgpt-desktop"
printf '  • Launcher: %s\n' "${LAUNCHER}"
printf '  • Desktop entry: %s\n' "${DESKTOP_FILE}"
printf '  • Icons installed in: %s\n' "${ICON_BASE_DIR}"
printf '    - 32x32, 128x128, 256x256 sizes\n'
printf '    - Tray icon variants included\n'
