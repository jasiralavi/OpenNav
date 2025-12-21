# OpenNav

**OpenNav** is a lightweight, keyboard-centric browser selector/launcher written in Rust using GTK4. 

It automatically detects your installed browsers and lets you quickly select which one to open a link with. It learns your preferences over time, sorting your most-used browsers to the top.

<div align="center">
  <img src="resources/opennav.png" width="200" alt="OpenNav Icon" />
</div>
<br>
<div align="center">
  <img src="resources/screenshot_1.png" width="75%" alt="OpenNav Screenshot 1" />
</div>
<div align="center">
  <img src="resources/screenshot_2.png" width="75%" alt="OpenNav Screenshot 2" />
</div>

## Features

-   🚀 **Fast Startup**: Optimized for instant launch.
-   ⌨️ **Keyboard Driven**: Navigate, filter, and launch without touching the mouse.
-   🔍 **Smart Filtering**: Type to filter browsers instantly.
-   📌 **Pinning**: Keep your favorite browsers pinned to the top (`Ctrl+P`).
-   📊 **Usage Sorting**: Automatically sorts browsers by usage frequency.
-   🛠️ **Customizable**: Supports custom icons (including absolute paths) and recognizes Flatpaks.
-   🌑 **Modern UI**: Dark theme support with readable styling.

## Installation

### AppImage (Portable)

Download the `.AppImage` from the [Releases](https://github.com/jasiralavi/opennav/releases) page.
```bash
chmod +x OpenNav-x86_64.AppImage
./OpenNav-x86_64.AppImage
```

### From Source (Rust)

1.  **Dependencies**:
    -   Fedora: `sudo dnf install gtk4-devel gcc`
    -   Ubuntu/Debian: `sudo apt install libgtk-4-dev build-essential`

2.  **Build & Run**:
    ```bash
    git clone https://github.com/jasiralavi/opennav.git
    cd opennav
    cargo run --release -- https://cyfersolutions.com
    ```

3.  **Install Desktop Entry** (Required for icons/integration):
    ```bash
    # Update paths in opennav.desktop if needed, then:
    cp opennav.desktop ~/.local/share/applications/com.opennav.app.desktop
    update-desktop-database ~/.local/share/applications/
    ```

### Flatpak (Local Build)

See [distribution.md](distribution.md) or `flatpak/` folder for instructions on building a local Flatpak.

## How to Use

OpenNav serves two main purposes:

1.  **Browser Picker**: When you click a link in another app (like Discord or Slack), OpenNav pops up, letting you choose which browser to open that specific link in.
2.  **Quick Launcher**: You can launch OpenNav directly to quickly open any of your installed browsers, optionally typing a URL to go straight there.
3.  **Direct Web Search**: Type a search query (e.g., "rust lang") in the URL bar and select a browser. OpenNav will automatically detect it's a search term and perform a web search using the browser's default search engine (or Google).

### Configuration

#### Set as Default Browser
To make OpenNav your browser picker, you must set it as your system's **Default Web Browser**:
-   **GNOME**: Settings -> Default Applications -> Web. Select "OpenNav".
-   **KDE Plasma**: System Settings -> Applications -> Default Applications -> Web Browser. Select "OpenNav".

#### Quick Launch Shortcut
To open OpenNav instantly with a keyboard shortcut (e.g., `Super+B` or `Ctrl+Alt+B`):
1.  Open your desktop environment's **Keyboard Shortcuts** settings.
2.  Add a new custom shortcut.
3.  Set the command to: `opennav` (or the path to your AppImage if using that).
4.  Assign your preferred key combination.

## Shortcuts

| Key | Action |
| :--- | :--- |
| **Type** | Filter list |
| **Ctrl + L** | Focus URL Bar |
| **Up / Down** | Navigation |
| **Enter / Click** | Launch Selected |
| **Ctrl + Enter** | Launch & Keep Open |
| **Ctrl + Click** | Launch & Keep Open |
| **Ctrl + P** | Pin/Unpin Browser |
| **Ctrl + S** | Open Settings |
| **Ctrl + ?** | Show Shortcuts |
| **Esc** | Close / Clear Search |



## License

GNU General Public License v3.0
