use std::{sync::Mutex, time::Instant};

use base64::{prelude::BASE64_STANDARD, Engine};
use esp_idf_svc::{http::{client::{Configuration, EspHttpConnection}, Method}, sys::{EspError, ESP_ERR_INVALID_RESPONSE, ESP_ERR_NOT_FOUND}};
use serde_json::Value;

use crate::{save_data::{read_data, save_data}, wifi::{send_delete_request, send_get_request, send_post_request, send_put_request}, SPOTIFY};

pub static CODE:Mutex<Option<String>> = Mutex::new(None);

pub struct SongInfo{
    pub title: String,
    pub artist: String,
    pub album: String,
    pub url: String,
    pub progress_ms: usize,
    pub duration_ms: usize
}

enum SearchResult{
    playlist,
    album,
}

#[derive(Debug)]
pub struct SpotifyController{
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_timer: Instant,
    pub refresh_timeout: u64,
    pub current_vol: Option<u8>,
    pub can_control_vol: bool,
    pub is_playing: bool,
    pub shuffle_state: bool,
    pub currently_playing_id: String,
}

impl SpotifyController{

    pub fn make_controller_from_token(token:&str) -> Result<SpotifyController,EspError> {
        let mut spotify_controller = SpotifyController {
            access_token: String::new(),
            refresh_token: token.to_string(),
            refresh_timer: Instant::now(),
            refresh_timeout: 0,
            current_vol: None,
            can_control_vol: true,
            is_playing: true,
            shuffle_state: false,
            currently_playing_id: String::new(),
        };
        spotify_controller.refresh_token()?;
        Ok(spotify_controller)
    }

    pub fn get_current_song(&mut self) -> Result<Option<SongInfo>,EspError>{
        if self.refresh_timer.elapsed().as_secs() >= self.refresh_timeout{
            self.refresh_token()?;
        }
        
        let auth = format!("Bearer {}",self.access_token);
        let headers = [
            ("Authorization", auth.as_str())
        ];
        let url = "https://api.spotify.com/v1/me/player";

        let mut client = send_get_request(&headers, &url)?;

        let resp = client.status();
        
        if resp == 200 {
            let mut info = SongInfo{
                title: String::new(),
                artist: String::new(),
                album: String::new(),
                url: String::new(),
                progress_ms: 0,
                duration_ms: 0,
            };

            let device_name = get_val(&mut client, "name",'\n')?;
            let support_vol = get_val(&mut client, "supports_volume",'\n')?;
            println!("Supports_vol:{support_vol}");
            self.can_control_vol = support_vol.parse().expect("Returned value is a bool");
            let curr_vol = get_val(&mut client, "volume_percent",'\n')?;
            println!("Current_vol:{curr_vol}");
            if None == self.current_vol{
                self.current_vol = Some(curr_vol.parse().expect("returned value is a number"));
            }
            
            
            let shuffle_state = get_val(&mut client, "shuffle_state",'\n')?.parse().expect("Returned value is a bool");
            self.shuffle_state = shuffle_state;

            info.progress_ms = get_val(&mut client, "progress_ms",'\n')?.parse().expect("returned value is a number");
            
            



            println!("Device:{device_name}");
            println!("Vol support:{support_vol}");

            let _ = get_val(&mut client, "height",'\n'); //skip to first height info
            let _ = get_val(&mut client, "url",'\n'); //skip first url

            info.url = remove_quotes(get_val(&mut client, "url",'\n')?);
            info.album = remove_quotes(get_val(&mut client, "name",'\n')?);
            info.artist = remove_quotes(get_val(&mut client, "name",'\n')?);
            info.duration_ms = get_val(&mut client, "duration_ms",'\n')?.parse().expect("returned value is a number");

            let _ = get_val(&mut client, "external_urls",'\n')?;

            let song_id = get_val(&mut client, "id",'\n')?;
            println!("Song_id:{song_id}");
            self.currently_playing_id = remove_quotes(song_id);

            info.title = remove_quotes(get_val(&mut client, "name",'\n')?);
            let is_playing = get_val(&mut client, "is_playing",'\n')?.parse().expect("Returned value is a bool");
            self.is_playing = is_playing;

            return Ok(Some(info));
        }else{
            println!("Got err code {}",resp);
            self.current_vol = None;
            return Ok(None);
        }
    }

