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
- `~/.config/dev.iperez.chatgpt-wrapper/settings.json` (UI preferences)
- `~/.local/share/dev.iperez.chatgpt-wrapper/webview-cache` (session storage)

Remove those folders to reset the app.

## Linux Installation Script

To install ChatGPT Desktop under `~/.local`, run:

```bash
./scripts/install-linux.sh
```

The script simply builds the optimized Tauri binary and installs:
- `~/.local/opt/chatgpt-wrapper/chatgpt-wrapper` (release binary)
- `~/.local/opt/chatgpt-wrapper/chatgpt-desktop` (launcher script)
- `~/.local/share/applications/chatgpt-wrapper.desktop`
- `~/.local/share/icons/hicolor/128x128/apps/chatgpt-wrapper.png`

No AppImage or linuxdeploy download is required, so it works fully offline. After it completes, launch **ChatGPT Desktop** from your application menu or via the launcher path.

## Uninstall

```bash
rm -f ~/.local/share/applications/chatgpt-wrapper.desktop
rm -rf ~/.local/opt/chatgpt-wrapper
rm -f ~/.local/share/icons/hicolor/128x128/apps/chatgpt-wrapper.png
update-desktop-database ~/.local/share/applications
```

Optionally delete the config/cache folders mentioned earlier.

## License

Released under the [MIT License](LICENSE).
