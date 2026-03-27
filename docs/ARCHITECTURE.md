# Cosmos OSS Architecture

This document explains the main runtime flows in Cosmos OSS using Mermaid diagrams. It is intended for contributors and advanced users who want to understand how the desktop app is structured internally.

## 1. High-Level Architecture

```mermaid
flowchart LR
    user["User"]
    ui["React UI\nsrc/"]
    tauri["Tauri desktop shell"]
    backend["Rust backend\nsrc-tauri/"]

    files["Local files, folders,\nand external drives"]
    models["Local models\nNomic + Whisper + tools"]
    db["Local SQLite/SQLCipher\n+ sqlite-vec index"]
    jobs["Background jobs\nindexing, watched folders,\nmodel downloads"]
    optional["Optional external APIs\nGemini / Veo,\nmodel downloads"]

    user --> ui
    ui --> tauri
    tauri --> backend

    backend --> files
    backend --> models
    backend --> db
    backend --> jobs
    backend -. "only when enabled" .-> optional

    files --> backend
    models --> backend
    db --> backend
```

## 2. Indexing Pipeline

```mermaid
flowchart TD
    start["User selects a folder\nor enables a watched folder"]
    discover["Rust backend discovers files\nand classifies file types"]
    route["Route by content type"]

    text["Text extraction\nchunk documents"]
    image["Image analysis"]
    video["Video analysis\nthumbnails + media handling"]
    audio["Optional transcription\nvia Whisper"]

    embedText["Generate text embeddings"]
    embedVisual["Generate visual embeddings"]
    transcript["Store transcript text"]

    writeMeta["Write file metadata"]
    writeVec["Write vectors to sqlite-vec"]
    writeChunks["Write text chunks / transcripts"]
    jobs["Update indexing jobs\nand progress events"]
    ui["UI updates library,\nsettings, and job status"]

    start --> discover --> route

    route --> text
    route --> image
    route --> video
    route --> audio

    text --> embedText
    image --> embedVisual
    video --> embedVisual
    audio --> transcript

    embedText --> writeVec
    embedVisual --> writeVec
    text --> writeChunks
    transcript --> writeChunks

    discover --> writeMeta
    writeMeta --> jobs
    writeVec --> jobs
    writeChunks --> jobs
    jobs --> ui
```

## 3. Search Pipeline

```mermaid
flowchart TD
    query["User enters text query\nor uploads an image"]
    mode["Choose search mode"]

    textQuery["Text query"]
    imageQuery["Image query"]

    textEmbed["Generate query embedding"]
    imageEmbed["Generate visual query embedding"]

    vecSearch["Vector similarity search\nin sqlite-vec"]
    join["Join vector matches with\nfile metadata, chunks,\nand transcripts"]
    rank["Rank and group results"]
    preview["Return previews,\nmetadata, and file actions"]
    ui["React UI renders results\nand preview area"]

    query --> mode
    mode --> textQuery
    mode --> imageQuery

    textQuery --> textEmbed --> vecSearch
    imageQuery --> imageEmbed --> vecSearch

    vecSearch --> join --> rank --> preview --> ui
```

## Notes

- Cosmos OSS is local-first. Core indexing and search run on-device.
- External network access is optional and mostly limited to model downloads or user-enabled integrations.
- The UI is intentionally thin compared to the backend; most indexing, search, media processing, and persistence logic lives in Rust.
