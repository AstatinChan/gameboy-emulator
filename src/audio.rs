// You are entering a very scary territory of magic numbers and arbitrary math operations.
// I don't remember why I did all of this but it works I guess :3

use crate::io::{self, Audio};

use std::sync::{Arc, Mutex};

pub const SAMPLE_RATE: u32 = 65536;

const SAMPLE_AVERAGING: usize = 1; //20;

const SQUARE_WAVE_PATTERN_DUTY_0: [u8; 32] = [
    0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf,
    0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0, 0, 0, 0,
];

const SQUARE_WAVE_PATTERN_DUTY_1: [u8; 32] = [
    0xd, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe,
    0xf, 0xe, 0xf, 0xe, 0xd, 2, 0, 1, 0, 1, 0, 1, 2,
];

const SQUARE_WAVE_PATTERN_DUTY_2: [u8; 32] = [
    0xd, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xf, 0xd, 2, 0, 1, 0, 1,
    0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 2,
];

const SQUARE_WAVE_PATTERN_DUTY_3: [u8; 32] = [
    0xd, 0xf, 0xe, 0xf, 0xe, 0xf, 0xe, 0xd, 2, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1,
    0, 1, 0, 1, 0, 2,
];

const SQUARE_WAVE_PATTERNS: [[u8; 32]; 4] = [
    SQUARE_WAVE_PATTERN_DUTY_0,
    SQUARE_WAVE_PATTERN_DUTY_1,
    SQUARE_WAVE_PATTERN_DUTY_2,
    SQUARE_WAVE_PATTERN_DUTY_3,
];

#[derive(Clone, Debug)]
pub struct Wave {
    period_value: u16,
    pub num_sample: usize,
    wave_pattern: [u8; 32],
    length_timer: u8,
    length_timer_enabled: bool,

    env_initial_volume: f32,
    env_direction: f32,
    env_sweep_pace: u8,

    period_sweep_pace: u8,
    period_sweep_direction: u8,
    period_sweep_slope: u8,

    left_volume: u8,
    right_volume: u8,
}

impl Wave {
    pub fn new(
        num_sample: usize,
        period_value: u16,
        wave_pattern: [u8; 32],
        env_initial_volume: u8,
        env_direction: u8,
        env_sweep_pace: u8,
        length_timer: u8,
        length_timer_enabled: bool,
        period_sweep_pace: u8,
        period_sweep_direction: u8,
        period_sweep_slope: u8,
        left_volume: u8,
        right_volume: u8,
    ) -> Wave {
        Wave {
            period_value,
            num_sample,
            wave_pattern,
            env_initial_volume: env_initial_volume as f32,
            env_direction: if env_direction == 0 { -1. } else { 1. },
            env_sweep_pace,
            length_timer,
            length_timer_enabled,
            period_sweep_pace,
            period_sweep_direction,
            period_sweep_slope,
            left_volume,
            right_volume,
        }
    }
}

impl io::Wave for Wave {
    fn next(&mut self, left: bool) -> Option<f32> {
        if left {
            self.num_sample = self.num_sample.wrapping_add(1);
        }

        let mut period_value = self.period_value;

        if period_value == 0 {
            return None;
        }

        if self.length_timer_enabled
            && self.length_timer < 64
            && SAMPLE_RATE * (64 - self.length_timer as u32) / 256 < self.num_sample as u32
        {
            return None;
        }

        if self.period_sweep_slope != 0 && self.period_sweep_pace != 0 {
            let sweep_i = ((self.num_sample as f32 * (32768 as f32 / SAMPLE_RATE as f32)) as u32
                / 256)
                / self.period_sweep_pace as u32;

            if self.period_sweep_direction == 0 {
                period_value = 2048
                    - ((2048 - period_value) as f32
                        * f32::powf(
                            f32::powf(2., -(self.period_sweep_slope as f32)) + 1.,
                            sweep_i as f32,
                        )) as u16;
            } else {
                period_value = 2048
                    - ((2048 - period_value) as f32
                        * f32::powf(
                            -f32::powf(2., -(self.period_sweep_slope as f32)) + 1.,
                            sweep_i as f32,
                        )) as u16;
            }

            if period_value > 2048 {
                return None;
            }
        }

        let envelope_time = if self.env_sweep_pace != 0 {
            (self.num_sample as f32 / SAMPLE_RATE as f32) * 64. / self.env_sweep_pace as f32
        } else {
            0.
        };

        let envelope = self.env_initial_volume + (self.env_direction * envelope_time);

        let envelope_boundaries = if envelope > 16. {
            16.
        } else if envelope < 0. {
            0.
        } else {
            envelope
        };

        let mut avg = 0.;

        for n in 0..SAMPLE_AVERAGING {
            if self.num_sample as i32 + n as i32 - SAMPLE_AVERAGING as i32 >= 0 {
                avg += (self.wave_pattern[(((8. * 32768. / (SAMPLE_RATE as f32)
                    * (self.num_sample + n - (SAMPLE_AVERAGING / 2)) as f32
                    / period_value as f32)
                    * 16.)
                    % 32.) as u8 as usize] as f32
                    * 2.
                    - 16.)
                    / 16.; // Before you ask, no I don't remember why it's so complicated :3
            }
        }

        if left {
            avg = (self.left_volume as f32 / 8.) * avg;
        } else {
            avg = (self.right_volume as f32 / 8.) * avg;
        }

        Some((avg / SAMPLE_AVERAGING as f32) * envelope_boundaries / 32.)
    }
}

