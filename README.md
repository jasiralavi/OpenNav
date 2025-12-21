# OpenNav

**OpenNav** is a lightweight, keyboard-centric browser selector/launcher written in Rust using GTK4. 

It automatically detects your installed browsers and lets you quickly select which one to open a link with. It learns your preferences over time, sorting your most-used browsers to the top.

![OpenNav Screenshot](resources/opennav.png)

## Features

-   🚀 **Fast Startup**: Optimized for instant launch.
-   ⌨️ **Keyboard Driven**: Navigate, filter, and launch without touching the mouse.
-   🔍 **Smart Filtering**: Type to filter browsers instantly.
-   📌 **Pinning**: Keep your favorite browsers pinned to the top (`Ctrl+P`).
-   📊 **Usage Sorting**: Automatically sorts browsers by usage frequency.
-   🛠️ **Customizable**: Supports custom icons (including absolute paths) and recognizes Flatpaks.
-   🌑 **Modern UI**: Dark theme support with readable styling.

## Installation

### From Source (Rust)

1.  **Dependencies**:
    -   Fedora: `sudo dnf install gtk4-devel gcc`
    -   Ubuntu/Debian: `sudo apt install libgtk-4-dev build-essential`

2.  **Build & Run**:
    ```bash
    git clone https://github.com/yourusername/opennav.git
    cd opennav
    cargo run --release -- https://google.com
    ```

3.  **Install Desktop Entry** (Required for icons/integration):
    ```bash
    # Update paths in opennav.desktop if needed, then:
    cp opennav.desktop ~/.local/share/applications/com.opennav.app.desktop
    update-desktop-database ~/.local/share/applications/
    ```

### Flatpak (Local Build)

See [distribution.md](distribution.md) or `flatpak/` folder for instructions on building a local Flatpak.

## Shortcuts

| Key | Action |
| :--- | :--- |
| **Type** | Filter list |
| **Enter** | Launch selected |
| **Ctrl + Enter** | Launch & Keep Open |
| **Ctrl + P** | Pin/Unpin Browser |
| **Ctrl + S** | Open Settings |
| **Ctrl + ?** | Show Shortcuts |
| **Esc** | Close / Clear Search |

## License

MIT
