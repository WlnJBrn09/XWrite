# XWrite

Local-first word processor with an Apple-inspired liquid-glass UI and a **Rust** backend.

Nothing is uploaded to the cloud. Documents are JSON files on disk.

## Requirements

- Rust 1.75+ (`cargo`)
- WebKitGTK: `libwebkit2gtk-4.1-dev` (Debian/Ubuntu) or `webkit2gtk4.1-devel` (Fedora)

## Desktop app (native)

Thin desktop host (system WebKitGTK) that spawns the Rust backend — **no Electron**.

```bash
npm run native:build
npm run native
npm run dist:deb       # Debian/Ubuntu .deb
npm run dist:rpm       # Fedora/RHEL/openSUSE .rpm
```

Binary (dev): `native-host/target/release/xwrite-native`

See `native-host/README.md`.

## Run (dev server)

```bash
cargo run
```

Then open **http://127.0.0.1:8787**

Optional environment variables:

| Variable | Default | Meaning |
| --- | --- | --- |
| `PORT` | `8787` | HTTP port (localhost only) |
| `XWRITE_DATA_DIR` | `./documents` | Where `.json` documents are stored |
| `XWRITE_STATIC_DIR` | `./static` | Frontend assets |

## Features

- **Apple-style Liquid Glass** chrome
- Local document create / open / auto-save / star
- Fonts, colors, highlight, bold/italic/underline, lists, links, print

## Files

Open TXT, Markdown, HTML, DOCX, PDF, and XWrite JSON files from the Documents library or the file picker. Export TXT, Markdown, DOCX, and PDF. DOCX and PDF exports preserve headings, emphasis, alignment, lists, links, tables, and embedded images. PDF exports embed fonts for searchable text and paginate long documents. Characters missing from the bundled PDF fonts produce an export error rather than a replacement glyph. DOCX imports currently extract text only; use the app's local JSON documents to keep editor formatting when editing again in XWrite.

## Tests

```bash
npm test
npm run test:glass
```

## License

MIT
