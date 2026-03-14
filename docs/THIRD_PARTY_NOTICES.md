# Third-Party Notices

Cosmos OSS includes or may bundle third-party binaries and model artifacts at build/release time.
This file tracks required attribution and license context for those components.

## FFmpeg / FFprobe

- **Component**: `ffmpeg`, `ffprobe`
- **Location in repo**: `src-tauri/bin/ffmpeg`, `src-tauri/bin/ffprobe`
- **Upstream project**: https://ffmpeg.org/
- **License**: FFmpeg is generally distributed under LGPL-2.1-or-later, with some builds/options requiring GPL.

Important:
- The exact obligations depend on the build configuration of the binaries you ship.
- If you distribute Cosmos with these binaries, ensure your release process includes the matching FFmpeg license texts and any required source/build-offer obligations for that binary build.

## ONNX Runtime (dynamic library at packaging time)

- **Component**: ONNX Runtime dynamic library
- **Expected filename in build script**: `libonnxruntime.1.22.0.dylib`
- **Build script reference**: `src-tauri/build.rs`
- **Upstream project**: https://github.com/microsoft/onnxruntime
- **License**: MIT

If you distribute bundles containing ONNX Runtime, include ONNX Runtime attribution/license text with your release artifacts.

## Nomic / Whisper model files

Cosmos downloads model files from upstream registries at runtime based on configured model endpoints.
Those model assets have their own licenses/terms from their respective providers.
When redistributing pre-bundled model files, verify and comply with each model license.
