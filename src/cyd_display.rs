use lazy_static::lazy_static;
use trezor_tjpgdec::{JpegInput, JpegOutput, JDEC};

lazy_static! {
    static ref HW_FW_KANA_MAP: HashMap<char, char> = {
        let hw_kana: Vec<char> = vec![
            ' ', '｡', '｢', '｣', '､', '･', 'ｰ',
            'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ',
            'ｶ', 'ｷ', 'ｸ', 'ｹ', 'ｺ',
            'ｻ', 'ｼ', 'ｽ', 'ｾ', 'ｿ',
            'ﾀ', 'ﾁ', 'ﾂ', 'ﾃ', 'ﾄ',
            'ﾅ', 'ﾆ', 'ﾇ', 'ﾈ', 'ﾉ',
            'ﾊ', 'ﾋ', 'ﾌ', 'ﾍ', 'ﾎ',
            'ﾏ', 'ﾐ', 'ﾑ', 'ﾒ', 'ﾓ',
            'ﾔ',      'ﾕ',      'ﾖ',
            'ﾗ', 'ﾘ', 'ﾙ', 'ﾚ', 'ﾛ',
            'ﾜ',                'ｦ',
            'ﾝ',
            'ｧ', 'ｨ', 'ｩ', 'ｪ', 'ｫ',
            'ｬ',      'ｭ',      'ｮ',
            'ｯ'];

        let fw_kana: Vec<char> = vec![
            '　', '。', '「', '」', '、', '・', 'ー',
            'ア', 'イ', 'ウ', 'エ', 'オ',
            'カ', 'キ', 'ク', 'ケ', 'コ',
            'サ', 'シ', 'ス', 'セ', 'ソ',
            'タ', 'チ', 'ツ', 'テ', 'ト',
            'ナ', 'ニ', 'ヌ', 'ネ', 'ノ',
            'ハ', 'ヒ', 'フ', 'ヘ', 'ホ',
            'マ', 'ミ', 'ム', 'メ', 'モ',
            'ヤ',       'ユ',       'ヨ',
            'ラ', 'リ', 'ル', 'レ', 'ロ',
            'ワ',                   'ヲ',
            'ン',
            'ァ', 'ィ', 'ゥ', 'ェ', 'ォ',
            'ャ',       'ュ',       'ョ',
            'ッ'];
            fw_kana.into_iter().zip(hw_kana.into_iter()).collect()
    };
}

use std::{collections::HashMap, sync::Mutex};

use display_interface_spi::SPIInterfaceNoCS;

use embedded_graphics::{
    draw_target::DrawTarget, image::{Image, ImageRawBE}, mono_font::{
        jis_x0201::{FONT_10X20, FONT_6X13, FONT_9X15}, MonoTextStyle
    }, pixelcolor::{raw::RawU16, Rgb565}, prelude::{
        Point, RgbColor, Size, WebColors
    }, primitives::{Circle, PrimitiveStyle, Rectangle, StyledDrawable}, text::{renderer::TextRenderer, Text}, Drawable
};

use esp_idf_svc::{
    hal::{
        delay::{Ets, FreeRtos}, gpio::{
            AnyOutputPin, InputPin, Output, OutputPin, PinDriver
        }, ledc::{LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver, LowSpeed}, peripheral::Peripheral, spi::{
            config::{
                self, 
                MODE_0
            }, 
            SpiDeviceDriver, 
            SpiDriver, 
            SpiDriverConfig, 
            SPI2
        }, units::FromValueType 
    }, http::client::EspHttpConnection, sys::EspError
};

use mipidsi::{
    Builder, 
    Display, 
    Orientation,
    ColorOrder::Rgb
};

use esp_idf_svc::hal::ledc::config::TimerConfig;

use crate::{wifi::send_get_request, DISPLAY};

pub type CydDisplay<'a> = Display<SPIInterfaceNoCS<SpiDeviceDriver<'a, SpiDriver<'a>>,PinDriver<'a, AnyOutputPin, Output>,>,mipidsi::models::ILI9341Rgb565,PinDriver<'a, AnyOutputPin, Output>,>;

static BACKLIGHT:Mutex<Option<LedcDriver>> = Mutex::new(None);

