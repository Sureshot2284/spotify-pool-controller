#![allow(dead_code)]
use std::time::{self, Instant};

use esp_idf_svc::hal::gpio::{Input, InputPin, PinDriver, Pull};

pub struct DebouncedButton<'a, A>
where
    A: InputPin,
{
    pin: PinDriver<'a, A, Input>,
    last_press: Instant,
    button_state: bool,
    last_state: bool,
}

impl<'a, A: esp_idf_svc::hal::gpio::InputPin + esp_idf_svc::hal::gpio::OutputPin> DebouncedButton<'a, A> {
    pub fn new(pin: A) -> Self {
        let mut driver = PinDriver::input(pin).unwrap();
        driver.set_pull(Pull::Up).unwrap();
        DebouncedButton {
            pin: driver,
            last_press: time::Instant::now(),
            button_state: false,
            last_state: false,
        }
    }

    pub fn read_debounced(&mut self) -> bool {
        let mut return_val = false;
        let reading = self.pin.get_level().into();
        if reading != self.last_state {
            self.last_press = time::Instant::now();
        }

        if self.last_press.elapsed().as_millis() > 5 {
            if reading != self.button_state {
                self.button_state = reading;
                if self.button_state == false {
                    return_val = true;
                }
            }
        }

        self.last_state = reading;
        return return_val;
    }

    pub fn read_continuous(&mut self) -> bool {
        self.pin.get_level().into()
    }
}
