use crate::UART;

use esp_idf_svc::{
    hal::{
        gpio::{
            AnyIOPin, 
            AnyInputPin, 
            AnyOutputPin
        }, uart::{
            self, 
            UART0,
        }, units::Hertz
    }, io::Write, 
    sys::EspError
};

pub fn init_uart(uart:UART0,tx:AnyOutputPin,rx:AnyInputPin) -> Result<(),EspError> {
    let config = uart::config::Config::default().baudrate(Hertz(115_200));

    let uart: uart::UartDriver = uart::UartDriver::new(
        uart,
        tx,
        rx,
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &config
    ).expect("UART should be initialized");

    UART.lock().expect("UART should not be held elsewhere").replace(uart);
    Ok(())
}

pub fn read_byte_to_buffer(buf:&mut [u8]) -> Result<usize,EspError> {
    
    let mut uart_lock = UART
    .lock()
    .expect("UART should not be held elsewhere");

    let uart = uart_lock
    .as_mut()
    .expect("UART should be initialized");
    let read_len = uart.read(buf,1)?;
    Ok(read_len)
}

pub fn _uart_write_all(buf:&[u8]) {
    let mut uart_lock = UART
    .lock()
    .expect("UART should not be held elsewhere");

    let uart = uart_lock
    .as_mut()
    .expect("UART should be initialized");

    uart.write_all(buf).expect("Writing to UART should not fail");
}

pub fn _uart_available() -> bool {
    let mut uart_lock = UART
    .lock()
    .expect("UART should not be held elsewhere");

    let uart = uart_lock
    .as_mut()
    .expect("UART should be initialized");

    uart.remaining_read().expect("Should be able to get bytes available for UART to read") > 0
}