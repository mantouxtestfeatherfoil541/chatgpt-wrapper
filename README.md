# ChatGPT Desktop Wrapper

A minimal Tauri-based Linux desktop wrapper for [chat.openai.com](https://chat.openai.com). It keeps your session data locally, remembers your window-decoration preference, and opens external links in the system browser.

> **Disclaimer**: ChatGPT, the ChatGPT logo, and other OpenAI trademarks are the property of OpenAI. This project is an unofficial community wrapper and is not affiliated with or endorsed by OpenAI.

## Requirements

- Rust toolchain (stable) and `cargo`
- Tauri CLI (`cargo install tauri-cli --version "^2.0.0" --locked`)
- Linux desktop dependencies (Debian/Ubuntu example):
    ```bash
    # Debian/Ubuntu
    sudo apt install build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.0-dev \
       libayatana-appindicator3-dev librsvg2-dev

    # Archlinux
    sudo pacman -S --needed \
      webkit2gtk-4.1 \
      base-devel \
      curl \
      wget \
      file \
      openssl \
      appmenu-gtk-module \
      libappindicator-gtk3 \
      librsvg \
      xdotool
    ```

## Development

```bash
cargo tauri dev
```

Session/config data lives in:
- `~/.config/dev.iperez.chatgpt-desktop/settings.json` (preferences)
- `~/.local/share/dev.iperez.chatgpt-desktop/webview-cache` (session storage)

Remove those folders to reset the app.

## Linux Installation Script

To install ChatGPT Desktop under `~/.local`, run:

```bash
./scripts/install-linux.sh
```

The script simply builds the optimized Tauri binary and installs:
- `~/.local/opt/chatgpt-desktop/chatgpt-desktop` (release binary)
- `~/.local/opt/chatgpt-desktop/chatgpt-desktop` (launcher script)
- `~/.local/opt/chatgpt-desktop/icons/` (runtime icons for tray)
- `~/.local/share/applications/chatgpt-desktop.desktop`
- `~/.local/share/icons/hicolor/*/apps/chatgpt-desktop*.png` (system icons in multiple sizes)

No AppImage or linuxdeploy download is required, so it works fully offline. After it completes, launch **ChatGPT Desktop** from your application menu or via the launcher path.

## Uninstall

```bash
# Remove application files
rm -f ~/.local/share/applications/chatgpt-desktop.desktop
rm -rf ~/.local/opt/chatgpt-desktop

# Remove icons
rm -f ~/.local/share/icons/hicolor/32x32/apps/chatgpt-desktop.png
rm -f ~/.local/share/icons/hicolor/32x32/apps/chatgpt-desktop-tray-light.png
rm -f ~/.local/share/icons/hicolor/128x128/apps/chatgpt-desktop.png
rm -f ~/.local/share/icons/hicolor/256x256/apps/chatgpt-desktop.png

# Update databases
update-desktop-database ~/.local/share/applications
gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor
```

Optionally delete the config/cache folders:
```bash
rm -rf ~/.config/dev.iperez.chatgpt-desktop
rm -rf ~/.cache/dev.iperez.chatgpt-desktop
```

## Settings

You can edit the settings file located at `~/.config/dev.iperez.chatgpt-desktop/settings.json`.

### Available Options

```json
{
  "notifications_enabled": true,  // Enable/disable desktop notifications
  "hide_decorations": false,      // Hide/show GTK window decorations (title bar)
  "show_tray": true,              // Show/hide system tray icon
  "close_to_tray": false,         // Minimize to tray instead of closing the app
  "tray_icon_light": false        // Use light icon for dark themes
}
```

**Note:** Changes to settings require restarting the application to take effect.

## License

Released under the [MIT License](LICENSE).
