# Distributing OpenNav as a Flatpak

## Prerequisites
You need `flatpak` and `flatpak-builder` installed.

## The Challenge: Sandboxing
Since OpenNav is a **Launcher**, it needs to:
1.  Read `.desktop` files from the **Host** system (not just the sandbox).
2.  Launch processes on the **Host** system.

The provided manifest grants:
-   Read-only access to `/usr/share/applications` and `~/.local/share/applications`.
-   Access to `org.freedesktop.Flatpak` to allow launching apps via `flatpak-spawn --host` (requires code changes to use `flatpak-spawn` if running inside container).

## Building Locally
1.  **Install Runtime**:
    ```bash
    flatpak install org.gnome.Platform//45 org.gnome.Sdk//45
    ```

2.  **Build**:
    ```bash
    # Note: --share=network is needed because we aren't pre-vendoring cargo crates in this simple manifest
    flatpak-builder --user --install --force-clean --share=network build-dir flatpak/com.opennav.app.yml
    ```

3.  **Run**:
    ```bash
    flatpak run com.opennav.app
    ```

## Publishing (Flathub)
To publish on Flathub, you cannot use `--share=network`. You must vendor your Rust dependencies.
Use [flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo) to modify the manifest:
```bash
python3 flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json
```
Then include `cargo-sources.json` in your YML.

---

# Alternative Distribution Methods

## 1. AppImage (Recommended for Portability)
AppImages are single-file, runnable executables. They are great for launchers because they are **not sandboxed** by default, meaning OpenNav can easily see valid system browsers.
-   **Tool**: `linuxdeploy` (can bundle GTK resources).
-   **Pros**: Runs on almost any distro, no installation, accesses host system easily.
-   **Cons**: No automatic updates (unless using AppImageUpdate).

## 2. Native Packages (.deb / .rpm)
Best for system integration (apt/dnf install).
-   **Cargo Tools**:
    -   `cargo-deb`: `cargo install cargo-deb && cargo deb` (Generates .deb)
    -   `cargo-generate-rpm`: `cargo install cargo-generate-rpm && cargo generate-rpm` (Generates .rpm)
-   **Pros**: Native system integration, dependency management.
-   **Cons**: Distro-specific.

## 3. GitHub Releases (Binary)
The simplest way to share with minimal effort.
1.  Build a release binary: `cargo build --release`.
2.  Zip the binary (`target/release/opennav`), the icon (`resources/opennav.png`), and the `.desktop` file.
3.  Upload to GitHub Releases.
4.  **User Instructions**: "Extract zip, run `install.sh` (which you write to copy files to `~/.local`)."

## 4. Cargo Install (For Rust Users)
If your audience uses Rust:
1.  Publish to crates.io or point to your git repo.
2.  Users run: `cargo install --git https://github.com/your/repo`.
3.  **Note**: Users still need to manually install the`.desktop` file for the icon to appear.