#[derive(Clone, Debug)]
pub struct NoiseWave {
    pub num_sample: usize,
    length_timer: u8,
    length_timer_enabled: bool,

    env_initial_volume: f32,
    env_direction: f32,
    env_sweep_pace: u8,

    clock_shift: u8,
    lsfr_width: u8,
    clock_divider: u8,

    rng: u16,
    last_i: usize,

    left_volume: u8,
    right_volume: u8,
}

impl NoiseWave {
    pub fn new(
        num_sample: usize,
        env_initial_volume: u8,
        env_direction: u8,
        env_sweep_pace: u8,
        length_timer: u8,
        length_timer_enabled: bool,
        clock_shift: u8,
        lsfr_width: u8,
        clock_divider: u8,
        left_volume: u8,
        right_volume: u8,
    ) -> NoiseWave {
        NoiseWave {
            num_sample,
            env_initial_volume: env_initial_volume as f32,
            env_direction: if env_direction == 0 { -1. } else { 1. },
            env_sweep_pace,
            length_timer,
            length_timer_enabled,
            clock_shift,
            lsfr_width,
            clock_divider,
            rng: 0x42,
            last_i: 0,
            left_volume,
            right_volume,
        }
    }
}

impl io::Wave for NoiseWave {
    fn next(&mut self, left: bool) -> Option<f32> {
        if left {
            self.num_sample = self.num_sample.wrapping_add(1);
        }

        let clock_divider = if self.clock_divider == 0 {
            0.5
        } else {
            self.clock_divider as f32
        };

        if self.length_timer_enabled
            && self.length_timer < 64
            && SAMPLE_RATE * (64 - self.length_timer as u32) / 256 < self.num_sample as u32
        {
            return None;
        }

        let envelope_time = if self.env_sweep_pace != 0 {
            (self.num_sample as f32 / SAMPLE_RATE as f32) * 64. / self.env_sweep_pace as f32
        } else {
            0.
        };

        let envelope = self.env_initial_volume + (self.env_direction * envelope_time);

        let envelope_boundaries = if envelope > 16. {
            16.
        } else if envelope < 0. {
            0.
        } else {
            envelope
        };

        let ns = ((262144. / ((clock_divider) * (2 << self.clock_shift) as f32)) / 32768.)
            * self.num_sample as f32;

        let i = (ns as f32 * (32768 as f32 / SAMPLE_RATE as f32)) as usize;

        let up = self.rng & 1;
        if i != self.last_i {
            self.last_i = i;

            self.rng >>= 1;
            if self.lsfr_width == 1 {
                self.rng |= ((self.rng & 1) ^ ((self.rng >> 1) & 1)) << 7;
            } else {
                self.rng |= ((self.rng & 1) ^ ((self.rng >> 1) & 1)) << 15;
            }
        }

        let mut res = up as f32 * 2. - 1.;
        if left {
            res = (self.left_volume as f32 / 8.) * res;
        } else {
            res = (self.right_volume as f32 / 8.) * res;
        }

        Some(res * envelope_boundaries / 64.)
    }
}

#[derive(Clone, Debug)]
pub struct MutableWave {
    wave_ch1: Arc<Mutex<Option<Wave>>>,
    wave_ch2: Arc<Mutex<Option<Wave>>>,
    wave_ch3: Arc<Mutex<Option<Wave>>>,
    wave_ch4: Arc<Mutex<Option<NoiseWave>>>,
}

