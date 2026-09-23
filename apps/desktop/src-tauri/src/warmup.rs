use crate::{
    messages::{emit, WorkerEvent},
    model_file,
};
use std::path::Path;
use transcribe_cpp::Model;

/// Warm the native backend with generated silence. Never open an audio device
/// or publish the recognizer's throwaway output. The parent owns/reaps this process.
pub fn run(path: &Path) -> Result<(), String> {
    emit(&WorkerEvent::Loading)?;
    let spec = model_file::verified_model(path)?;
    let model = Model::load(path).map_err(|_| "Could not prepare the voice model")?;
    let mut session = model
        .session()
        .map_err(|_| "Could not prepare recognition")?;
    let (run, options) = crate::engine_options::options(spec.family)?;
    let mut stream = session
        .stream(&run, &options)
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
