use std::{collections::HashMap, sync::Mutex, thread, time::Instant};

use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};

use esp_idf_svc::{
    hal::{gpio::PinDriver, prelude::Peripherals, uart::UartDriver},
    log::EspLogger,
    nvs::{EspNvsPartition, NvsDefault},
    sys::{esp_log_level_set, esp_task_wdt_deinit, EspError, ESP_ERR_HTTP_EAGAIN, LOG_LEVEL_WARN},
    wifi::{BlockingWifi, EspWifi},
};

use std::ffi::CString;

mod debounced_button;

mod wifi;
use wifi::{connect_to_wifi, init_wifi, wifi_connected, WifiCredentials};

mod save_data;
use save_data::{delete_data, read_data};

mod spotify;
use spotify::{
    can_control_vol, code_is_set, get_current_song, get_playback_state, init_spotify, set_playback_state, SpotifyController, _change_volume, _get_liked_state, _set_liked_state, _skip_track, start_song_radio
};

mod cyd_display;
use cyd_display::{
    clear_display, draw_centered_text, draw_image, draw_progress_bar, draw_song_info,
    fade_in_backlight, fade_out_backlight, init_display, set_brightness,
};

// mod improv;
// use improv::try_read_improv_data;

mod uart_control;
use uart_control::{init_uart, read_byte_to_buffer};

mod bluetooth;
use bluetooth::{bluetooth_connected, init_bluetooth, send_media_command, BluetoothMediaKeys};

struct PlaybackControls {
    play_pause: bool,
    _next: bool,
    _prev: bool,
    supports_vol: bool,
    like: bool,
    volume_change: i8,
    start_radio: bool,
}

static PLAYBACK_CONTROLS: Mutex<PlaybackControls> = Mutex::new(PlaybackControls {
    play_pause: false,
    _next: false,
    _prev: false,
    supports_vol: true,
    volume_change: 0,
    like: false,
    start_radio: false
});

static WIFI: Mutex<Option<BlockingWifi<EspWifi>>> = Mutex::new(None);
static NVS_DEFAULT: Mutex<Option<EspNvsPartition<NvsDefault>>> = Mutex::new(None);
static DISPLAY: Mutex<Option<cyd_display::CydDisplay>> = Mutex::new(None);
static UART: Mutex<Option<UartDriver>> = Mutex::new(None);
static BLUETOOTH: Mutex<Option<bluetooth::BleController>> = Mutex::new(None);
static SPOTIFY: Mutex<Option<spotify::SpotifyController>> = Mutex::new(None);

const LOCAL_URL: &str = "music-controller";

// #define LED_RED 4
// #define LED_GREEN 17
// #define LED_BLUE 16

