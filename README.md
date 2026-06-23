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

## Screenshots

### Markdown Files (.md)
<img width="1340" height="764" alt="Markdown file preview" src="https://github.com/user-attachments/assets/1b977583-c273-4849-92f2-818123bb8591" />

### Audio Files (.mp3)
<img width="1340" height="764" alt="Audio file preview" src="https://github.com/user-attachments/assets/bc5855a7-c066-48a1-96f2-a944e3c6f0c6" />

### Image Files (.png)
<img width="1340" height="764" alt="Image file preview" src="https://github.com/user-attachments/assets/cb9a797c-053b-4f06-9f53-015d44580a7b" />

### Other Text Files (.docx)
It is worth pointing out that this app currently does not support Word Documents, but can still open them for basic viewing. Many features are unsupported.

<img width="1340" height="764" alt="image" src="https://github.com/user-attachments/assets/1d26d619-b896-4c48-ba95-77b91daca01c" />


