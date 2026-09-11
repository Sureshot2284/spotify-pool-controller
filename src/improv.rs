use esp_idf_svc::sys::EspError;

use crate::{
    uart_control::{
        read_byte_to_buffer,
        uart_write_all
    }, wifi::{self, 
        connect_to_wifi, 
        get_networks, 
    }
};

pub const VERSION: u8 = 1;

pub enum ImprovError {
    _ErrorNone = 0x00,
    ErrorInvalidRpc = 0x01,
    ErrorUnknownRpc = 0x02,
    ErrorUnableToConnect = 0x03,
    _ErrorNotAuthorized = 0x04,
    _ErrorUnknown = 0xFF,
}

pub enum State {
    StateStopped = 0x00,
    _StateAwaitingAuthorization = 0x01,
    StateAuthorized = 0x02,
    StateProvisioning = 0x03,
    StateProvisioned = 0x04,
}

pub enum Command {
    Unknown = 0x00,
    WifiSettings = 0x01,
    GetCurrentState = 0x02,
    GetDeviceInfo = 0x03,
    GetWifiNetworks = 0x04,
    BadChecksum = 0xFF,
}

pub enum ImprovSerialType {
    TypeCurrentState = 0x01,
    TypeErrorState = 0x02,
    TypeRpc = 0x03,
    TypeRpcResponse = 0x04
}

impl TryFrom<u8> for Command{    
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 =>   Ok(Command::Unknown),
            1 =>   Ok(Command::WifiSettings),
            2 =>   Ok(Command::GetCurrentState),
            3 =>   Ok(Command::GetDeviceInfo),
            4 =>   Ok(Command::GetWifiNetworks),
            255 => Ok(Command::BadChecksum),
            x => Err(format!("Bad conversion for {x} to Command Enum"))
        }
    }
}


pub struct ImprovCommand  {
    pub command: Command,
    pub ssid: String,
    pub password: String
}

pub fn parse_improv_data(data: Vec<u8>,length:usize, check_checksum:bool) -> ImprovCommand {
    let mut improv_command = ImprovCommand{
        command:Command::Unknown,
        ssid: String::new(),
        password: String::new()
    };
    let command = Command::try_from(data[0]).unwrap();
    let data_length = data[1];
    let check_checksum_val = match check_checksum {
        true => 1,
        false => 0,
    };
    if data_length != (length - 2 - check_checksum_val) as u8 {
        improv_command.command = Command::Unknown;
        return improv_command;
      }
    
      if check_checksum {
        let checksum = data[length - 1];
    
        let mut calculated_checksum:u8 = 0;
        for i in &data {
          calculated_checksum = calculated_checksum.wrapping_add(*i);
        }
    
        if calculated_checksum != checksum {
          improv_command.command = Command::BadChecksum;
          return improv_command;
        }
      }
    
      if let Command::WifiSettings = command {
        let ssid_length = data[2];
        let ssid_start = 3;
        let ssid_end = ssid_start + ssid_length;
    
        let pass_length = data[ssid_end as usize];
        let pass_start = ssid_end + 1;
        let pass_end:u8 = pass_start + pass_length;
        let ssid = String::from_utf8(data[ssid_start.into()..ssid_end.into()].into()).unwrap();
        let password = String::from_utf8(data[pass_start.into()..pass_end.into()].into()).unwrap();
        return  ImprovCommand{
            command,
            ssid,
            password
        };
      }
    
      improv_command.command = command;
      improv_command
}

pub fn parse_improve_serial_byte(position:usize, byte:u8,buffer:&[u8],callback:fn(ImprovCommand) -> bool,on_error:fn(ImprovError)) -> bool{
    let char = char::from(byte);
    match position {
        0 => return char == 'I',
        1 => return char == 'M',
        2 => return char == 'P',
        3 => return char == 'R',
        4 => return char == 'O',
        5 => return char == 'V',
        6 => return byte == VERSION,
        7 | 8 => return true,
        _ => ()
    };

    let serial_type = buffer[7];
    let data_len = buffer[8];

    if position <= (8 + data_len).into() {
        return true;
    }
    if position == (8 + data_len + 1) as usize {
        let mut checksum:u8 = 0x00;
        for i in 0..position {
            checksum = checksum.wrapping_add(buffer[i]);
        }

        if checksum != byte {
            on_error(ImprovError::ErrorInvalidRpc);
            return false;
        }
        if ImprovSerialType::TypeRpc as u8 == serial_type {
            let command = parse_improv_data(buffer[9..].to_vec(),data_len as usize, false);
            return callback(command);
        }
    }

    false
}

