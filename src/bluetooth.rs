const KEYBOARD_ID: u8 = 0x01;
const MEDIA_KEYS_ID: u8 = 0x02;

const HID_REPORT_DISCRIPTOR: &[u8] = hid!(
    (USAGE_PAGE, 0x01), // USAGE_PAGE (Generic Desktop Ctrls)
    (USAGE, 0x06),      // USAGE (Keyboard)
    (COLLECTION, 0x01), // COLLECTION (Application)
    // ------------------------------------------------- Keyboard
    (REPORT_ID, KEYBOARD_ID), //   REPORT_ID (1)
    (USAGE_PAGE, 0x07),       //   USAGE_PAGE (Kbrd/Keypad)
    (USAGE_MINIMUM, 0xE0),    //   USAGE_MINIMUM (0xE0)
    (USAGE_MAXIMUM, 0xE7),    //   USAGE_MAXIMUM (0xE7)
    (LOGICAL_MINIMUM, 0x00),  //   LOGICAL_MINIMUM (0)
    (LOGICAL_MAXIMUM, 0x01),  //   Logical Maximum (1)
    (REPORT_SIZE, 0x01),      //   REPORT_SIZE (1)
    (REPORT_COUNT, 0x08),     //   REPORT_COUNT (8)
    (HIDINPUT, 0x02), //   INPUT (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (REPORT_COUNT, 0x01), //   REPORT_COUNT (1) ; 1 byte (Reserved)
    (REPORT_SIZE, 0x08), //   REPORT_SIZE (8)
    (HIDINPUT, 0x01), //   INPUT (Const,Array,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (REPORT_COUNT, 0x05), //   REPORT_COUNT (5) ; 5 bits (Num lock, Caps lock, Scroll lock, Compose, Kana)
    (REPORT_SIZE, 0x01),  //   REPORT_SIZE (1)
    (USAGE_PAGE, 0x08),   //   USAGE_PAGE (LEDs)
    (USAGE_MINIMUM, 0x01), //   USAGE_MINIMUM (0x01) ; Num Lock
    (USAGE_MAXIMUM, 0x05), //   USAGE_MAXIMUM (0x05) ; Kana
    (HIDOUTPUT, 0x02), //   OUTPUT (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    (REPORT_COUNT, 0x01), //   REPORT_COUNT (1) ; 3 bits (Padding)
    (REPORT_SIZE, 0x03), //   REPORT_SIZE (3)
    (HIDOUTPUT, 0x01), //   OUTPUT (Const,Array,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    (REPORT_COUNT, 0x06), //   REPORT_COUNT (6) ; 6 bytes (Keys)
    (REPORT_SIZE, 0x08), //   REPORT_SIZE(8)
    (LOGICAL_MINIMUM, 0x00), //   LOGICAL_MINIMUM(0)
    (LOGICAL_MAXIMUM, 0x65), //   LOGICAL_MAXIMUM(0x65) ; 101 keys
    (USAGE_PAGE, 0x07), //   USAGE_PAGE (Kbrd/Keypad)
    (USAGE_MINIMUM, 0x00), //   USAGE_MINIMUM (0)
    (USAGE_MAXIMUM, 0x65), //   USAGE_MAXIMUM (0x65)
    (HIDINPUT, 0x00),  //   INPUT (Data,Array,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (END_COLLECTION),  // END_COLLECTION
    // ------------------------------------------------- Media Keys
    (USAGE_PAGE, 0x0C),         // USAGE_PAGE (Consumer)
    (USAGE, 0x01),              // USAGE (Consumer Control)
    (COLLECTION, 0x01),         // COLLECTION (Application)
    (REPORT_ID, MEDIA_KEYS_ID), //   REPORT_ID (3)
    (USAGE_PAGE, 0x0C),         //   USAGE_PAGE (Consumer)
    (LOGICAL_MINIMUM, 0x00),    //   LOGICAL_MINIMUM (0)
    (LOGICAL_MAXIMUM, 0x01),    //   LOGICAL_MAXIMUM (1)
    (REPORT_SIZE, 0x01),        //   REPORT_SIZE (1)
    (REPORT_COUNT, 0x10),       //   REPORT_COUNT (16)
    (USAGE, 0xB5),              //   USAGE (Scan Next Track)     ; bit 0: 1
    (USAGE, 0xB6),              //   USAGE (Scan Previous Track) ; bit 1: 2
    (USAGE, 0xB7),              //   USAGE (Stop)                ; bit 2: 4
    (USAGE, 0xCD),              //   USAGE (Play/Pause)          ; bit 3: 8
    (USAGE, 0xE2),              //   USAGE (Mute)                ; bit 4: 16
    (USAGE, 0xE9),              //   USAGE (Volume Increment)    ; bit 5: 32
    (USAGE, 0xEA),              //   USAGE (Volume Decrement)    ; bit 6: 64
    (USAGE, 0x23, 0x02),        //   Usage (WWW Home)            ; bit 7: 128
    (USAGE, 0x94, 0x01),        //   Usage (My Computer) ; bit 0: 1
    (USAGE, 0x92, 0x01),        //   Usage (Calculator)  ; bit 1: 2
    (USAGE, 0x2A, 0x02),        //   Usage (WWW fav)     ; bit 2: 4
    (USAGE, 0x21, 0x02),        //   Usage (WWW search)  ; bit 3: 8
    (USAGE, 0x26, 0x02),        //   Usage (WWW stop)    ; bit 4: 16
    (USAGE, 0x24, 0x02),        //   Usage (WWW back)    ; bit 5: 32
    (USAGE, 0x83, 0x01),        //   Usage (Media sel)   ; bit 6: 64
    (USAGE, 0x8A, 0x01),        //   Usage (Mail)        ; bit 7: 128
    (HIDINPUT, 0x02), // INPUT (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (END_COLLECTION), // END_COLLECTION
);

