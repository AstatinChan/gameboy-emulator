use rodio::{OutputStream, Sink, Source};

use crate::audio::SAMPLE_RATE;
use crate::io::{Wave, Audio};
use std::time::Duration;

pub struct RodioAudio {
    _stream: OutputStream,
    _sink: Sink,
}

struct RodioWave<W: Wave + Send + 'static>(W, usize);

impl<W: Wave + Send + 'static> Iterator for RodioWave<W>
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.1 += 1;
        let left = self.1 % 2 == 0;
        self.0.next(left)
    }
}

impl<W: Wave + Send + 'static> Source for RodioWave<W>
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
    fn new<S: Wave + Send + 'static>(wave: S) -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();

        let sink = Sink::try_new(&stream_handle).unwrap();
        sink.append(RodioWave(wave, 0));

        RodioAudio {
            _stream: stream,
            _sink: sink,
        }
    }
}
