use std::{num::NonZero, time::Duration};

use crate::{save_data::save_data, spotify::CODE, wifi, LOCAL_URL, NVS_DEFAULT, WIFI};

use esp_idf_svc::{
    eventloop::EspSystemEventLoop, 
    hal::modem::Modem, 
    http::{
        client::{Configuration as RequestConfiguration, EspHttpConnection}, server::EspHttpServer, Method
    }, io::{
        EspIOError, 
        Write
    }, 
    mdns::EspMdns, 
    nvs::EspDefaultNvsPartition, 
    sys::{EspError, ESP_ERR_WIFI_MODE}, 
    wifi::{
        AccessPointInfo, 
        AuthMethod, 
        BlockingWifi, 
        ClientConfiguration, 
        Configuration, 
        EspWifi
    }
};

const STACK_SIZE: usize = 10240;

static SPOTIFY_SETUP_PAGE:&str = include_str!("spotify_index.html");

pub fn init_wifi(modem:Modem) -> Result<(),EspError> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    
    let wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs.clone()))?,
        sys_loop,
    )?;

    WIFI.lock().expect("WIFI should not be held elsewhere").replace(wifi);
    NVS_DEFAULT.lock().expect("NVS_DEFAULT should not be held elsewhere").replace(nvs);

    let mut m_dns = EspMdns::take()?;
    m_dns.set_hostname(LOCAL_URL)?;
    core::mem::forget(m_dns);

    start_server().expect("server should start properly");

    Ok(())
}

pub fn send_get_request(headers:&[(&str,&str)],url:&str) -> Result<EspHttpConnection,EspError>{
    println!("Get request with url: {url}");
    let config = &RequestConfiguration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let mut client = EspHttpConnection::new(&config)?;
    client.initiate_request(Method::Get, &url, &headers)?;
    client.initiate_response()?;
    Ok(client)
}

pub fn send_put_request(headers:&[(&str,&str)],url:&str,payload:&str) -> Result<EspHttpConnection,EspError>{
    println!("Put request with url: {url}");
    let config = &RequestConfiguration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let mut client = EspHttpConnection::new(&config)?;
    client.initiate_request(Method::Put, &url, &headers)?;
    client.write_all(payload.as_bytes())?;
    client.initiate_response()?;
    Ok(client)
}

pub fn send_post_request(headers:&[(&str,&str)],url:&str,payload:&str) -> Result<EspHttpConnection,EspError>{
    println!("Post request with url: {url}");
    let config = &RequestConfiguration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let mut client = EspHttpConnection::new(&config)?;
    client.initiate_request(Method::Post, &url, &headers)?;
    client.write_all(payload.as_bytes())?;
    client.initiate_response()?;
    Ok(client)
}

pub fn send_delete_request(headers:&[(&str,&str)],url:&str) -> Result<EspHttpConnection,EspError>{
    println!("Delete request with url: {url}");
    let config = &RequestConfiguration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let mut client = EspHttpConnection::new(&config)?;
    client.initiate_request(Method::Delete, &url, &headers)?;
    client.initiate_response()?;
    Ok(client)
}

fn start_server() -> Result<(),EspIOError> {
    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: STACK_SIZE,
        ..Default::default()
    };

    let mut server = EspHttpServer::new(&server_configuration)?;
    server.fn_handler::<EspIOError,_>("/", Method::Get, |req| {
        req.into_ok_response()?.write_all(SPOTIFY_SETUP_PAGE.as_bytes())?;
        Ok(())
    })?;

    server.fn_handler::<EspIOError,_>("/get_data", Method::Get, |req| {
        println!("Req is: {}",req.uri());
        //Get just the args of the request after the ?
        let args = req.uri().split("?").nth(1).unwrap();

        //Split on & to get the args
        let mut split_args = args.split("&");

        //get client id from args
        let client_id = split_args.nth(1).unwrap().split("=").nth(1).expect("There should be a value");
        //get client secret from args
        let client_secret = split_args.nth(0).unwrap().split("=").nth(1).expect("There should be a value");

        save_data("client_id", client_id)?;
        save_data("client_secret", client_secret)?;
        let new_uri = format!("https://accounts.spotify.com/authorize?{args}");
        let response = format!(
            "<!DOCTYPE html>
            <html>
                <head>
                    <title>HTML Meta Tag</title>
                    <meta http-equiv = \"refresh\" content = \"1; url = {new_uri}\" />
                </head>
                <body>
                    <p>Redirecting to Spotify Login</p>
                </body>
            </html>"
        );
        req.into_ok_response()?.write_all(response.as_bytes())?;
        Ok(())
    })?;

    server.fn_handler::<EspIOError,_>("/callback", Method::Get, |req| {
        let args = req.uri().split("?").nth(1).unwrap();
        let code = args.split("=").nth(1).unwrap().to_string();

        CODE.lock().expect("CODE is not held elsewhere").replace(code);

        req.into_ok_response()?.write_all("Code Set, Close this page".as_bytes())?;
        Ok(())
    })?;

    core::mem::forget(server);
    Ok(())
}

