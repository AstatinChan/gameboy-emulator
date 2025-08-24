use rodio::{Sink, Source};
use rodio::stream::{OutputStreamBuilder, OutputStream};
use cpal::BufferSize;

use crate::audio::{MutableWave, SAMPLE_RATE};
use crate::io::{Audio, Wave};
use crate::logs::{log, LogLevel};
use std::mem;
use std::time::{SystemTime, Duration};

const BUFFER_SIZE: usize = 16;
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
                return Some(now.duration_since(self.buf[0]).unwrap().as_secs_f32() / (self.i - 1) as f32);
            }
        } else {
            return Some(now.duration_since(previous).unwrap().as_secs_f32() / TIME_RING_BUFFER_SIZE as f32);
        }
    }
}

pub struct RodioAudio {
    sink: Sink,
    stream: OutputStream,

    speed_finder: SpeedFinder,
    wave: RodioWave<MutableWave>,
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

impl Audio for RodioAudio {
    fn new(wave: MutableWave) -> Self {
        let stream = OutputStreamBuilder::from_default_device().unwrap().with_buffer_size(BufferSize::Fixed(RODIO_BUFFER_SIZE as u32)).open_stream().unwrap();

        let sink = Sink::connect_new(stream.mixer());
        let wave = RodioWave(wave, 0);

        RodioAudio {
            speed_finder: SpeedFinder::new(),
            sink: sink,
            wave,
            buffer: Box::new([0.0; BUFFER_SIZE]),
            buffer_i: 0,
            stream,
        }
    }

    fn next(&mut self) {
        if let Some(v) = self.wave.next() {
            self.buffer[self.buffer_i] = v;
            self.buffer_i += 1;

            if self.buffer_i == BUFFER_SIZE {
                self.buffer_i = 0;
                let mut buffer = Box::new([0.0; BUFFER_SIZE]);
                mem::swap(&mut self.buffer, &mut buffer);
                if let Some(speed) = self.speed_finder.tick() {
                    let mut late_speedup: f32;
                    let rodio_buffers_sink_late = self.sink.len() as f32 / (RODIO_BUFFER_SIZE / BUFFER_SIZE) as f32;
                    late_speedup = ((rodio_buffers_sink_late - RODIO_BUFFER_SINK_LATE_EXPECTED).powi(3) / LATE_SPEEDUP_INTENSITY_INV) + 1.;

                    if late_speedup > SPEEDUP_SKIP_LIMIT {
                        while late_speedup > 1.0 {
                            let rodio_buffers_sink_late = self.sink.len() as f32 / (RODIO_BUFFER_SIZE / BUFFER_SIZE) as f32;
                            late_speedup = ((rodio_buffers_sink_late - RODIO_BUFFER_SINK_LATE_EXPECTED).powi(3) / LATE_SPEEDUP_INTENSITY_INV) + 1.;

                            self.sink.skip_one();
                        }
                        late_speedup = 1.;
                    }
                    let average_speed = (1./speed) / (2 * SAMPLE_RATE / BUFFER_SIZE as u32) as f32;
                    let rodio_buffers_sink_late = self.sink.len() as f32 / (RODIO_BUFFER_SIZE / BUFFER_SIZE) as f32;
                    log(LogLevel::AudioLatency, format!("audio sink latency: {}ms", (1000. * rodio_buffers_sink_late / ((2*SAMPLE_RATE) as f32 / RODIO_BUFFER_SIZE as f32))));

                    self.sink.set_speed(late_speedup * average_speed);
                }
                self.sink.append(RodioBuffer(buffer.into_iter()));
            }
        }
    }
}
