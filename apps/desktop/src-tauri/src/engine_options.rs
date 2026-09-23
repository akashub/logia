use crate::catalog::Family;
use transcribe_cpp::{CommitPolicy, MoonshineStreamingOptions, ParakeetBufferedStreamOptions, RunOptions, StreamExtension, StreamOptions};

/// Warmup and live capture must drive the selected family identically.
pub fn options(family: Family) -> Result<(RunOptions, StreamOptions), String> {
    let family = match family {
        Family::MoonshineStreaming => StreamExtension::MoonshineStreaming(MoonshineStreamingOptions {
            min_decode_interval_ms: Some(480),
        }),
        Family::ParakeetBuffered => StreamExtension::ParakeetBuffered(ParakeetBufferedStreamOptions::default()),
        Family::Offline => return Err("This model cannot provide live captions".into()),
    };
    Ok((RunOptions { language: Some("en".into()), ..Default::default() }, StreamOptions {
        commit_policy: CommitPolicy::OnFinalize, family: Some(family), ..Default::default()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn families_use_distinct_streaming_contracts_and_offline_is_rejected() {
        let (run, small) = options(Family::MoonshineStreaming).unwrap();
        assert_eq!(run.language.as_deref(), Some("en"));
        assert!(matches!(small.family, Some(StreamExtension::MoonshineStreaming(MoonshineStreamingOptions { min_decode_interval_ms: Some(480) }))));
        let (_, unified) = options(Family::ParakeetBuffered).unwrap();
        assert!(matches!(unified.family, Some(StreamExtension::ParakeetBuffered(_))));
        assert!(options(Family::Offline).is_err());
    }
}