#[derive(Debug)]
pub struct PrintOut{red_acc:usize,green_acc:usize,blue_acc:usize,total_pix:usize}
  
  impl JpegOutput for PrintOut {
      fn write(
          &mut self,
          _jd: &JDEC,
          _rect_origin: (u32, u32),
          _rect_size: (u32, u32),
          _bitmap: &[u16],
      ) -> bool {
        // println!("origin:{_rect_origin:?},size:{_rect_size:?},map:{_bitmap:?}");
        let mut data = Vec::new();
        for i in _bitmap{
          let split = i.to_be_bytes();
          data.push(split[0]);
          data.push(split[1]);
          let new_col = Rgb565::from(RawU16::from(*i));
          self.red_acc += new_col.r() as usize;
          self.green_acc += new_col.g() as usize;
          self.blue_acc += new_col.b() as usize;
          self.total_pix += 1;
        }
        let image_offset = Point::new(10, 10);
        let image_data = ImageRawBE::<Rgb565>::new(&data, _rect_size.0);
        let image_to_draw = Image::new(&image_data, Point::new(_rect_origin.0 as i32, _rect_origin.1 as i32) + image_offset);
        let mut display_lock = DISPLAY
        .lock()
        .expect("DISPLAY should not be held elsewhere");

        let display = display_lock
        .as_mut()
        .expect("DISPLAY should be initialized");
        image_to_draw.draw(display).unwrap();
        true
      }
  }
struct WebInput{client:EspHttpConnection}

impl JpegInput for WebInput {
    fn read(&mut self, buf: Option<&mut [u8]>, nread: usize) -> usize {
        let ret_len;
        if let Some(reader) = buf{
            ret_len = self.client.read(reader).unwrap();
        }else{
            let mut trash_buf = vec![0;nread];
            ret_len = self.client.read(trash_buf.as_mut_slice()).unwrap();
        }
        ret_len
    }
}

pub fn init_display<C,T>(
    spi: SPI2,
    rst: AnyOutputPin,
    dc: AnyOutputPin,
    bl: AnyOutputPin,
    ledc:impl Peripheral<P = C> + 'static,
    timer: impl Peripheral<P = T> + 'static,
    sclk: impl Peripheral<P = impl OutputPin> + 'static,
    sda: impl Peripheral<P = impl OutputPin> + 'static,
    sdi: impl Peripheral<P = impl InputPin> + 'static,
    cs: impl Peripheral<P = impl OutputPin> + 'static
) -> Result<(),EspError> 
where
    C: LedcChannel<SpeedMode = LowSpeed>,
    T: LedcTimer<SpeedMode = LowSpeed> + 'static,
    {
    let rst = PinDriver::output(rst)?;
    let dc = PinDriver::output(dc)?;
  
    let mut delay = Ets;
  
    // configuring the spi interface, note that in order for the ST7789 to work, the data_mode needs to be set to MODE_3
    let config = config::Config::new()
        .baudrate(30.MHz().into())
        .data_mode(MODE_0);
  
    let device = SpiDeviceDriver::new_single(
        spi,
        sclk,
        sda,
        Some(sdi),
        Some(cs),
        &SpiDriverConfig::new(),
        &config,
    )?;
  
    let di = SPIInterfaceNoCS::new(device, dc);
  
    let display = Builder::ili9341_rgb565(di)
        .with_display_size(240, 320)
        .with_color_order(Rgb)
        // set default orientation
        .with_orientation(Orientation::Landscape(true))
        .init(&mut delay, Some(rst))
        .unwrap();
    
    let mut channel = LedcDriver::new(
        ledc,
        LedcTimerDriver::new(
            timer,
            &TimerConfig::new().frequency(25.kHz().into()),
        )?,
        bl,
    )?;

    channel.set_duty(channel.get_max_duty())?;
    
    BACKLIGHT.lock().expect("BACKLIGHT is not held elsewhere").replace(channel);

    DISPLAY.lock().expect("DISPLAY should not be held elsewhere").replace(display);
    Ok(())
}

pub fn clear_display(color:Rgb565) -> Result<(),EspError> {
    DISPLAY.lock()
    .expect("DISPLAY should not be held elsewhere")
    .as_mut()
    .expect("DISPLAY should be initialized")
    .clear(color)
    .expect("DISPLAY should clear");

    Ok(())
}

pub fn draw_centered_text(text:&str){
    let mut display_lock = DISPLAY
    .lock()
    .expect("DISPLAY should not be held elsewhere");

    let display = display_lock
    .as_mut()
    .expect("DISPLAY should be initialized");
    
    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);

    let mut new_text = String::new();
    for char in text.chars(){
        if let Some(new_char) = HW_FW_KANA_MAP.get(&char) {
            new_text.push(*new_char);
        }else {
            new_text.push(char);
        }
    }
    Text::with_alignment(&new_text, Point::new(160, 120), style,embedded_graphics::text::Alignment::Center).draw(display).unwrap();

}