impl MutableWave {
    pub fn new(
        wave_ch1: Arc<Mutex<Option<Wave>>>,
        wave_ch2: Arc<Mutex<Option<Wave>>>,
        wave_ch3: Arc<Mutex<Option<Wave>>>,
        wave_ch4: Arc<Mutex<Option<NoiseWave>>>,
    ) -> Self {
        Self {
            wave_ch1,
            wave_ch2,
            wave_ch3,
            wave_ch4,
        }
    }
}

impl io::Wave for MutableWave {
    fn next(&mut self, left: bool) -> Option<f32> {
        let mut res = 0.;

        // Imagine using an Arc<Mutex<>> in a sound wave generation function that needs to reliably
        // run 65536 times a second. Couldn't be me :3
        if let Ok(mut wave_o) = self.wave_ch1.lock() {
            if let Some(wave) = wave_o.as_mut() {
                if let Some(result) = wave.next(left) {
                    res += result / 4.;
                } else {
                    *wave_o = None;
                }
            }
        }

        if let Ok(mut wave_o) = self.wave_ch2.lock() {
            if let Some(wave) = wave_o.as_mut() {
                if let Some(result) = wave.next(left) {
                    res += result / 4.;
                } else {
                    *wave_o = None;
                }
            }
        }

        if let Ok(mut wave_o) = self.wave_ch3.lock() {
            if let Some(wave) = wave_o.as_mut() {
                if let Some(result) = wave.next(left) {
                    res += result / 4.;
                } else {
                    *wave_o = None;
                }
            }
        }

        if let Ok(mut wave_o) = self.wave_ch4.lock() {
            if let Some(wave) = wave_o.as_mut() {
                if let Some(result) = wave.next(left) {
                    res += result / 4.;
                } else {
                    *wave_o = None;
                }
            }
        }

        Some(res)
    }
}

pub struct AudioSquareChannel {
    wave: Arc<Mutex<Option<Wave>>>,

    pub length_timer: u8,
    pub length_timer_enabled: bool,
    pub on: bool,
    pub period_value: u16,
    pub duty: u8,
    pub initial_volume: u8,
    pub env_direction: u8,
    pub sweep: u8,
    pub period_sweep_pace: u8,
    pub period_sweep_direction: u8,
    pub period_sweep_slope: u8,
    pub left: bool,
    pub right: bool,
    pub left_volume: u8,
    pub right_volume: u8,
}

impl AudioSquareChannel {
    pub fn new(wave: Arc<Mutex<Option<Wave>>>) -> Self {
        Self {
            on: true,
            period_value: 0,
            duty: 0,
            initial_volume: 0,
            env_direction: 0,
            sweep: 0,
            wave,
            length_timer: 0,
            length_timer_enabled: false,
            period_sweep_pace: 0,
            period_sweep_direction: 0,
            period_sweep_slope: 0,
            left: true,
            right: true,
            left_volume: 7,
            right_volume: 7,
        }
    }

    pub fn update(&mut self, reset: bool) {
        if let Ok(mut wave) = self.wave.lock() {
            let num_sample = if let Some(w) = &*wave {
                w.num_sample
            } else {
                0
            };
            if self.on && (reset || wave.is_some()) {
                *wave = Some(Wave::new(
                    if reset { 0 } else { num_sample },
                    2048 - self.period_value,
                    SQUARE_WAVE_PATTERNS[self.duty as usize],
                    self.initial_volume,
                    self.env_direction,
                    self.sweep,
                    self.length_timer,
                    self.length_timer_enabled,
                    self.period_sweep_pace,
                    self.period_sweep_direction,
                    self.period_sweep_slope,
                    if self.left { self.left_volume + 1 } else { 0 },
                    if self.right { self.right_volume + 1 } else { 0 },
                ));
            } else {
                *wave = None;
            }
        }
    }

    pub fn get_wave_mutex(&self) -> Arc<Mutex<Option<Wave>>> {
        return self.wave.clone();
    }
}

pub struct AudioCustomChannel {
    wave: Arc<Mutex<Option<Wave>>>,

    pub length_timer: u8,
    pub length_timer_enabled: bool,
    pub wave_pattern: [u8; 32],
    pub on: bool,
    pub period_value: u16,
    pub initial_volume: u8,

    pub left: bool,
    pub right: bool,
    pub left_volume: u8,
    pub right_volume: u8,
}

