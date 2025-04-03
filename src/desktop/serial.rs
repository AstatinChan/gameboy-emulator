use std::fs::File;
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::io::Serial;

pub struct UnconnectedSerial {}

impl Serial for UnconnectedSerial {
    fn write(&mut self, byte: u8) {
        println!("Writing {} to unconnected serial", byte);
    }

    fn read(&mut self) -> u8 {
        println!("Reading 0 from unconnected serial");
        0
    }

    fn new_transfer(&mut self) -> bool {
        false
    }

    fn clock_master(&mut self) -> bool {
        false
    }

    fn set_clock_master(&mut self, _clock_master: bool) {}
}

pub struct FIFOPackage {
    t: bool,
    value: u8,
}

pub struct FIFOSerial {
    input: Receiver<u8>,
    output: Sender<FIFOPackage>,
    clock_change: Receiver<bool>,
    last_read_byte: u8,
    clock_master: bool,
}

impl FIFOSerial {
    pub fn new(input_path: String, output_path: String) -> FIFOSerial {
        let (tx, input) = mpsc::channel::<u8>();
        let (clock_tx, clock_change) = mpsc::channel::<bool>();
        thread::spawn(move || {
            let mut input_f = File::open(input_path).unwrap();
            loop {
                let mut byte = [0, 0];

                input_f.read(&mut byte).unwrap();
                if byte[0] == 1 {
                    tx.send(byte[1]).unwrap();
                } else {
                    clock_tx.send(byte[1] == 0).unwrap();
                }
            }
        });

        let (output, rx) = mpsc::channel::<FIFOPackage>();
        thread::spawn(move || {
            let mut output_f = File::create(output_path).unwrap();
            for b in rx.iter() {
                if b.t {
                    output_f.write(&[1, b.value]).unwrap();
                } else {
                    output_f.write(&[0, b.value]).unwrap();
                }
            }
        });

        FIFOSerial {
            input,
            output,
            clock_change,
            last_read_byte: 0xff,
            clock_master: false,
        }
    }
}

impl Serial for FIFOSerial {
    fn write(&mut self, byte: u8) {
        println!("Writing {} to fifo serial", byte);
        if let Err(err) = self.output.send(FIFOPackage {
            t: true,
            value: byte,
        }) {
            eprintln!("Error while sending serial package: {}", err);
        };
    }

    fn read(&mut self) -> u8 {
        println!("Reading {} from fifo serial", self.last_read_byte);
        self.last_read_byte
    }

    fn new_transfer(&mut self) -> bool {
        match self.input.try_recv() {
            Ok(byte) => {
                println!("Received: {}", byte);
                self.last_read_byte = byte;
                true
            }
            _ => false,
        }
    }
    fn clock_master(&mut self) -> bool {
        match self.clock_change.try_recv() {
            Ok(byte) => {
                println!("Received clock change, master: {}", byte);
                self.clock_master = byte;
            }
            _ => {}
        };
        self.clock_master
    }

    fn set_clock_master(&mut self, clock_master: bool) {
        self.clock_master = clock_master;
        if let Err(err) = self.output.send(FIFOPackage {
            t: false,
            value: (if clock_master { 1 } else { 0 }),
        }) {
            eprintln!("Error while sending serial package: {}", err);
        }
    }
}