// const MediaKeyReport KEY_MEDIA_NEXT_TRACK = {1, 0};
// const MediaKeyReport KEY_MEDIA_PREVIOUS_TRACK = {2, 0};
// const MediaKeyReport KEY_MEDIA_STOP = {4, 0};
// const MediaKeyReport KEY_MEDIA_PLAY_PAUSE = {8, 0};
// const MediaKeyReport KEY_MEDIA_MUTE = {16, 0};
// const MediaKeyReport KEY_MEDIA_VOLUME_UP = {32, 0};
// const MediaKeyReport KEY_MEDIA_VOLUME_DOWN = {64, 0};
// const MediaKeyReport KEY_MEDIA_WWW_HOME = {128, 0};
// const MediaKeyReport KEY_MEDIA_LOCAL_MACHINE_BROWSER = {0, 1}; // Opens "My Computer" on Windows
// const MediaKeyReport KEY_MEDIA_CALCULATOR = {0, 2};
// const MediaKeyReport KEY_MEDIA_WWW_BOOKMARKS = {0, 4};
// const MediaKeyReport KEY_MEDIA_WWW_SEARCH = {0, 8};
// const MediaKeyReport KEY_MEDIA_WWW_STOP = {0, 16};
// const MediaKeyReport KEY_MEDIA_WWW_BACK = {0, 32};
// const MediaKeyReport KEY_MEDIA_CONSUMER_CONTROL_CONFIGURATION = {0, 64}; // Media Selection
// const MediaKeyReport KEY_MEDIA_EMAIL_READER = {0, 128};

#[repr(u16)]
pub enum BluetoothMediaKeys {
    _PlayPause = u16::from_be_bytes([8, 0]),
    VolumeUp = u16::from_be_bytes([32, 0]),
    VolumeDown = u16::from_be_bytes([64, 0]),
}

use std::sync::Arc;

use esp32_nimble::{
    enums::*, hid::*, utilities::mutex::Mutex, BLEAdvertisementData, BLECharacteristic, BLEDevice,
    BLEError, BLEHIDDevice, BLEServer,
};

pub struct BleController {
    server: &'static mut BLEServer,
    input_media_keys: Arc<Mutex<BLECharacteristic>>,
}

use crate::BLUETOOTH;

pub fn init_bluetooth() -> Result<(), BLEError> {
    let device = BLEDevice::take();
    device
        .security()
        .set_auth(AuthReq::Bond)
        .set_passkey(123456)
        .set_io_cap(SecurityIOCap::NoInputNoOutput)
        .resolve_rpa();

    BLEDevice::set_device_name("Music Controller")?;

    let server = device.get_server();
    let mut hid = BLEHIDDevice::new(server);

    let input_media_keys = hid.input_report(MEDIA_KEYS_ID);

    hid.manufacturer("Espressif");
    hid.pnp(0x02, 0x05ac, 0x820a, 0x0210);
    hid.hid_info(0x00, 0x01);

    hid.report_map(HID_REPORT_DISCRIPTOR);

    hid.set_battery_level(100);

    let ble_advertising = device.get_advertising();
    ble_advertising.lock().scan_response(false).set_data(
        BLEAdvertisementData::new()
            .name("Music Controller")
            .appearance(0x03C1)
            .add_service_uuid(hid.hid_service().lock().uuid()),
    )?;

    ble_advertising.lock().start()?;
    let ble_controller = BleController {
        server,
        input_media_keys,
    };
    BLUETOOTH
        .lock()
        .expect("BLUETOOTH should not be held elsewhere")
        .replace(ble_controller);

    Ok(())
}

pub fn bluetooth_connected() -> bool {
    let bluetooth_lock = BLUETOOTH
        .lock()
        .expect("BLUETOOTH should not be held elsewhere");
    let bluetooth = bluetooth_lock
        .as_ref()
        .expect("BLUETOOTH should be initiated");

    bluetooth.server.connected_count() > 0
}

pub fn _send_command(cmd: &[u8]) {
    BLUETOOTH
        .lock()
        .expect("BLUETOOTH should not be held elsewhere")
        .as_ref()
        .expect("BLUETOOTH should be initiated")
        .input_media_keys
        .lock()
        .set_value(cmd)
        .notify();
}

pub fn send_media_command(cmd: BluetoothMediaKeys) {
    let bl_lock = BLUETOOTH
        .lock()
        .expect("BLUETOOTH should not be held elsewhere");

    let bl = bl_lock.as_ref().expect("BLUETOOTH should be initiated");

    bl.input_media_keys
        .lock()
        .set_value(&(cmd as u16).to_be_bytes())
        .notify();
    bl.input_media_keys.lock().set_value(&[0,0]).notify();
}
