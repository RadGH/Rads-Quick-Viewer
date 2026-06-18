# Rads Quick Viewer

Rads Quick Viewer is a lightweight desktop viewer for Markdown, `.eml` email, images, audio, video, text, and lightweight `.docx` text extraction.

The app is built with Tauri 2, Preact, and Rust. The viewer layer is intentionally
small and registry-based so future file types can be added without reshaping the
main window.

## Current v1 scope

- Markdown preview with source toggle.
- `.eml` email preview with source toggle.
- OS light/dark theme matching.
- File picker for supported files.
- Command-line file path support for file associations.
- Settings button that opens the Windows default-app settings page.

## Development

```powershell
npm install
npm run dev
```

## Build

```powershell
npm run build
```

The Windows bundle target is currently NSIS.
