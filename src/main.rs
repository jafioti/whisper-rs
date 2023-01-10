use record_audio::audio_clip::AudioClip as ac;
use whisper_rs::{WhisperContext, FullParams, SamplingStrategy};


fn main() {
    // Record clip
    let clip = ac::record()
        .unwrap()
        .resample(16000);
        
    // load a context and model
    let mut ctx = WhisperContext::new("./ggml-base.en.bin").unwrap();
    
    // now we can run the model
    ctx.full(FullParams::new(SamplingStrategy::Greedy { n_past: 0 }), &clip.samples[..]).unwrap();

    // fetch the results
    for i in 0..ctx.full_n_segments() {
        let segment = ctx.full_get_segment_text(i).unwrap();
        let start_timestamp = ctx.full_get_segment_t0(i);
        let end_timestamp = ctx.full_get_segment_t1(i);
        println!("[{} - {}]: {}", start_timestamp, end_timestamp, segment);
    }
}