use std::fs::File;
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::net::{TcpListener, TcpStream};
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

pub struct FIFOSerial {
    transfer_requested: bool,
    current_transfer: bool,
    current_data: u8,

    external_clock: bool,
    next_byte_transfer_cycle: u128,
    no_response: bool,

    input: Receiver<u8>,
    output: Sender<u8>,
}

impl FIFOSerial {
    pub fn new(input_path: String, output_path: String, no_response: bool) -> FIFOSerial {
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

            no_response,

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
                if !self.no_response {
                    self.output.send(self.current_data).unwrap();
                }
                self.current_data = x;
                self.external_clock = true;
                self.transfer_requested = false;
            }
            true
        } else if !self.external_clock && !self.current_transfer
            && self.transfer_requested {
                if cycles > self.next_byte_transfer_cycle {
                    self.output.send(self.current_data).unwrap();
                    self.current_transfer = true;
                    if self.no_response {
                        self.current_data = 0;
                        self.transfer_requested = false;
                        self.next_byte_transfer_cycle = cycles + ((CPU_CLOCK_SPEED as u128) / 1024);
                        self.current_transfer = false;
                    }
                }
            false
        } else {
            false
        }
    }
}

pub struct TcpSerial {
    transfer_requested: bool,
    current_transfer: bool,
    current_data: u8,

    external_clock: bool,
    next_byte_transfer_cycle: u128,

    no_response: bool,

    input: Receiver<u8>,
    output: Sender<u8>,
}

impl TcpSerial {
    pub fn handle_stream(mut stream: TcpStream, tx: Sender<u8>, rx: Receiver<u8>) {
            let mut stream_clone = stream.try_clone().unwrap();
            thread::spawn(move || {
                loop {
                    let mut byte = [0];

                    stream_clone.read(&mut byte).unwrap();

                    tx.send(byte[0]).unwrap();
                }
            });

            thread::spawn(move || {
                for b in rx.iter() {
                    stream.write(&[b]).unwrap();
                }
            });
    }

    pub fn new_listener(port: u16, no_response: bool) -> Self {
        let (tx, input) = mpsc::channel::<u8>();
        let (output, rx) = mpsc::channel::<u8>();
        thread::spawn(move || {
            match TcpListener::bind(("0.0.0.0", port)).unwrap().accept() {
                Ok((socket, addr)) => {
                    println!("Connection on {:?}", addr);
                    Self::handle_stream(socket, tx, rx);
                }
                _ => ()
            };
        });

        Self {
            transfer_requested: false,
            current_transfer: false,
            current_data: 0,

            no_response,

            external_clock: false,
            next_byte_transfer_cycle: 0,

            input,
            output,
        }
    }

    pub fn connect(addr: String, no_response: bool) -> Self {
        let (tx, input) = mpsc::channel::<u8>();
        let (output, rx) = mpsc::channel::<u8>();
        thread::spawn(move || {
            if let Ok(socket) =  TcpStream::connect(&addr) {
                    println!("Connected to {:?}", addr);
                    Self::handle_stream(socket, tx, rx);
            }
        });

        Self {
            transfer_requested: false,
            current_transfer: false,
            current_data: 0,

            no_response,

            external_clock: false,
            next_byte_transfer_cycle: 0,

            input,
            output,
        }
    }
}

impl Serial for TcpSerial {
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
        if cycles < self.next_byte_transfer_cycle {
            return false;
        }
        if let Ok(x) = self.input.try_recv() {
            if self.current_transfer {
                self.current_data = x;
                self.external_clock = false;
                self.transfer_requested = false;
                self.next_byte_transfer_cycle = cycles + ((CPU_CLOCK_SPEED as u128) / 1024);
                self.current_transfer = false;
            } else {
                if !self.no_response || self.transfer_requested {
                    self.output.send(self.current_data).unwrap();
                }
                self.current_data = x;
                self.external_clock = true;
                self.transfer_requested = false;
            }
            self.next_byte_transfer_cycle = cycles + ((CPU_CLOCK_SPEED as u128) / 16384);
            true
        } else if !self.external_clock && !self.current_transfer
            && self.transfer_requested {
                if cycles > self.next_byte_transfer_cycle {
                    self.output.send(self.current_data).unwrap();
                    self.current_transfer = true;
                    if self.no_response {
                        self.current_data = 0;
                        self.transfer_requested = false;
                        self.next_byte_transfer_cycle = cycles + ((CPU_CLOCK_SPEED as u128) / 1024);
                        self.current_transfer = false;
                    }
                }
            false
        } else {
            false
        }
    }
}
