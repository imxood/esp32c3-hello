use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;

mod ec11;
mod oled;
mod wifi;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let Peripherals {
        pins, modem, i2c0, ..
    } = Peripherals::take().unwrap();

    let Pins {
        // ec11 key
        gpio2,
        // ec11 a
        gpio3,
        // ec11 b
        gpio4,
        // oled sda
        gpio5,
        // oled scl
        gpio6,
        ..
    } = pins;

    // 连接 wifi, 并连接 tcp server
    #[cfg(feature = "wifi")]
    {
        use esp_idf_svc::eventloop::EspSystemEventLoop;
        let sysloop = EspSystemEventLoop::take().unwrap();
        use crate::wifi::tcp_service;
        tcp_service(sysloop.clone(), modem);
    }

    // ec11 编码器: A B
    #[cfg(feature = "ec11")]
    {
        use crate::ec11::ec11_service;
        ec11_service(gpio2, gpio3, gpio4);
    }

    // oled 显示屏
    #[cfg(feature = "oled")]
    {
        use crate::oled::oled_service;
        oled_service(i2c0, gpio5, gpio6);
    }

    println!("Hello, world!");
}