impl AudioCustomChannel {
    pub fn new(wave: Arc<Mutex<Option<Wave>>>) -> Self {
        Self {
            wave_pattern: [0; 32],
            on: true,
            period_value: 0,
            initial_volume: 0,
            wave,
            length_timer: 0,
            length_timer_enabled: false,

            left: true,
            right: true,
            left_volume: 7,
            right_volume: 7,
        }
    }

    pub fn update(&mut self, reset: bool) {
        if let Ok(mut wave) = self.wave.lock() {
            let num_sample = if let Some(w) = &*wave {
                w.num_sample
            } else {
                0
            };
            if self.on && (reset || wave.is_some()) {
                *wave = Some(Wave::new(
                    if reset { 0 } else { num_sample },
                    2 * (2048 - (self.period_value * 2)),
                    self.wave_pattern,
                    self.initial_volume,
                    0,
                    0,
                    self.length_timer,
                    self.length_timer_enabled,
                    0,
                    0,
                    0,
                    if self.left { self.left_volume + 1 } else { 0 },
                    if self.right { self.right_volume + 1 } else { 0 },
                ));
            } else {
                *wave = None;
            }
        }
    }

    pub fn get_wave_mutex(&self) -> Arc<Mutex<Option<Wave>>> {
        return self.wave.clone();
    }
}

pub struct AudioNoiseChannel {
    wave: Arc<Mutex<Option<NoiseWave>>>,

    pub length_timer: u8,
    pub length_timer_enabled: bool,
    pub on: bool,
    pub initial_volume: u8,
    pub env_direction: u8,
    pub sweep: u8,
    pub clock_shift: u8,
    pub lsfr_width: u8,
    pub clock_divider: u8,

    pub left: bool,
    pub right: bool,
    pub left_volume: u8,
    pub right_volume: u8,
}

impl AudioNoiseChannel {
    pub fn new(wave: Arc<Mutex<Option<NoiseWave>>>) -> Self {
        Self {
            on: true,
            initial_volume: 0,
            env_direction: 0,
            sweep: 0,
            wave,
            length_timer: 0,
            length_timer_enabled: false,
            clock_shift: 0,
            lsfr_width: 0,
            clock_divider: 0,

            left: true,
            right: true,
            left_volume: 7,
            right_volume: 7,
        }
    }

    pub fn update(&mut self, reset: bool) {
        if let Ok(mut wave) = self.wave.lock() {
            let num_sample = if let Some(w) = &*wave {
                w.num_sample
            } else {
                0
            };
            if self.on && (reset || wave.is_some()) {
                *wave = Some(NoiseWave::new(
                    if reset { 0 } else { num_sample },
                    self.initial_volume,
                    self.env_direction,
                    self.sweep,
                    self.length_timer,
                    self.length_timer_enabled,
                    self.clock_shift,
                    self.lsfr_width,
                    self.clock_divider,
                    if self.left { self.left_volume + 1 } else { 0 },
                    if self.right { self.right_volume + 1 } else { 0 },
                ));
            } else {
                *wave = None;
            }
        }
    }

    pub fn get_wave_mutex(&self) -> Arc<Mutex<Option<NoiseWave>>> {
        return self.wave.clone();
    }
}

pub struct Channels<A: Audio> {
    audio: A,

    pub ch1: AudioSquareChannel,
    pub ch2: AudioSquareChannel,
    pub ch3: AudioCustomChannel,
    pub ch4: AudioNoiseChannel,
}

impl<A: Audio> Channels<A> {
    pub fn new(mut audio: A) -> Self {
        let wave_ch1 = Arc::new(Mutex::new(None));
        let wave_ch2 = Arc::new(Mutex::new(None));
        let wave_ch3 = Arc::new(Mutex::new(None));
        let wave_ch4 = Arc::new(Mutex::new(None));

        audio.attach_wave(MutableWave::new(
            wave_ch1.clone(),
            wave_ch2.clone(),
            wave_ch3.clone(),
            wave_ch4.clone(),
        ));

        Self {
            audio: audio,
            ch1: AudioSquareChannel::new(wave_ch1),
            ch2: AudioSquareChannel::new(wave_ch2),
            ch3: AudioCustomChannel::new(wave_ch3),
            ch4: AudioNoiseChannel::new(wave_ch4),
        }
    }

    pub fn next(&mut self) {
        self.audio.next();
    }
}
