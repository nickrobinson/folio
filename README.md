# Folio

A pure-Rust PDF library with cross-language bindings via [UniFFI](https://github.com/nickvision/uniffi-rs).

## Status

**Early development.** Tier 1 (core foundation) is implemented. The library can open, inspect, create, and save PDF documents.

### What works today

- **PDF parser** — tokenizer, object parser, xref table parsing, incremental update support
- **COS object model** — all 8 PDF object types (null, bool, integer, real, name, string, array, dict, stream) plus indirect references
- **Document API** — open from file/bytes, page count, page access, page creation, save to file/bytes
- **Page model** — media/crop/bleed/trim/art boxes, rotation, effective width/height, annotations count
- **Filter pipeline** — FlateDecode, ASCII85Decode, ASCIIHexDecode, LZWDecode, RunLengthDecode, PNG/TIFF predictors
- **Document metadata** — title, author, subject, keywords, creator, producer, dates
- **Serialization** — full PDF write with xref table and trailer
- **UniFFI bindings** — skeleton with all primitive types exposed to Swift/Kotlin

### Oracle-verified

Every feature is tested against independent PDF implementations for correctness:

- Native parser opens **28/28** (100%) of corpus PDFs
- Page count matches oracles on **14/14** (100%) of comparable PDFs
- Page geometry matches on **20/25** (80%) of comparable pages
- Round-trip (open → save → re-open by oracle) passes for **9/15** (60%)

### Known limitations

- Cross-reference streams (PDF 1.5+) not yet decoded — affects 14 corpus PDFs
- Object streams not yet decompressed
- Page tree attribute inheritance (MediaBox from parent Pages node) not implemented
- Incremental save not yet supported (full save only)
- No encryption/decryption yet

## Architecture

```
folio/
├── crates/
│   ├── folio-core/       # Primitive types: Rect, Matrix2D, ColorPt, Point, Date, Error
│   ├── folio-filters/    # Stream decode/encode: Flate, ASCII85, ASCIIHex, LZW, RunLength
│   ├── folio-cos/        # COS object model, PDF tokenizer, parser, xref, serializer
│   ├── folio-doc/        # High-level PdfDoc, Page, DocInfo
│   └── folio-uniffi/     # UniFFI language bindings (Swift, Kotlin, Python)
└── tests/corpus/         # 28 test PDFs
```

The architecture follows a layered design:

1. **COS layer** (`folio-cos`) — raw PDF object graph, parsing, serialization
2. **Document layer** (`folio-doc`) — PDF-specific semantics (pages, metadata, structure)
3. **Bindings layer** (`folio-uniffi`) — cross-language API via UniFFI

## Building

```sh
# Build everything
cargo build --workspace

# Run all tests (89 tests)
cargo test --workspace

```

### Requirements

- Rust 1.82+ (2024 edition)

## Roadmap

| Tier | Scope | Status |
|------|-------|--------|
| 0 | Project scaffolding, oracle harness, UniFFI skeleton | Done |
| 1 | COS object model, filters, PDF parser, PDFDoc/Page | Done |
| 2 | Content streams, graphics state, fonts, images, text extraction | Planned |
| 3 | Annotations, forms, bookmarks, content creation | Planned |
| 4 | Encryption, digital signatures, optimization | Planned |
| 5 | Rendering, conversion, PDF/A compliance, layout | Planned |

## License

MIT OR Apache-2.0
