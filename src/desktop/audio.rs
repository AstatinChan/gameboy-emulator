use cpal::platform::Stream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{self, BufferSize};
use rodio::stream::{OutputStream, OutputStreamBuilder};
use rodio::{Sink, Source};

use crate::audio::{MutableWave, SAMPLE_RATE};
use crate::io::{Audio, Wave};
use crate::logs::{elog, log, LogLevel};
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

#[cfg(target_family = "wasm")]
use crate::wasm::utils::SystemTime;
use std::time::Duration;
#[cfg(not(target_family = "wasm"))]
use std::time::SystemTime;

const BUFFER_SIZE: usize = 16;
const CPAL_BUFFERSIZE: u32 = 4096;
const RODIO_BUFFER_SIZE: usize = 2048;
const RODIO_BUFFER_SINK_LATE_EXPECTED: f32 = 2.;
const LATE_SPEEDUP_INTENSITY_INV: f32 = 2048.0;
const SPEEDUP_SKIP_LIMIT: f32 = 1.008;

const TIME_RING_BUFFER_SIZE: usize = (SAMPLE_RATE as usize / BUFFER_SIZE) * 10;
struct SpeedFinder {
    buf: [SystemTime; TIME_RING_BUFFER_SIZE],
    i: usize,
    has_circled: bool,
}

impl SpeedFinder {
    fn new() -> Self {
        Self {
            buf: [SystemTime::now(); TIME_RING_BUFFER_SIZE],
            i: 0,
            has_circled: false,
        }
    }

    fn tick(&mut self) -> Option<f32> {
        if self.i >= TIME_RING_BUFFER_SIZE {
            self.i = 0;
            self.has_circled = true;
        }

        let previous = self.buf[self.i];
        let now = SystemTime::now();

        self.buf[self.i] = now;
        self.i += 1;
        if !self.has_circled {
            if self.i == 1 {
                return None;
            } else {
                return Some(
                    now.duration_since(self.buf[0]).unwrap().as_secs_f32() / (self.i - 1) as f32,
                );
            }
        } else {
            return Some(
                now.duration_since(previous).unwrap().as_secs_f32() / TIME_RING_BUFFER_SIZE as f32,
            );
        }
    }
}

pub struct HeadlessAudio {}

impl Audio for HeadlessAudio {
    fn attach_wave(&mut self, _wave: MutableWave) {}
    fn next(&mut self) {}
}

pub struct CpalAudio {
    _stream: Stream,
    wave_sender: Sender<f32>,

    left: bool,
    wave: Option<MutableWave>,

    samples_to_play: Arc<AtomicUsize>,
}

impl CpalAudio {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .next()
            .expect("No supported configs")
            .with_sample_rate(cpal::SampleRate(SAMPLE_RATE));

        let mut config = supported_config.config();

        config.buffer_size = BufferSize::Fixed(CPAL_BUFFERSIZE);
        config.channels = 2;

        let (sender, receiver) = channel::<f32>();

        let samples_to_play = Arc::new(AtomicUsize::new(0));
        let samples_to_play_clone = samples_to_play.clone();

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut sample_received = data.len();
                for i in 0..data.len() {
                    if let Ok(value) = receiver.try_recv() {
                        data[i] = value;
                    } else {
                        sample_received = i;
                        break;
                    }
                }
                samples_to_play_clone.fetch_sub(sample_received, Ordering::SeqCst);
                let samples_to_play = samples_to_play_clone.load(Ordering::SeqCst);
                let latency_s = samples_to_play as f64 / (SAMPLE_RATE as f64);
                let latency_in_buffers = samples_to_play as f64 / (data.len() as f64);
                if latency_in_buffers > 3. && latency_s > 0.1 {
                    let mut skipping_count = samples_to_play - 3*data.len();
                    log(LogLevel::Infos, format!("Audio Latency higher than 100ms, skipping {} samples", skipping_count));
                    for i in 0..skipping_count {
                        if let Err(_) = receiver.try_recv() {
                            elog(LogLevel::Error, format!("Samples were fewer than expected when skipping samples (expected {}, got {}", samples_to_play, i));
                            skipping_count = i;
                            break;
                        }
                    }

                    samples_to_play_clone.fetch_sub(skipping_count, Ordering::SeqCst);
                }
            },
            |err| {
                log(LogLevel::Error, format!("Cpal Stream error: {:?}", err))
            },
            None
        ).unwrap();

        stream.play().unwrap();

        Self {
            _stream: stream,
            wave_sender: sender,

            wave: None,
            left: false,
            samples_to_play,
        }
    }
}

