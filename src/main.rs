use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;

use crate::ec11::ec11_service;
use crate::wifi::tcp_service;

mod ec11;
mod wifi;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let Peripherals { pins, modem, .. } = Peripherals::take().unwrap();

    let Pins {
        gpio5,
        gpio6,
        gpio7,
        ..
    } = pins;

    let sysloop = EspSystemEventLoop::take().unwrap();

    // 连接 wifi, 并连接 tcp server
    tcp_service(sysloop.clone(), modem);

    // ec11 编码器: A B
    ec11_service(gpio5, gpio6, gpio7);

    println!("Hello, world!");
}
