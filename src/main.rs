use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use embedded_svc::ipv4;
use embedded_svc::wifi::AccessPointConfiguration;
use embedded_svc::wifi::ClientConfiguration;
use embedded_svc::wifi::Configuration;
use embedded_svc::wifi::Wifi;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::netif::*;
use esp_idf_svc::ping;
use esp_idf_svc::wifi::EspWifi;
use esp_idf_svc::wifi::WifiWait;
use log::*;

const SSID: &str = "maxu";
const PASS: &str = "mx123456";

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let sysloop = EspSystemEventLoop::take().unwrap();

    let _ = wifi(peripherals.modem, sysloop.clone()).unwrap();

    test_tcp().unwrap();

    let mut button = PinDriver::input(peripherals.pins.gpio9).unwrap();

    button.set_pull(Pull::Down).unwrap();

    std::thread::spawn(|| loop {
        println!("Hello, world! {:?}", std::thread::current());
        std::thread::sleep(Duration::from_millis(1000));
    });

    println!("Hello, world!");
}

fn wifi(
    modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
) -> Result<Box<EspWifi<'static>>> {
    use std::net::Ipv4Addr;

    let mut wifi = Box::new(EspWifi::new(modem, sysloop.clone(), None)?);

    info!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == SSID);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    wifi.start()?;

    info!("Starting wifi...");

    if !WifiWait::new(&sysloop)?
        .wait_with_timeout(Duration::from_secs(20), || wifi.is_started().unwrap())
    {
        bail!("Wifi did not start");
    }

    info!("Connecting wifi...");

    wifi.connect()?;

    if !EspNetifWait::new::<EspNetif>(wifi.sta_netif(), &sysloop)?.wait_with_timeout(
        Duration::from_secs(20),
        || {
            wifi.is_connected().unwrap()
                && wifi.sta_netif().get_ip_info().unwrap().ip != Ipv4Addr::new(0, 0, 0, 0)
        },
    ) {
        bail!("Wifi did not connect or did not receive a DHCP lease");
    }

    let ip_info = wifi.sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);

    ping(ip_info.subnet.gateway)?;

    ping("192.168.3.118".parse().unwrap())?;

    Ok(wifi)
}

fn ping(ip: ipv4::Ipv4Addr) -> Result<()> {
    info!("About to do some pings for {:?}", ip);

    let ping_summary = ping::EspPing::default().ping(ip, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        bail!("Pinging IP {} resulted in timeouts", ip);
    }

    info!("Pinging done");

    Ok(())
}

fn test_tcp() -> Result<()> {
    let addr = "192.168.3.118:1090";
    info!("About to open a TCP connection to {}", addr);

    match TcpStream::connect(addr) {
        Ok(mut stream) => {
            info!("1");
            stream.write_all("hello world\n\n".as_bytes())?;

            info!("2");

            let mut result = Vec::new();

            let size = stream.read_to_end(&mut result)?;
            info!("recv: {:?}", &result[..size]);

            Ok(())
        }
        Err(e) => {
            error!("connect failed, e: {:?}", &e);
            Err(anyhow!(e))
        }
    }
}