pub fn build_rpc_response(command:Command,datum:Vec<&str>,add_checksum:bool) -> Vec<u8> {
    let mut out = Vec::new();
    let mut length = 0;
    out.push(command as u8);
    for str in datum{
        let len = str.len();
        length += len + 1;
        out.push(len as u8);
        for char in str.chars(){
            out.push(char as u8)
        }
    }

    out.insert(1, length as u8);

    if add_checksum {
        let mut calculated_checksum:u8 = 0;
        for byte in &out{
            calculated_checksum = calculated_checksum.wrapping_add(*byte);
        }
        out.push(calculated_checksum);
    }
    out
}

//---------------------------------------

pub fn try_read_improv_data() -> Result<(),EspError> {
    let mut x_buffer = [0u8;200];
    let mut x_position = 0;
    loop{
        let mut buf = [0u8;1];
        
        if read_byte_to_buffer(&mut buf)? == 0 {
            break;
        }

        if parse_improve_serial_byte(x_position, buf[0], &x_buffer, on_command, on_error){
            x_buffer[x_position] = buf[0];
            x_position += 1;
        } else {
            x_position = 0;
        }
    }
    Ok(())
}

pub fn on_error(_err:ImprovError) {
    loop {
        
    }
}

pub fn on_command(cmd: ImprovCommand) -> bool {
    match cmd.command {
        Command::WifiSettings => {
            if !cmd.ssid.is_empty() {
                set_state(State::StateProvisioning);
                
                let creds = wifi::WifiCredentials {
                    ssid: cmd.ssid,
                    password: cmd.password
                };

                match connect_to_wifi(Some(creds)) {
                    Ok(_) => {
                        set_state(State::StateProvisioned);
                        let data = build_rpc_response(Command::WifiSettings, vec!["http://music-controller.local"], false);
                        send_response(data);
                    },
                    Err(_) => {
                        set_state(State::StateStopped);
                        set_error(ImprovError::ErrorUnableToConnect);
                    },
                }
            }else{
                set_error(ImprovError::ErrorInvalidRpc);
            }
        },
        Command::GetCurrentState => {
            if wifi::wifi_connected().unwrap() {
                set_state(State::StateProvisioned);
                let data = build_rpc_response(Command::GetCurrentState, vec!["http://music-controller.local"], false);
                send_response(data);
            }else{
                set_state(State::StateAuthorized);
            }            
        },
        Command::GetDeviceInfo => {
            let infos = vec!["music-controller","1.0.0","ESP32","Music controller for spotify"];
            let data = build_rpc_response(Command::GetDeviceInfo, infos, false);
            send_response(data);
        },
        Command::GetWifiNetworks => {
            let wifis = get_networks().expect("Scanning WIFI APs should not fail");
            for wifi in wifis{
                let auth = match wifi.auth_method {
                    Some(_) => "YES",
                    None => "NO",
                };
                let ssid = format!("{}",wifi.ssid);
                let rssi = format!("{}",wifi.signal_strength);
                let nets = vec![&ssid,&rssi,auth];
                let data = build_rpc_response(Command::GetWifiNetworks, nets, false);
                send_response(data);
            }
            
            let nets = vec![];
            let data = build_rpc_response(Command::GetWifiNetworks, nets, false);
            send_response(data);
        },
        _ => {
            set_error(ImprovError::ErrorUnknownRpc);
            return false;
        }    
    }
    
    true
}

fn send_response(mut response:Vec<u8>) {
    let mut packet = Vec::new();
    let mut improv = vec![73,77,80,82,79,86];
    packet.append(&mut improv);
    packet.push(VERSION);
    packet.push(ImprovSerialType::TypeRpcResponse as u8);
    packet.push(response.len() as u8);
    packet.append(&mut response);
    let mut sum:u8 = 0;
    for i in &packet {
        sum = sum.wrapping_add(*i);
    }
    packet.push(sum);
    uart_write_all(&packet);
}

fn set_state(state:State) {
    let mut packet = Vec::new();
    let mut improv = vec![73,77,80,82,79,86];
    packet.append(&mut improv);
    packet.push(VERSION);
    packet.push(ImprovSerialType::TypeCurrentState as u8);
    packet.push(1);
    packet.push(state as u8);
    let mut sum:u8 = 0;
    for i in &packet {
        sum = sum.wrapping_add(*i);
    }
    packet.push(sum);
    uart_write_all(&packet);
}

fn set_error(err:ImprovError) {
    let mut packet = Vec::new();
    let mut improv = vec![73,77,80,82,79,86]; //221
    packet.append(&mut improv);
    packet.push(VERSION);
    packet.push(ImprovSerialType::TypeErrorState as u8);
    packet.push(1);
    packet.push(err as u8);
    let mut sum:u8 = 0;
    for i in &packet {
        sum = sum.wrapping_add(*i)
    }
    packet.push(sum);
    uart_write_all(&packet);
}