    pub fn get_liked_state(&mut self) -> Result<bool,EspError>{
        if self.refresh_timer.elapsed().as_secs() >= self.refresh_timeout{
            self.refresh_token()?;
        }
        
        let auth = format!("Bearer {}",self.access_token);
        let headers = [
            ("Authorization", auth.as_str()),
            ("Content-Type","application/json")
        ];
        let url = "https://api.spotify.com/v1/me/tracks/contains?ids=".to_string() + &self.currently_playing_id;

        let mut client = send_get_request(&headers, &url)?;
        let mut buf = [0u8;25];
        client.read(&mut buf)?;
        let resp = String::from_utf8(buf.to_vec()).unwrap();

        println!("Resp: {resp:?}");
        println!("resp comp is: {}",resp.trim_matches(char::from(0)) == "[true]");
        Ok(resp.trim_matches(char::from(0)) == "[true]")
    }

    pub fn set_liked_state(&mut self,state:bool) -> Result<(),EspError> {
        if self.refresh_timer.elapsed().as_secs() >= self.refresh_timeout{
            self.refresh_token()?;
        }
        
        let auth = format!("Bearer {}",self.access_token);
        let headers = [
            ("Authorization", auth.as_str())
        ];
        println!("Song_id:{}",self.currently_playing_id);
        let url = "https://api.spotify.com/v1/me/tracks?ids=".to_string() + &self.currently_playing_id;
        if state{
            send_put_request(&headers, &url,"")?;
        }else {
            send_delete_request(&headers, &url)?;
        }
        Ok(())
    }

    pub fn set_playback_state(&mut self,play:bool) -> Result<(),EspError> {
        if self.refresh_timer.elapsed().as_secs() >= self.refresh_timeout{
            self.refresh_token()?;
        }
        
        let auth = format!("Bearer {}",self.access_token);
        let headers = [
            ("Authorization", auth.as_str())
        ];
        let url = "https://api.spotify.com/v1/me/player/".to_string() + if play { "play" } else { "pause" };

        let client = send_put_request(&headers, &url,"")?;
        let resp = client.status();
        if resp == 200 {
            println!("Succesful query");
        }else{
            println!("Got err code {}",resp);
            return Err(EspError::from_non_zero(ESP_ERR_NOT_FOUND.try_into().unwrap()));
        }
        Ok(())
    }

    pub fn set_shuffle_state(&mut self,state:bool) -> Result<(),EspError> {
        if self.refresh_timer.elapsed().as_secs() >= self.refresh_timeout{
            self.refresh_token()?;
        }
        
        let auth = format!("Bearer {}",self.access_token);
        let headers = [
            ("Authorization", auth.as_str())
        ];
        let url = "https://api.spotify.com/v1/me/player/shuffle?state=".to_string() + if state { "true" } else { "false" };

        send_put_request(&headers, &url,"")?;

        Ok(())
    }

    pub fn skip_track(&mut self, forward:bool) -> Result<(),EspError> {
        if self.refresh_timer.elapsed().as_secs() >= self.refresh_timeout{
            self.refresh_token()?;
        }
        
        let auth = format!("Bearer {}",self.access_token);
        let headers = [
            ("Authorization", auth.as_str())
        ];
        let url = "https://api.spotify.com/v1/me/player/".to_string() + if forward { "next" } else { "previous" };

        send_post_request(&headers, &url,"")?;
        Ok(())
    }

    pub fn change_volume(&mut self, up:i32) -> Result<(),EspError> {
        if self.refresh_timer.elapsed().as_secs() >= self.refresh_timeout{
            self.refresh_token()?;
        }
        
        let auth = format!("Bearer {}",self.access_token);
        let headers = [
            ("Authorization", auth.as_str())
        ];
        let mut curr_vol = self.current_vol.expect("Volume should be parsed");
        if up > 0 {
            curr_vol = 100.min(curr_vol as i32 + up) as u8;
        }else{
            curr_vol = 0.max(curr_vol as i32 + up) as u8;
        }
        self.current_vol = Some(curr_vol);
        
        let url = format!("https://api.spotify.com/v1/me/player/volume?volume_percent={}",curr_vol);
        

        send_put_request(&headers, &url,"")?;
        Ok(())
    }