pub fn draw_song_info(title:&str,album:&str,artist:&str) -> Result<(),EspError>{

    let mut display_lock = DISPLAY
    .lock()
    .expect("DISPLAY should not be held elsewhere");

    let display = display_lock
    .as_mut()
    .expect("DISPLAY should be initialized");

    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    let mut new_text = String::new();
    for char in title.chars(){
        if let Some(new_char) = HW_FW_KANA_MAP.get(&char) {
            new_text.push(*new_char);
        }else {
            new_text.push(char);
        }
    }
    let new_text_len = style.measure_string(&new_text, Point::zero(), embedded_graphics::text::Baseline::Bottom);
    println!("Text len is: {}",new_text_len.bounding_box.size.width);
    let mut num_lines = 0;
    if new_text_len.bounding_box.size.width > 140 {
        (new_text,num_lines) = split_on_whitespace(new_text,14,3);
    }
    Text::with_alignment(&new_text, Point::new(170, 50), style,embedded_graphics::text::Alignment::Left).draw(display).unwrap();

    let style = MonoTextStyle::new(&FONT_9X15, Rgb565::WHITE);
    let mut new_text = String::new();
    for char in album.chars(){
        if let Some(new_char) = HW_FW_KANA_MAP.get(&char) {
            new_text.push(*new_char);
        }else {
            new_text.push(char);
        }
    }
    let new_text_len = style.measure_string(&new_text, Point::zero(), embedded_graphics::text::Baseline::Bottom);
    println!("Text len is: {}",new_text_len.bounding_box.size.width);
    let mut num_album_lines = 0;
    if new_text_len.bounding_box.size.width > 140 {
        (new_text,num_album_lines) = split_on_whitespace(new_text,15,2);
    }
    Text::with_alignment(&new_text, Point::new(170, 80 + num_lines*20), style,embedded_graphics::text::Alignment::Left).draw(display).unwrap();
    
    let style = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);
    let mut new_text = String::new();
    for char in artist.chars(){
        if let Some(new_char) = HW_FW_KANA_MAP.get(&char) {
            new_text.push(*new_char);
        }else {
            new_text.push(char);
        }
    }
    Text::with_alignment(&new_text, Point::new(170, 110 + num_lines*20 + num_album_lines * 10), style,embedded_graphics::text::Alignment::Left).draw(display).unwrap();

    Ok(())
}

pub fn split_on_whitespace(mut in_text:String, max_len:usize, max_lines:i32) -> (String,i32) {
    let mut ret_str = String::new();
    let mut chars_left = max_len;
    let mut temp_str = String::new();
    let mut num_lines = 0;

    in_text.push(' ');
    for char in in_text.chars() {
        if !char.is_whitespace(){
            temp_str.push(char);
        }else{
            if temp_str.len() < chars_left{
                ret_str.push_str(&temp_str);
                ret_str.push(char);
                chars_left -= temp_str.len() + 1;
                temp_str = String::new();
            }else{
                if num_lines < max_lines {
                    ret_str.push('\n');
                    ret_str.push_str(&temp_str);
                    ret_str.push(char);
                    chars_left = max_len-temp_str.len();
                    temp_str = String::new();
                    num_lines += 1;
                }else{
                    ret_str.push_str("...");
                    return (ret_str,num_lines);
                }
            }
                
        }
    }
    
    println!("Ret str:{ret_str}");
    (ret_str,num_lines)
}

pub fn draw_progress_bar(elapsed:f32,clear:bool) -> Result<(),EspError> {
    let mut display_lock = DISPLAY
    .lock()
    .expect("DISPLAY should not be held elsewhere");

    let display = display_lock
    .as_mut()
    .expect("DISPLAY should be initialized");
    if clear{
        display.fill_solid(&Rectangle::new(Point::new(0, 200), Size::new(320, 12)), Rgb565::BLACK).unwrap();
    }
    display.fill_solid(&Rectangle::new(Point::new(0, 200), Size::new((320.0 * elapsed) as u32, 12)), Rgb565::WHITE).unwrap();
    let mut circle_style = PrimitiveStyle::with_fill(Rgb565::CSS_GRAY);
    circle_style.stroke_color = Some(Rgb565::GREEN);
    circle_style.stroke_width = 3;
    Circle::with_center(Point::new((320.0 * elapsed) as i32, 200+6), 12-3).draw_styled(&circle_style, display).unwrap();
    Ok(())
}