pub fn wifi_connected() -> Result<bool, EspError>{
    let mut wifi_lock = WIFI.lock().
    expect("WIFI should not be held elsewhere");

    let wifi = wifi_lock.as_mut()
    .expect("WIFI should be initialized");
    
    let wifi_connected = wifi.is_connected()?;

    Ok(wifi_connected)
}

pub fn _get_networks() -> Result<Vec<AccessPointInfo>,EspError> {
    let mut wifi_lock = WIFI.lock().
    expect("WIFI should not be held elsewhere");

    let wifi = wifi_lock.as_mut()
    .expect("WIFI should be initialized");

    if !wifi.is_started()? {
        wifi.start()?;
    }

    let networks = wifi.scan()?;

    Ok(networks)
}

#[derive(Debug)]
pub struct WifiCredentials{
    pub ssid:String,
    pub password:String
}

pub fn connect_to_wifi(credentials:Option<WifiCredentials>) -> Result<(), EspError>{
    let mut wifi_lock = WIFI.lock().
    expect("WIFI should not be held elsewhere");

    let wifi = wifi_lock.as_mut()
    .expect("WIFI should be initialized");

    if !wifi.is_started()? {
        println!("Started");
        wifi.start()?;
    }
    // for i in 0..5 {
    
    if let Some(creds) = &credentials {
        let mut wifi_conf = ClientConfiguration {
            ssid: creds.ssid.as_str().try_into().unwrap(),
            bssid: None,
            auth_method: AuthMethod::WPA2Personal,
            password: creds.password.as_str().try_into().unwrap(),
            channel: None,
            scan_method:esp_idf_svc::wifi::ScanMethod::FastScan,
            ..Default::default()
        };
        for wifi in wifi.scan()? {
            if wifi.ssid.to_string() == creds.ssid{
                wifi_conf.bssid = Some(wifi.bssid);
                wifi_conf.auth_method = wifi.auth_method.unwrap_or(AuthMethod::None);
                wifi_conf.channel = Some(wifi.channel);
                break;
            }
        }
        let wifi_configuration = Configuration::Client(wifi_conf);
        wifi.set_configuration(&wifi_configuration)?;
        println!("Config set: {wifi_configuration:?}");
    };

    println!("Wifi started");
    match wifi.connect() {
        Ok(_) => println!("Connected!"),
        Err(x) => match x {
            wifi_err => {
                if wifi_err == EspError::from(ESP_ERR_WIFI_MODE).unwrap() {
                    println!("Wifi mode");
                    let wifi_conf = ClientConfiguration {
                        ..Default::default()
                    };
                    let wifi_configuration = Configuration::Client(wifi_conf);
                    println!("Confing");
                    wifi.set_configuration(&wifi_configuration)?;
                    wifi.connect()?;
                }else{
                    return Err(wifi_err)
                }
                
            },
            
        }
    }
    // match wifi.connect() {
    //     Ok(_) => break,
    //     Err(e) => {
    //         if i != 4 {
    //             println!("Failed attempt {i}");
    //         }else{
    //             println!("Failed 5 attempts, exiting");
    //             return Err(e);
    //         }
    //     },
    // }
// }
   

    println!("Wifi connected");

    wifi.wait_netif_up()?;

    println!("Wifi up");

    Ok(())
}