fn main() -> Result<(), EspError> {
    let peripherals = init_svc()?;
    init_wifi(peripherals.modem)?;
    init_display(
        peripherals.spi2,
        peripherals.pins.gpio23.into(),
        peripherals.pins.gpio2.into(),
        peripherals.pins.gpio21.into(),
        peripherals.ledc.channel0,
        peripherals.ledc.timer0,
        peripherals.pins.gpio14,
        peripherals.pins.gpio13,
        peripherals.pins.gpio12,
        peripherals.pins.gpio15,
    )?;
    init_uart(
        peripherals.uart0,
        peripherals.pins.gpio1.into(),
        peripherals.pins.gpio3.into(),
    )?;

    let mut spotify_initiated = false;
    let mut wifi_started = false;
    set_brightness(5)?;
    clear_display(Rgb565::BLACK)?;

    let _bad_creds = WifiCredentials {
        ssid: String::from("Bad"),
        password: String::from("No Good"),
    };

    draw_centered_text("Setting up wifi...\n Hang Tight!!");

    match connect_to_wifi(None) {
        Ok(_) => {
            println!("WIFI Connected!");
            clear_display(Rgb565::BLACK)?;
            wifi_started = true;
            let mut token_buf = [0u8; 1000];

            if let Some(token) =
                read_data("refresh_token", &mut token_buf).expect("Reading data should not fail")
            {
                match SpotifyController::make_controller_from_token(token) {
                    Ok(spotify_controller) => {
                        draw_centered_text("WIFI and spotify connected!\nStarting system!");
                        println!("controller info:{:?}", spotify_controller);
                        SPOTIFY
                            .lock()
                            .expect("SPOTIFY should not be held elsewhere")
                            .replace(spotify_controller);
                        spotify_initiated = true;
                    }
                    Err(e) => {
                        println!("Got err {e}, could not refresh spotify token");
                        draw_centered_text(&format!("Wifi connected but spotify token did not work :(\nGo to {LOCAL_URL}.local to get set up!"));
                    }
                };
            } else {
                draw_centered_text(&format!("Wifi connected but could not connect to spotify :(\nGo to {LOCAL_URL}.local to get set up!"));
            }
        }
        Err(err) => {
            println!("WIFI Could not connect: {}", err);
            clear_display(Rgb565::BLACK)?;
            draw_centered_text(&format!(
                "Could not connect to wifi :(\nGo to webserial.io on\nedge or chrome to get set up!"
            ));
        }
    };

    init_bluetooth().expect("BLUETOOTH initialization should be successful");

    if false {
        delete_data("auth_token")?;
        delete_data("refresh_token")?;
        delete_data("client_id")?;
        delete_data("client_secret")?;
    }

    let mut last_title = String::new();

    let mut pin_a = debounced_button::DebouncedButton::new(peripherals.pins.gpio22);
    let mut pin_b = debounced_button::DebouncedButton::new(peripherals.pins.gpio27);
    let mut play_button = debounced_button::DebouncedButton::new(peripherals.pins.gpio10);
    let mut forward_button = debounced_button::DebouncedButton::new(peripherals.pins.gpio4);
    let mut backward_button = debounced_button::DebouncedButton::new(peripherals.pins.gpio17);
    let mut like_button = debounced_button::DebouncedButton::new(peripherals.pins.gpio16);
    let mut radio_button = debounced_button::DebouncedButton::new(peripherals.pins.gpio9);
    let mut last_astate = pin_a.read_continuous();

    let mut should_pause = false;
    let mut should_next = false;
    let mut should_prev = false;
    let mut start_radio = false;
    let mut should_like = false;
    let mut should_vol_up = false;
    let mut should_vol_down = false;
    thread::spawn(move || {
        loop {
            //BLUETOOTH Control

            let a_state = pin_a.read_continuous();
            if a_state != last_astate {
                if pin_b.read_continuous() != a_state {
                    should_vol_up = true;
                } else {
                    should_vol_down = true;
                }
            }
            last_astate = a_state;
            if play_button.read_debounced() {
                should_pause = true;
            }
            if forward_button.read_debounced(){
                should_next = true;
            }
            if backward_button.read_debounced(){
                should_prev = true;
            }
            if like_button.read_debounced() {
                should_like = true;
            }
            if radio_button.read_debounced(){
                start_radio = true;
            }
            if should_pause {
                println!("Adding pause");
                let play_pause = &mut PLAYBACK_CONTROLS.lock().unwrap().play_pause;
                *play_pause = !*play_pause;
                should_pause = false;
            }
            if should_vol_up {
                if bluetooth_connected() && !PLAYBACK_CONTROLS.lock().unwrap().supports_vol {
                    send_media_command(BluetoothMediaKeys::VolumeUp);
                } else {
                    PLAYBACK_CONTROLS.lock().unwrap().volume_change += 1;
                }
                should_vol_up = false;
            }
            if should_vol_down {
                if bluetooth_connected() && !PLAYBACK_CONTROLS.lock().unwrap().supports_vol {
                    send_media_command(BluetoothMediaKeys::VolumeDown);
                } else {
                    PLAYBACK_CONTROLS.lock().unwrap().volume_change -= 1;
                }
                should_vol_down = false;
            }
            if should_next {
                PLAYBACK_CONTROLS.lock().unwrap()._next = true;
                should_next = false;
            }
            if should_like {
                PLAYBACK_CONTROLS.lock().unwrap().like = true;
                should_like = false;
            }
            if should_prev {
                PLAYBACK_CONTROLS.lock().unwrap()._prev = true;
                should_prev = false;
            }
            if start_radio {
                PLAYBACK_CONTROLS.lock().unwrap().start_radio = true;
                start_radio = false;
            }
        }
    });

    // clear_display(Rgb565::BLACK)?;
    let mut song_info_timer = Instant::now();
    // unsafe{
    //     println!("Free heap: {}",esp_get_free_heap_size());
    //     println!("Free heap internal: {}",esp_get_free_internal_heap_size());
    //     println!("Free heap min: {}",esp_get_minimum_free_heap_size());
    // }

    let mut last_progress = 0f32;
    let mut print_wifi_msg = !wifi_started;
    loop {
        if print_wifi_msg && !wifi_connected()? {
            println!("Wifi not connected, enter your wifi below in the form ssid:password");
            print_wifi_msg = false;
        }
        //IMPROV WiFi Check
        let mut buf = [0u8;1];
        let mut serial_string = String::new();
        while read_byte_to_buffer(&mut buf)? != 0 {
            serial_string.push(buf[0] as char);
        }
        if serial_string.len() > 2 {
            println!("Got serial string {serial_string}");
            let mut parts = serial_string.split(":");
            let Some(ssid) = parts.next() else {
                println!("Did not get ssid");
                print_wifi_msg = true;
                continue;
            };
            let Some(mut pass) = parts.next() else {
                println!("Did not get password");
                print_wifi_msg = true;
                continue;
            };
            println!("pass no trim: {pass:?} ,with trim {}",pass.trim_end().trim_end());
            pass = pass.trim_end();
            
            let creds = WifiCredentials {
                ssid: ssid.to_string(),
                password: pass.to_string(),
            };
            println!("Got creds: {creds:?}");
            match connect_to_wifi(Some(creds)) {
                Ok(_) => {
                    println!("Wifi is connected");
                },
                Err(e) => {
                    println!("Wifi did not connect with err:{e}");
                    print_wifi_msg = true;
                },
            }
        }
        
        if wifi_connected()? && !spotify_initiated {
            clear_display(Rgb565::BLACK)?;
            let mut token_buf = [0u8; 1000];

            if let Some(token) =
                read_data("refresh_token", &mut token_buf).expect("Reading data should not fail")
            {
                match SpotifyController::make_controller_from_token(token) {
                    Ok(spotify_controller) => {
                        clear_display(Rgb565::BLACK)?;
                        draw_centered_text("WIFI and spotify connected!\nStarting system!");
                        println!("controller info:{:?}", spotify_controller);
                        SPOTIFY
                            .lock()
                            .expect("SPOTIFY should not be held elsewhere")
                            .replace(spotify_controller);
                        spotify_initiated = true;
                    },
                    Err(e) => println!("Could not make controller from token:{e}"),
                }
            }
            if !spotify_initiated && code_is_set() {
                init_spotify()?;
                draw_centered_text("WIFI and spotify connected!\nStarting system!");
                spotify_initiated = true;
            }
            if !spotify_initiated {
                draw_centered_text(&format!("Wifi connected but spotify token did not work :(\nGo to {LOCAL_URL}.local to get set up!"));
            }
        }

        if spotify_initiated && song_info_timer.elapsed().as_millis() > 500 {
            match get_current_song() {
                Ok(response) => {
                    if let Some(info) = response {
                        if info.title != last_title {
                            fade_out_backlight()?;
                            last_title = info.title;
                            println!("Title: {}", last_title);
                            println!("Album: {}", info.album);
                            println!("Artist: {}", info.artist);
                            println!(
                                "Elapsed: {}%",
                                info.progress_ms as f32 / info.duration_ms as f32 * 100.0
                            );
                            // clear_display(Rgb565::BLACK)?;
                            while let Err(x) = draw_image(&info.url) {
                                println!("Err while drawing image:{x}");
                            }
                            // clear_text_from_display(Rgb565::BLACK)?;
                            draw_song_info(&last_title, &info.album, &info.artist)?;
                            draw_progress_bar(
                                info.progress_ms as f32 / info.duration_ms as f32,
                                true,
                            )?;
                            fade_in_backlight()?;
                        }
                        println!(
                            "Elapsed: {}%",
                            info.progress_ms as f32 / info.duration_ms as f32 * 100.0
                        );
                        let new_progress = info.progress_ms as f32 / info.duration_ms as f32;
                        draw_progress_bar(new_progress, new_progress < last_progress)?;
                        last_progress = new_progress;
                    }
                }
                Err(e) => println!("Got error while requesting info: {e}"),
            }
            PLAYBACK_CONTROLS.lock().unwrap().supports_vol = can_control_vol();
            song_info_timer = Instant::now();
        }

        let mut control_lock = PLAYBACK_CONTROLS.lock().unwrap();
        if control_lock.play_pause {
            match set_playback_state(!get_playback_state()) {
                Ok(_) => (),
                Err(x) => {
                    if x != EspError::from_infallible::<ESP_ERR_HTTP_EAGAIN>() {
                        return Err(x);
                    }
                }
            }
            control_lock.play_pause = false;
        }
        if control_lock.volume_change != 0 {
            println!("New_vol:{}", control_lock.volume_change);
            let _ =_change_volume(control_lock.volume_change.into());
            control_lock.volume_change = 0;
        }
        if control_lock._next {
            let _ = _skip_track(true);
            control_lock._next = false;
        }
        if control_lock._prev {
            let _ = _skip_track(false);
            control_lock._prev = false;
        }
        if control_lock.like {
            let _ = _set_liked_state(!_get_liked_state());
            control_lock.like = false;
        }
        if control_lock.start_radio{
            let _ = start_song_radio(&last_title);
            control_lock.start_radio = false;
        }
    }
}

fn init_svc() -> Result<Peripherals, EspError> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    unsafe {
        //Stopping the watchdog allows us to run the main loop as fast as possible
        if esp_task_wdt_deinit() != 0 {
            panic!("Could not stop watchdog timer!!")
        }
        let log_string = CString::new("*").unwrap();
        esp_log_level_set(log_string.as_ptr(), LOG_LEVEL_WARN);
    }
    Ok(Peripherals::take()?)
}

