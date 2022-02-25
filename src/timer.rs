use std::{thread, time};
use bus::Bus;
use std::io::{self, Write};

const SAVE: &str = "\x1b7";
const RESTORE: &str = "\x1b8";
const MOVE_TO_TOP_LEFT: &str = "\x1b[1;1H";
const RED: &str = "\x1b[1;31m";
const ERASE_LINE: &str = "\x1b[2K";
const RESET: &str = "\x1b[0m";

const TOTAL_TRANSACTION_TIME: u64 = 30;
const REFRESH_INTERVAL: u64 = 100;

pub struct Timer {
    bus: Bus<String>
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            bus: Bus::new(10)
        }
    }

    pub fn restart(&mut self) {
        self.stop_timer();
        self.run_timer();
    }

    pub fn stop(&mut self) {
        self.stop_timer();        
    }
    
    pub fn run_timer(&mut self) {
        let mut recv = self.bus.add_rx();
        thread::spawn(move || {
            let start = time::Instant::now();
            loop {
                if recv.try_recv().is_ok() {
                    break
                }
                let duration = TOTAL_TRANSACTION_TIME - start.elapsed().as_secs();
                if duration > 0 {
                    print!("{} {:?} {}", SAVE.to_owned() + MOVE_TO_TOP_LEFT + ERASE_LINE + RED, duration, RESTORE.to_owned() + RESET);
                    io::stdout().flush().unwrap();
                } else {
                    print!("{} {} {}", SAVE.to_owned() + MOVE_TO_TOP_LEFT + ERASE_LINE + RED, "TIMEOUT", RESTORE.to_owned() + RESET);
                    io::stdout().flush().unwrap();
                    break
                }
                thread::sleep(time::Duration::from_millis(REFRESH_INTERVAL));
            }
        });
    }

    pub fn stop_timer(&mut self) {
        self.bus.broadcast("stop".to_string());
    }
}