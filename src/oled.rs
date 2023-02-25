//! I2C test with SSD1306
//!
//! Folowing pins are used:
//! SDA     GPIO5
//! SCL     GPIO6
//!
//! Depending on your target and the board you are using you have to change the pins.
//!
//! For this example you need to hook up an SSD1306 I2C display.
//! The display will flash black and white.

use std::time::Duration;

use esp_idf_hal::gpio::IOPin;
use esp_idf_hal::i2c::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::prelude::*;

use embedded_graphics::{
    mono_font::{ascii::*, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

pub fn oled_service(
    i2c: impl Peripheral<P = impl I2c> + 'static,
    sda: impl Peripheral<P = impl IOPin> + 'static,
    scl: impl Peripheral<P = impl IOPin> + 'static,
) {
    log::info!("Starting I2C SSD1306 test");

    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config).unwrap();

    std::thread::spawn(move || {
        let interface = I2CDisplayInterface::new(i2c);

        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().unwrap();

        let line_style = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .build();

        Rectangle::new(Point::new(0, 0), Size::new(128, 64))
            .into_styled(line_style)
            .draw(&mut display)
            .unwrap();

        Line::new(Point::new(63, 0), Point::new(63, 63))
            .into_styled(line_style)
            .draw(&mut display)
            .unwrap();

        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_9X15)
            .text_color(BinaryColor::On)
            .build();

        Text::with_baseline("Hello", Point::new(3, 3), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        Text::with_baseline("Rust!", Point::new(66, 3), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        display.flush().unwrap();

        let dyn_text_style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let mut i = 0i16;
        loop {
            i += 1;

            if i > 998 || i < -999 {
                i = 0;
            }

            Text::with_baseline(
                &format!("{i:03?}"),
                Point::new(3, 25),
                dyn_text_style,
                Baseline::Top,
            )
            .draw(&mut display)
            .unwrap();

            Text::with_baseline(
                &format!("{:03}", i + 1),
                Point::new(66, 25),
                dyn_text_style,
                Baseline::Top,
            )
            .draw(&mut display)
            .unwrap();

            display.flush().unwrap();

            std::thread::sleep(Duration::from_millis(16));
        }
    });
}
