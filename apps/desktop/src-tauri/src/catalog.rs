//! claude 2026-09-14: the model catalogue (R09).
//!
//! `transcribe-cpp` 0.2.3 already carries Whisper, Parakeet, Moonshine and
//! Voxtral behind one `StreamExtension` enum, so offering a choice of models
//! needs no second runtime — which is what made this expensive in Handy.
//!
//! Every entry states a pinned revision URL, an exact byte size and a SHA-256.
//! An artifact that does not match all three is never activated: a truncated or
//! substituted model would otherwise surface as mysteriously bad transcription
//! rather than as a download problem.
//!
//! `Family` decides how the worker drives the model. Only the streaming
//! families can satisfy D03's live-caption requirement; a batch-only entry must
//! declare that honestly rather than appear to stream and then not.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Emits partial hypotheses while audio arrives. Satisfies D03.
    MoonshineStreaming,
    /// Parakeet Unified's chunked-attention streaming path.
    ParakeetBuffered,
    /// TDT v3 cannot satisfy live captions in this runtime.
    Offline,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub detail: &'static str,
    /// Languages the upstream model card claims. Not measured by us.
    pub languages: &'static str,
    pub file: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub bytes: u64,
    pub family: Family,
}

impl Model {
    pub fn streams(&self) -> bool {
        matches!(
            self.family,
            Family::MoonshineStreaming | Family::ParakeetBuffered
        )
    }
}

/// Ordered as presented. The first entry is the default for a fresh install.
pub const MODELS: &[Model] = &[
    Model {
        id: "moonshine-streaming-small",
        name: "Moonshine Streaming Small",
        detail: "Live captions while you speak. Fast, and light on memory.",
        languages: "English",
        file: "moonshine-streaming-small-Q8_0.gguf",
        url: "https://huggingface.co/handy-computer/moonshine-streaming-small-gguf/resolve/41444173ed8210852a883e046fadcfba3e7bfbae/moonshine-streaming-small-Q8_0.gguf",
        sha256: "d03670f69629b649085d0f44a63d97668b4119117cc9611a4e4ad94341713dfc",
        bytes: 198_506_848,
        family: Family::MoonshineStreaming,
    },
    Model {
        id: "moonshine-streaming-medium",
        name: "Moonshine Streaming Medium",
        detail: "A larger English model with live captions. Try it alongside Small.",
        languages: "English",
        file: "moonshine-streaming-medium-Q8_0.gguf",
        url: "https://huggingface.co/handy-computer/moonshine-streaming-medium-gguf/resolve/0f99e956a9e63d591ddd7f2a20dfead255e96c68/moonshine-streaming-medium-Q8_0.gguf",
        sha256: "f7c9564249b508f6012927ec4f9e536087da53a7047f858ca9975bea5f75299e",
        bytes: 295_793_568,
        family: Family::MoonshineStreaming,
    },
    Model {
        id: "parakeet-unified-en",
        name: "Parakeet Unified EN",
        detail: "English captions in buffered chunks. Larger download and memory use.",
        languages: "English",
        file: "parakeet-unified-en-0.6b-Q8_0.gguf",
        url: "https://huggingface.co/handy-computer/parakeet-unified-en-0.6b-gguf/resolve/d5249700b2382bf5c5024c2421d101b8db54a629/parakeet-unified-en-0.6b-Q8_0.gguf",
        sha256: "4b50b6dd862bf6e346929aaf4f5eaacec003bfa3f56462d6c874b41ef2f38795",
        bytes: 731_357_568,
        family: Family::ParakeetBuffered,
    },
    Model {
        id: "parakeet-v3",
        name: "Parakeet v3",
        detail: "Batch transcription only in the current runtime. Not offered for live dictation.",
        languages: "English and 24 European languages",
        file: "parakeet-tdt-0.6b-v3-Q8_0.gguf",
        // claude 2026-09-14: UNVERIFIED. This revision, size and digest have not
        // been fetched or checked on this machine. `verified()` excludes any
        // entry whose digest is still the placeholder, so it cannot be offered
        // for download until someone confirms the real values. Leaving a guessed
        // hash in place would mean a failed download reported as a corrupt file.
        url: "",
        sha256: "",
        bytes: 0,
        family: Family::Offline,
    },
];

pub const DEFAULT_ID: &str = MODELS[0].id;

pub fn find(id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|model| model.id == id)
}

/// Entries whose artifact details are actually pinned, and so may be offered.
pub fn offerable() -> impl Iterator<Item = &'static Model> {
    MODELS
        .iter()
        .filter(|model| model.streams() && !model.url.is_empty() && !model.sha256.is_empty() && model.bytes > 0)
}

pub fn default_model() -> &'static Model {
    find(DEFAULT_ID).expect("the default model is part of the catalogue")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_model_is_in_the_catalog_and_streams() {
        assert!(default_model().streams(), "D03 requires live captions");
    }

    #[test]
    fn every_offerable_entry_is_fully_pinned() {
        for model in offerable() {
            assert!(model.url.starts_with("https://"), "{} needs an https url", model.id);
            assert_eq!(model.sha256.len(), 64, "{} needs a full sha-256", model.id);
            assert!(model.bytes > 0, "{} needs an exact size", model.id);
            assert!(!model.file.is_empty());
        }
    }

    #[test]
    fn unpinned_entries_are_never_offered() {
        // Parakeet's artifact details are still placeholders. It must be listed
        // for the reader but never handed to the downloader.
        assert!(find("parakeet-v3").is_some(), "listed in the catalogue");
        assert!(
            !offerable().any(|model| model.id == "parakeet-v3"),
            "an unpinned model must not be downloadable"
        );
    }

    #[test]
    fn identifiers_are_unique() {
        for (index, model) in MODELS.iter().enumerate() {
            assert!(
                MODELS.iter().skip(index + 1).all(|other| other.id != model.id),
                "duplicate id {}",
                model.id
            );
        }
    }

    #[test]
    fn unknown_identifiers_resolve_to_nothing() {
        assert!(find("not-a-model").is_none());
    }

    #[test]
    fn offline_parakeet_does_not_claim_live_captions() {
        assert!(!find("parakeet-v3").unwrap().streams());
    }

    #[test]
    fn alternative_streaming_models_are_pinned_and_offerable() {
        for id in ["moonshine-streaming-medium", "parakeet-unified-en"] {
            let model = offerable().find(|model| model.id == id).expect("usable streaming alternative");
            assert!(model.streams());
        }
    }
}