    pub fn search(&mut self,title:&str) -> Result<(SearchResult,String),EspError>{
        if self.refresh_timer.elapsed().as_secs() >= self.refresh_timeout{
            self.refresh_token()?;
        }
        
        let auth = format!("Bearer {}",self.access_token);
        let headers = [
            ("Authorization", auth.as_str())
        ];
        let url = format!("https://api.spotify.com/v1/search?q={}+radio&type=playlist",title.replace(" ", "+").replace("\"", "").replace("\\", ""));

        let mut client = send_get_request(&headers, &url)?;

        let resp = client.status();
        if resp == 200 {
            println!("Succesful query");
            let mut playlist_id = get_val(&mut client, "id",',')?;
            playlist_id = playlist_id.trim_end_matches("\"").to_string();
            if playlist_id.len() > 0 {
                println!("Playlist id: {playlist_id}");
                return Ok((SearchResult::playlist,playlist_id));
            }
        }else{
            println!("Got err code {}",resp);
            return Err(EspError::from_non_zero(ESP_ERR_NOT_FOUND.try_into().unwrap()));
        }

        let url = format!("https://api.spotify.com/v1/search?q={}&type=album",title.replace(" ", "+").replace("\"", "").replace("\\", ""));

        let mut client = send_get_request(&headers, &url)?;

        let resp = client.status();
        if resp == 200 {
            println!("Succesful query");
            let mut playlist_id = get_val(&mut client, "id",',')?;
            playlist_id = playlist_id.trim_end_matches("\"").to_string();
            if playlist_id.len() > 0 {
                println!("Playlist id: {playlist_id}");
                return Ok((SearchResult::album,playlist_id));
            }else{
                println!("Could not find playlist or album");
                return Err(EspError::from_non_zero(ESP_ERR_NOT_FOUND.try_into().unwrap()));
            }
        }else{
            println!("Got err code {}",resp);
            return Err(EspError::from_non_zero(ESP_ERR_NOT_FOUND.try_into().unwrap()));
        }
    }

    pub fn start_song(&mut self,id:String,typ:SearchResult) -> Result<(),EspError> {
        if self.refresh_timer.elapsed().as_secs() >= self.refresh_timeout{
            self.refresh_token()?;
        }
        
        let auth = format!("Bearer {}",self.access_token);
        let payload = match typ {
            SearchResult::playlist => format!("{{\"context_uri\": \"spotify:playlist:{id}\"}}"),
            SearchResult::album => format!("{{\"context_uri\": \"spotify:album:{id}\"}}")
        };
        let headers = [
            ("Authorization", auth.as_str()),
            ("content-type","application/json"),
            ("content-length",&payload.len().to_string())
        ];
        let url = "https://api.spotify.com/v1/me/player/play";

        let mut client = send_put_request(&headers, &url,&payload)?;
        let resp = client.status();
        
        if resp == 204 {
            println!("Succesful query");
        }else{
            println!("Got err code {}",resp);
            let mut buf = [0u8;25];
            while client.read(&mut buf)? != 0{
                let resp = String::from_utf8(buf.to_vec()).unwrap();

                println!("Resp: {resp:?}");
            };
            return Err(EspError::from_non_zero(ESP_ERR_NOT_FOUND.try_into().unwrap()));
        }
        Ok(())
    }