pub fn draw_image(image_url:&str) -> Result<(),EspError> {
    let headers = [("accept", "text/plain")];
    let client = send_get_request(&headers, image_url)?;
    let mut buf = WebInput{client};
    let mut p = [0u8;10000];
    let mut input = match JDEC::new(&mut buf, &mut p) {
        Ok(x) => x,
        Err(x) => match x {
            trezor_tjpgdec::Error::Interrupted => todo!(),
            trezor_tjpgdec::Error::Input => todo!(),
            trezor_tjpgdec::Error::MemoryPool => todo!(),
            trezor_tjpgdec::Error::MemoryInput => todo!(),
            trezor_tjpgdec::Error::Parameter => todo!(),
            trezor_tjpgdec::Error::InvalidData => todo!(),
            trezor_tjpgdec::Error::UnsupportedJpeg => todo!(),
        },
    };
    let _ = input.set_scale(1);

    let mut decom = PrintOut{
        red_acc:0,
        green_acc:0,
        blue_acc:0,
        total_pix:0
    };
    let _ = input.decomp(&mut decom);
    println!("Decom: {:?}",decom);
    let new_display_color = Rgb565::new(
        (decom.red_acc/decom.total_pix) as u8,
        (decom.green_acc/decom.total_pix) as u8, 
        (decom.blue_acc/decom.total_pix) as u8,
    );
    clear_display_around_image(new_display_color)?;
    Ok(())
}

pub fn _clear_text_from_display(color:Rgb565) -> Result<(),EspError> {
    let mut display_lock = DISPLAY
    .lock()
    .expect("DISPLAY should not be held elsewhere");

    let display = display_lock
    .as_mut()
    .expect("DISPLAY should be initialized");

    display.fill_solid(&Rectangle::new(Point::new(160, 0), Size::new(160, 160)), color).unwrap();
    Ok(())
}

pub fn clear_display_around_image(color:Rgb565) -> Result<(),EspError> {
    let mut display_lock = DISPLAY
    .lock()
    .expect("DISPLAY should not be held elsewhere");

    let display = display_lock
    .as_mut()
    .expect("DISPLAY should be initialized");

    display.fill_solid(&Rectangle::new(Point::new(0, 0), Size::new(160, 10)), color).unwrap();
    display.fill_solid(&Rectangle::new(Point::new(0, 0), Size::new(10, 160)), color).unwrap();
    display.fill_solid(&Rectangle::new(Point::new(160, 0), Size::new(160, 160)), color).unwrap();
    display.fill_solid(&Rectangle::new(Point::new(0, 160), Size::new(320, 80)), color).unwrap();
    Ok(())
}

pub fn fade_out_backlight() -> Result<(),EspError>{
    let mut backlight_lock = BACKLIGHT
    .lock()
    .expect("BACKLIGHT should not be held elsewhere");

    let backlight = backlight_lock
    .as_mut()
    .expect("BACKLIGHT should be initialized");

    let max_duty = backlight.get_max_duty();
    for numerator in (0..100).rev() {
        backlight.set_duty(max_duty * numerator / 100)?;
        FreeRtos::delay_ms(5);
    }

    Ok(())
}

pub fn set_brightness(mut percent:u8) -> Result<(),EspError> {
    let mut backlight_lock = BACKLIGHT
    .lock()
    .expect("BACKLIGHT should not be held elsewhere");

    let backlight = backlight_lock
    .as_mut()
    .expect("BACKLIGHT should be initialized");

    let max_duty = backlight.get_max_duty();
    if percent > 100 {
        percent = 100;
    }
    backlight.set_duty(max_duty * percent as u32 / 100)?;
 

    Ok(())
}

pub fn fade_in_backlight() -> Result<(),EspError>{
    let mut backlight_lock = BACKLIGHT
    .lock()
    .expect("BACKLIGHT should not be held elsewhere");

    let backlight = backlight_lock
    .as_mut()
    .expect("BACKLIGHT should be initialized");

    let max_duty = backlight.get_max_duty();
    for numerator in 0..100 {
        backlight.set_duty(max_duty * numerator / 100)?;
        FreeRtos::delay_ms(5);
    }

    Ok(())
}