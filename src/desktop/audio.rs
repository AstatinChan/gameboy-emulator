use rodio::{OutputStream, Sink, Source};

use crate::audio::SAMPLE_RATE;
use crate::io::Audio;
use std::time::Duration;

pub struct RodioAudio {
    _stream: OutputStream,
    _sink: Sink,
}

struct RodioWave<W: Iterator + Send + 'static>(W);

impl<W: Iterator + Send + 'static> Iterator for RodioWave<W>
where
    <W as Iterator>::Item: rodio::Sample,
{
    type Item = W::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<W: Iterator + Send + 'static> Source for RodioWave<W>
where
    <W as Iterator>::Item: rodio::Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Audio for RodioAudio {
    fn new<S: Iterator<Item = f32> + Send + 'static>(wave: S) -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();

        let sink = Sink::try_new(&stream_handle).unwrap();
        sink.append(RodioWave(wave));

        RodioAudio {
            _stream: stream,
            _sink: sink,
        }
    }
}