    pub fn refresh_token(&mut self) -> Result<(),EspError>{
        let payload = format!("grant_type=refresh_token&refresh_token={}",self.refresh_token);
        let content_length_header = format!("{}", payload.len());

        let mut client_buf = [0u8;1000];
        let mut secret_buf = [0u8;1000];
        let client_id = read_data("client_id", &mut client_buf)?.expect("client_id is initialized");
        let client_secret = read_data("client_secret", &mut secret_buf)?.expect("client_secret is initialized");
        let auth = format!("Basic {}",BASE64_STANDARD.encode(format!("{}:{}",client_id,client_secret)));

        let headers = [
            ("Authorization",auth.as_str()),
            ("content-type", "application/x-www-form-urlencoded"),
            ("content-length", &*content_length_header),
        ];
        let url = "https://accounts.spotify.com/api/token";
        println!("Requesting new token from spotify");
        let mut client = send_post_request(&headers, &url, &payload)?;

        let mut buf = [0u8; 1024];
        let read_len = client.read(&mut buf)?;

        let json_response = std::str::from_utf8(&buf[0..read_len]).map_err(|_| EspError::from_infallible::<ESP_ERR_INVALID_RESPONSE>())?;
        let response:Value = serde_json::from_str(json_response).map_err(|_| EspError::from_infallible::<ESP_ERR_INVALID_RESPONSE>() )?;

        let refresh = response["refresh_token"].to_string();
        let mut refresh = refresh.chars();
        refresh.next();
        refresh.next_back();

        let token = response["access_token"].to_string();
        let mut token = token.chars();
        token.next();
        token.next_back();
        self.access_token = token.collect();
        println!("Got token:{} and refresh:{}",self.access_token,refresh.collect::<String>());
        // self.refresh_token = refresh.collect();
        self.refresh_timeout = response["expires_in"].to_string().parse().map_err(|_| EspError::from_infallible::<ESP_ERR_NOT_FOUND>())?;
        self.refresh_timer = Instant::now();
        Ok(())
    }
    
}

pub fn code_is_set() -> bool{
    
    let code_is_valid = CODE.lock()
    .expect("CODE is not held elsewhere")
    .is_some();
    if code_is_valid {
        println!("Code is valid");
    }
    code_is_valid
}

pub fn init_spotify() -> Result<(),EspError>{
    println!("Initing spotify");
    let config = &Configuration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };
    let mut client = EspHttpConnection::new(&config)?;

    let code = CODE.lock()
    .expect("CODE is not held elsewhere")
    .as_ref()
    .expect("CODE should be initialized")
    .clone();

    let (access_token,refresh_token,refresh_timeout) = post_spotify(&mut client, &code)?;
    let spotify_controller = SpotifyController{
        access_token,
        refresh_token,
        refresh_timer:Instant::now(),
        refresh_timeout,
        current_vol:None,
        can_control_vol:true,
        is_playing: true,
        shuffle_state: false,
        currently_playing_id: String::new()
    };
    SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere").replace(spotify_controller);
    Ok(())
}

pub fn can_control_vol() -> bool{
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");

    spotify.can_control_vol
}

pub fn get_current_song() -> Result<Option<SongInfo>,EspError> {
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
   
    spotify.get_current_song()
}

pub fn get_playback_state() -> bool{
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
    spotify.is_playing
}

pub fn _get_shuffle_state() -> bool{
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
    spotify.shuffle_state
}

pub fn _set_shuffle_state(state:bool) -> Result<(), EspError>{
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
    spotify.set_shuffle_state(state)
}

pub fn _get_current_song_id() -> String{
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
    spotify.currently_playing_id.clone()
}

pub fn _get_liked_state() -> bool {
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
    spotify.get_liked_state().unwrap()
}

pub fn _set_liked_state(state:bool) -> Result<(),EspError> {
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
    spotify.set_liked_state(state)
}

pub fn set_playback_state(play:bool) -> Result<(),EspError> {
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
   
    spotify.set_playback_state(play)
}

pub fn _skip_track(forward:bool) -> Result<(),EspError> {
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
   
    spotify.skip_track(forward)
}

pub fn _change_volume(up:i32) -> Result<(),EspError> {
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
    if let Some(_) = spotify.current_vol{
        return spotify.change_volume(up);
    }
    Ok(())
}

pub fn start_song_radio(title:&str) -> Result<(),EspError>{
    let mut spotify_lock = SPOTIFY.lock().expect("SPOTIFY should not be held elsewhere");
    let spotify = spotify_lock.as_mut().expect("SPOTIFY should be instantiated");
    let id = spotify.search(title)?;
    spotify.start_song(id.1,id.0)
}

