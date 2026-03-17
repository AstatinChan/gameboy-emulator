#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    pub fn now() -> u32;

    #[wasm_bindgen]
    pub fn alert(u: u32);
}

#[derive(Copy, Clone, Debug)]
pub struct Duration(pub u32);

impl Duration {
    pub fn as_nanos(&self) -> u128 {
        self.0 as u128 * 1000_000
    }
    pub fn as_millis(&self) -> u128 {
        self.0 as u128
    }
    pub fn as_micros(&self) -> u128 {
        self.0 as u128 * 1000
    }
    pub fn as_secs_f32(&self) -> f32 {
        self.0 as f32 / 1000.
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SystemTime(pub u32);

impl SystemTime {
    pub fn now() -> SystemTime {
        SystemTime(now())
    }

    pub fn duration_since(&self, other: SystemTime) -> Result<Duration, ()> {
        Ok(Duration(self.0 - other.0))
    }
}

#[cfg(target_family = "wasm")]
pub fn test() {
    alert(now());
}
