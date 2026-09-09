use crate::{
    messages::{emit, WorkerEvent},
    model_file,
};
use std::path::Path;
use transcribe_cpp::{Model, RunOptions, StreamOptions};

/// Warm the native backend with generated silence. Never open an audio device
/// or publish the recognizer's throwaway output. The parent owns/reaps this process.
pub fn run(path: &Path) -> Result<(), String> {
    emit(&WorkerEvent::Loading)?;
    model_file::verify(path)?;
    let model = Model::load(path).map_err(|_| "Could not prepare the voice model")?;
    let mut session = model
        .session()
        .map_err(|_| "Could not prepare recognition")?;
    let mut stream = session
        .stream(&RunOptions::default(), &StreamOptions::default())
        .map_err(|_| "Could not prepare live recognition")?;
    for _ in 0..32 {
        stream
            .feed(&[0.; 1024])
            .map_err(|_| "Voice model preparation failed")?;
    }
    stream
        .finalize()
        .map_err(|_| "Could not finish voice model preparation")?;
    Ok(())
}