impl Audio for CpalAudio {
    fn attach_wave(&mut self, wave: MutableWave) {
        self.wave = Some(wave);
    }

    fn next(&mut self) {
        if let Some(wave) = &mut self.wave {
            if let Some(v) = wave.next(self.left) {
                let _ = self.wave_sender.send(v);
                self.samples_to_play.fetch_add(1, Ordering::SeqCst);
            }
            self.left = !self.left;
        }
    }
}

pub struct RodioAudio {
    sink: Sink,
    _stream: OutputStream,

    speed_finder: SpeedFinder,
    wave: Option<RodioWave<MutableWave>>,
    buffer: Box<[f32; BUFFER_SIZE]>,
    buffer_i: usize,
}

struct RodioWave<W: Wave + Send + 'static>(W, usize);

impl<W: Wave + Send + 'static> Iterator for RodioWave<W> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.1 += 1;
        let left = self.1 % 2 == 0;
        let result = self.0.next(left);

        result
    }
}

struct RodioBuffer<I: Iterator<Item = f32>>(I);

impl<I: Iterator<Item = f32>> Iterator for RodioBuffer<I> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<I: Iterator<Item = f32>> Source for RodioBuffer<I> {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl RodioAudio {
    pub fn new() -> Self {
        let stream = OutputStreamBuilder::from_default_device()
            .unwrap()
            .with_buffer_size(BufferSize::Fixed(RODIO_BUFFER_SIZE as u32))
            .open_stream()
            .unwrap();

        let sink = Sink::connect_new(stream.mixer());

        RodioAudio {
            speed_finder: SpeedFinder::new(),
            sink: sink,
            wave: None,
            buffer: Box::new([0.0; BUFFER_SIZE]),
            buffer_i: 0,
            _stream: stream,
        }
    }
}

impl Audio for RodioAudio {
    fn attach_wave(&mut self, wave: MutableWave) {
        let wave = RodioWave(wave, 0);

        self.wave = Some(wave);
    }

    fn next(&mut self) {
        if let Some(wave) = &mut self.wave {
            if let Some(v) = wave.next() {
                self.buffer[self.buffer_i] = v;
                self.buffer_i += 1;

                if self.buffer_i == BUFFER_SIZE {
                    self.buffer_i = 0;
                    let mut buffer = Box::new([0.0; BUFFER_SIZE]);
                    mem::swap(&mut self.buffer, &mut buffer);
                    if let Some(speed) = self.speed_finder.tick() {
                        let mut late_speedup: f32;
                        let rodio_buffers_sink_late =
                            self.sink.len() as f32 / (RODIO_BUFFER_SIZE / BUFFER_SIZE) as f32;
                        late_speedup =
                            ((rodio_buffers_sink_late - RODIO_BUFFER_SINK_LATE_EXPECTED).powi(3)
                                / LATE_SPEEDUP_INTENSITY_INV)
                                + 1.;

                        if late_speedup > SPEEDUP_SKIP_LIMIT {
                            while late_speedup > 1.0 {
                                let rodio_buffers_sink_late = self.sink.len() as f32
                                    / (RODIO_BUFFER_SIZE / BUFFER_SIZE) as f32;
                                late_speedup = ((rodio_buffers_sink_late
                                    - RODIO_BUFFER_SINK_LATE_EXPECTED)
                                    .powi(3)
                                    / LATE_SPEEDUP_INTENSITY_INV)
                                    + 1.;

                                self.sink.skip_one();
                            }
                            late_speedup = 1.;
                        }
                        let average_speed =
                            (1. / speed) / (2 * SAMPLE_RATE / BUFFER_SIZE as u32) as f32;
                        let rodio_buffers_sink_late =
                            self.sink.len() as f32 / (RODIO_BUFFER_SIZE / BUFFER_SIZE) as f32;
                        log(
                            LogLevel::AudioLatency,
                            format!(
                                "audio sink latency: {}ms",
                                (1000. * rodio_buffers_sink_late
                                    / ((2 * SAMPLE_RATE) as f32 / RODIO_BUFFER_SIZE as f32))
                            ),
                        );

                        self.sink.set_speed(late_speedup * average_speed);
                    }
                    self.sink.append(RodioBuffer(buffer.into_iter()));
                }
            }
        }
    }
}