fn remove_quotes(input:String) -> String{
    let mut chars = input.chars();
    chars.next();
    chars.next_back();
    chars.collect()
}

fn get_val(client:&mut EspHttpConnection,key:&str,line_end:char) -> Result<String,EspError>{
    let mut look = false;
    let mut found = false;
    let mut seek = true;

    let key_chars:Vec<char> = key.chars().collect();

    let mut ind = 0;
    let mut ret_str = Vec::new();

    let mut buf = [0u8; 1];
    let mut read_len = client.read(&mut buf)?;
    // println!("Read {} bytes",read_len);
    while read_len != 0 {
        // println!("current char: {}",char::from(buf[0]));
        if found {
            if seek && char::from(buf[0]) != ':' {
                
            } else if char::from(buf[0]) != line_end{
                if seek && char::from(buf[0]) == ':' {
                    // println!("found :");
                    seek = false;
                    client.read(&mut buf)?;
                }else{
                    // println!("added: {}",char::from(buf[0]));
                    ret_str.push(buf[0]);
                    // ret_str += &char::from(buf[0]).to_string();
                }
            } else{
                break;
            }
        }else if (!look) && (char::from(buf[0]) == key_chars[0]) {
            look = true;
            ind = 1;
        } else if look && (char::from(buf[0]) == key_chars[ind]) {
            ind += 1;
            if ind == key.len() {
                found = true;
            }
        } else if look && (char::from(buf[0]) != key_chars[ind]) {
            ind = 0;
            look = false;
        }
    
        read_len = client.read(&mut buf)?;
        // println!("Read {} at {} bytes after,",char::from(buf[0]),read_len);
    }
    let mut string = String::from_utf8(ret_str).expect("String should be utf-8 encoded");
    if string.ends_with(',') {
        let mut chars = string.chars();
        chars.next_back();
        string = chars.collect();
    }
    Ok(string)
}

fn post_spotify(client: &mut EspHttpConnection,code: &str) -> Result<(String,String,u64),EspError> {
    let payload = format!("grant_type=authorization_code&code={}&redirect_uri=http://music-controller.local/callback",code);
    println!("Payload: {}",payload);
    let content_length_header = format!("{}", payload.len());

    let mut client_buf = [0u8;1000];
    let mut secret_buf = [0u8;1000];
    let client_id = read_data("client_id", &mut client_buf)?.expect("client_id is initialized");
    let client_secret = read_data("client_secret", &mut secret_buf)?.expect("client_secret is initialized");
    let auth = format!("Basic {}",BASE64_STANDARD.encode(format!("{}:{}",client_id,client_secret)));
    
    println!("Auth is {}",auth);
    let headers = [
        ("Authorization",auth.as_str()),
        ("content-type", "application/x-www-form-urlencoded"),
        ("content-length", &*content_length_header),
    ];
    let url = "https://accounts.spotify.com/api/token";
    client.initiate_request(Method::Post, &url, &headers)?;
    client.write_all(payload.as_bytes())?;
    println!("-> POST {}",url);
    client.initiate_response()?;

    let resp = client.status();
    let len:usize = client.header("Content-Length").unwrap().parse().unwrap();
    println!("Resp {}",resp);
    println!("Resp len: {:?}",len);

    let mut buf = [0u8; 1024];
    let read_len = client.read(&mut buf)?;
    println!("read {} bytes",read_len);
    let json_response = std::str::from_utf8(&buf[0..read_len]).unwrap();
    println!("Json Response: {}",json_response);
    let response:Value = serde_json::from_str(json_response).unwrap();
    println!("Got access_token {}, refresh token {}",response["access_token"],response["refresh_token"]);
    let refresh = response["refresh_token"].to_string();
    let mut refresh = refresh.chars();
    refresh.next();
    refresh.next_back();
    let token = response["access_token"].to_string();
    let mut token = token.chars();
    token.next();
    token.next_back();
    let token = token.collect::<String>();
    let refresh = refresh.collect::<String>();

    save_data("auth_token",&token)?;
    save_data("refresh_token",&refresh)?;

    Ok((token,refresh,response["expires_in"].to_string().parse().unwrap()))
}