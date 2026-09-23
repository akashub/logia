# Third-party components

Logia's preview uses the following primary components. Their licenses do not select a license for Logia's own code.

- [transcribe.cpp](https://github.com/handy-computer/transcribe.cpp), version 0.2.3: native speech recognition, MIT. The license is retained in `licenses/transcribe-cpp-MIT.txt`.
- [Moonshine streaming small, GGUF conversion](https://huggingface.co/handy-computer/moonshine-streaming-small-gguf/tree/41444173ed8210852a883e046fadcfba3e7bfbae): separately downloaded model artifact. Original model authors: Moonshine AI. Conversion publisher: handy-computer. The pinned artifact hash is in `src-tauri/src/catalog.rs`; model weights are excluded from this repository.
- [Moonshine streaming medium, GGUF conversion](https://huggingface.co/handy-computer/moonshine-streaming-medium-gguf/tree/0f99e956a9e63d591ddd7f2a20dfead255e96c68): MIT model weights, separately downloaded. Original model authors: Moonshine AI; conversion publisher: handy-computer. Exact revision, size and SHA-256 are pinned in `src-tauri/src/catalog.rs`.
- [Parakeet Unified English 0.6B, GGUF conversion](https://huggingface.co/handy-computer/parakeet-unified-en-0.6b-gguf/tree/d5249700b2382bf5c5024c2421d101b8db54a629): [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) model weights, separately downloaded. Original model authors: NVIDIA; conversion and Q8_0 quantization publisher: handy-computer. Exact revision, size and SHA-256 are pinned in `src-tauri/src/catalog.rs`. Logia makes no further changes to the downloaded weights and does not include them in source or app bundles.
- Archivo and Newsreader, distributed by [Fontsource](https://fontsource.org/): SIL Open Font License 1.1. Both license texts and copyright notices are retained in `licenses/`.
- [Tauri](https://github.com/tauri-apps/tauri), its clipboard plugin, and [global shortcut plugin](https://v2.tauri.app/plugin/global-shortcut/) (2.3.2): MIT or Apache-2.0.
- [objc2 AppKit bindings](https://github.com/madsmtm/objc2) (0.3.2): MIT. Used to reveal the Mac dictation window without making it the keyboard target.
- [React](https://github.com/facebook/react): MIT.
- [CPAL](https://github.com/RustAudio/cpal): Apache-2.0; [Rubato](https://github.com/HEnquist/rubato): MIT.

Exact direct and transitive package versions are recorded in `pnpm-lock.yaml` and `src-tauri/Cargo.lock`. A distributable release must include the complete dependency notices, including transitive native libraries. This is a local development preview, with no signed public binary release.
