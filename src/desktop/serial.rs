use std::fs::File;
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::io::Serial;
use crate::consts::CPU_CLOCK_SPEED;

pub struct UnconnectedSerial {}

impl Serial for UnconnectedSerial {
    fn read_data(&self) -> u8 {
        0xff
    }
    fn read_control(&self) -> u8 {
        0
    }
    fn write_data(&mut self, _data: u8) {
    }
    fn write_control(&mut self, _control: u8) {
    }
    fn update_serial(&mut self, _cycles: u128) -> bool {
        false
    }
}

enum FIFOMessage {
    Request(u8),
    Response(u8),
}

pub struct FIFOSerial {
    transfer_requested: bool,
    current_transfer: bool,
    current_data: u8,

    external_clock: bool,
    next_byte_transfer_cycle: u128,

    input: Receiver<u8>,
    output: Sender<u8>,
}

impl FIFOSerial {
    pub fn new(input_path: String, output_path: String) -> FIFOSerial {
        let (tx, input) = mpsc::channel::<u8>();
        thread::spawn(move || {
            let mut input_f = File::open(input_path).unwrap();
            loop {
                let mut byte = [0];

                input_f.read(&mut byte).unwrap();

                tx.send(byte[0]).unwrap();
            }
        });

        let (output, rx) = mpsc::channel::<u8>();
        thread::spawn(move || {
            let mut output_f = File::create(output_path).unwrap();
            for b in rx.iter() {
                output_f.write(&[b]).unwrap();
            }
        });

        FIFOSerial {
            transfer_requested: false,
            current_transfer: false,
            current_data: 0,

            external_clock: false,
            next_byte_transfer_cycle: 0,

            input,
            output,
        }
    }
}

impl Serial for FIFOSerial {
    fn read_data(&self) -> u8 {
        self.current_data
    }

    fn read_control(&self) -> u8 {
        (if self.external_clock { 0 } else { 0x01 }) | 
            (if self.transfer_requested { 0x80 } else { 0 })
    }

    fn write_data(&mut self, data: u8) {
        self.current_data = data;
    }

    fn write_control(&mut self, control: u8) {
        self.external_clock = (control & 0b01) == 0;
        self.transfer_requested = (control & 0x80) != 0;
    }

    fn update_serial(&mut self, cycles: u128) -> bool {
        if let Ok(x) = self.input.try_recv() {
            if self.current_transfer {
                self.current_data = x;
                self.external_clock = false;
                self.transfer_requested = false;
                self.next_byte_transfer_cycle = cycles + ((CPU_CLOCK_SPEED as u128) / 1024);
                self.current_transfer = false;
            } else {
                self.output.send(self.current_data).unwrap();
                println!("recv {:02x}, send back {:02x}", x, self.current_data);
                self.current_data = x;
                self.external_clock = true;
                self.transfer_requested = false;
            }
            true
        } else if !self.external_clock && !self.current_transfer
            && self.transfer_requested {
                if cycles > self.next_byte_transfer_cycle {
                    self.output.send(self.current_data).unwrap();
                    println!("send {:02x}", self.current_data);
                    self.current_transfer = true;
                }
            false
        } else {
            false
        }
    }
}
