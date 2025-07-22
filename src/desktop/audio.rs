use rodio::{OutputStream, Sink, Source};

use crate::audio::{SAMPLE_RATE, MutableWave};
use crate::io::{Wave, Audio};
use std::time::Duration;
use std::mem;

const BUFFER_SIZE: usize = 1024;

pub struct RodioAudio {
    _stream: OutputStream,
    sink: Sink,
    wave: RodioWave<MutableWave>,
    buffer: Box<[f32; BUFFER_SIZE]>,
    buffer_i: usize,
}

struct RodioWave<W: Wave + Send + 'static>(W, usize);

impl<W: Wave + Send + 'static> Iterator for RodioWave<W>
{
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

impl<I: Iterator<Item = f32>> Source for RodioBuffer<I>
{
    fn current_frame_len(&self) -> Option<usize> {
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
        let (stream, stream_handle) = OutputStream::try_default().unwrap();

        let sink = Sink::try_new(&stream_handle).unwrap();
        let wave = RodioWave(wave, 0);

        RodioAudio {
            _stream: stream,
            sink: sink,
            wave,
            buffer: Box::new([0.0; BUFFER_SIZE]),
            buffer_i: 0,
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
                self.sink.append(RodioBuffer(buffer.into_iter()));
            }
        }
    }
}
