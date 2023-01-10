use std::sync::{mpsc::channel, Arc, Mutex};

use color_eyre::eyre::eyre;
use color_eyre::eyre::Result;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use dasp::interpolate::linear::Linear;
use dasp::{signal, Signal};

type ClipHandle = Arc<Mutex<Option<AudioClip>>>;

#[derive(Clone)]
pub struct AudioClip {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl AudioClip {
    pub fn record() -> Result<AudioClip> {        
        //get the default input device
        let device = cpal::default_host()
            .default_input_device()
            .ok_or_else(|| eyre!("No input device!"))?;

        //get default config - channels, sample_rate,buffer_size, sample_format
        let config = device.default_input_config()?;

        //init a audio clip
        let clip = AudioClip {
            samples: Vec::new(),
            sample_rate: config.sample_rate().0,
        };

        let clip = Arc::new(Mutex::new(Some(clip)));

        // Run the input stream on a separate thread.
        let clip_2 = clip.clone();

        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        //get number of channels
        let channels = config.channels();

        //create stream
        let stream = device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data(data, channels, &clip_2),
            err_fn,
        )?;

        //run stream
        stream.play()?;

        //ctrl c signal
        let (tx, rx) = channel();
        ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))?;

        rx.recv()?;

        drop(stream);
        let clip = clip.lock().unwrap().take().unwrap();
        Ok(clip)
    }

    pub fn export(&self, path: &str) -> Result<()> {
        if !path.ends_with(".wav") {
            return Err(eyre!("Expected {} to end in .wav", path));
        }

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;
        for sample in &self.samples {
            println!("Sample: {}", sample);
            writer.write_sample(*sample as i16)?;
        }

        writer.finalize()?;

        Ok(())
    }

    pub fn resample(&self, sample_rate: u32) -> AudioClip {
        if sample_rate == self.sample_rate {
            return self.clone();
        }

        let mut signal = signal::from_iter(self.samples.iter().copied());
        let a = signal.next();
        let b = signal.next();

        let linear = Linear::new(a, b);

        let clip = AudioClip {            
            samples: signal
                .from_hz_to_hz(linear, self.sample_rate as f64, sample_rate as f64)
                .take(self.samples.len() * (sample_rate as usize) / (self.sample_rate as usize))
                .collect(),
            sample_rate: sample_rate,
        };

        clip
    }
}

fn write_input_data(input: &[f32], channels: u16, writer: &ClipHandle)
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for frame in input.chunks(channels.into()) {
                writer.samples.push(frame[0]);
            }
        }
    }
}
