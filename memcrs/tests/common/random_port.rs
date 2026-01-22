use lazy_static::lazy_static;

use std::sync::Mutex;

const STARTING_PORT: u16 = 10000;
pub struct PseudoRandomMemcrsdPort {
    port: u16,
}

impl PseudoRandomMemcrsdPort {
    pub fn new() -> PseudoRandomMemcrsdPort {
        PseudoRandomMemcrsdPort {
            port: STARTING_PORT,
        }
    }

    pub fn get_next_port(&mut self) -> u16 {
        self.port += 10;
        self.port
    }
}

lazy_static! {
    pub static ref pseudoRanomPort: Mutex<PseudoRandomMemcrsdPort> =
        Mutex::new(PseudoRandomMemcrsdPort::new());
}
