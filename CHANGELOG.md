# Changelog

## [1.2.0] - 2025-12-23

### Added
- **Dynamic URL Bar Icons**: The URL bar now displays context-aware icons:
    - A **Globe icon** (custom wireframe style) for valid URLs and when the field is empty.
    - **Search Engine Brand Icons** (e.g., Google, YouTube) when entering search queries or using keywords (e.g., "yt search").
    - Falls back to the Default Search Engine's icon for generic queries.
- **Browser Sorting**: Added a "Browser List Order" dropdown in Settings with options:
    - **Alphabetical**: Sort browsers by name.
    - **Recently Used**: Sort by most recently launched.
    - **Frequently Used**: Sort by usage count.
- **Reset Stats**: Added context-sensitive "Reset" buttons in Settings to clear usage statistics for "Recently Used" and "Frequently Used" sorting modes.
- **AppImage Support**: Added build configuration for generating AppImage bundle.

### Changed
- **UI Polish**:
    - Unified styling for "Add", "Reset", and "Close" buttons to use a consistent blue accent (`suggested-action`) and fixed width.
    - Updated URL bar placeholder text.
    - Improved responsiveness of the Settings dialog.
- **Icon Handling**:
    - Improved logic to support custom branding icons downloaded from the web (favicons) in the URL bar.
    - Fixed an issue where the "network-server" icon was used incorrectly for the globe.

### Fixed
- **Release Builds**: Fixed resource loading path for release binaries and AppImages to correctly locate icons and stylesheets.
- **Search Engine Icons**: Resolved a bug where custom `icon_path`s containing file paths were ignored in the URL bar.

## [1.1.0] - 2025-12-20
- Initial release with AppImage support.
- Configurable search engines.
- Keyboard navigation improvements.
