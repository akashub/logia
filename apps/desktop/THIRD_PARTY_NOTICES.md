# Third-party components

Logia's preview uses the following primary components. Their licenses do not select a license for Logia's own code.

- [transcribe.cpp](https://github.com/handy-computer/transcribe.cpp), version 0.2.3: native speech recognition, MIT. The license is retained in `licenses/transcribe-cpp-MIT.txt`.
- [Moonshine streaming small, GGUF conversion](https://huggingface.co/handy-computer/moonshine-streaming-small-gguf/tree/41444173ed8210852a883e046fadcfba3e7bfbae): separately downloaded model artifact. Original model authors: Moonshine AI. Conversion publisher: handy-computer. The pinned artifact hash is in `src-tauri/src/model_file.rs`; model weights are excluded from this repository.
- Archivo and Newsreader, distributed by [Fontsource](https://fontsource.org/): SIL Open Font License 1.1. Both license texts and copyright notices are retained in `licenses/`.
- [Tauri](https://github.com/tauri-apps/tauri) and its clipboard plugin: MIT or Apache-2.0.
- [React](https://github.com/facebook/react): MIT.
- [CPAL](https://github.com/RustAudio/cpal): Apache-2.0; [Rubato](https://github.com/HEnquist/rubato): MIT.

Exact direct and transitive package versions are recorded in `pnpm-lock.yaml` and `src-tauri/Cargo.lock`. A distributable release must include the complete dependency notices, including transitive native libraries. This is a local development preview, with no signed public binary release.
