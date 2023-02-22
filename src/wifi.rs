use std::io::Read;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::time::Duration;
use std::time::Instant;

use anyhow::bail;
use anyhow::Result;
use embedded_svc::ipv4;
use embedded_svc::wifi::ClientConfiguration;
use embedded_svc::wifi::Configuration;
use embedded_svc::wifi::Wifi;
use esp_idf_hal::modem::Modem;
use esp_idf_hal::peripheral;
use esp_idf_svc::eventloop::EspEventLoop;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::eventloop::System;
use esp_idf_svc::netif::*;
use esp_idf_svc::ping;
use esp_idf_svc::wifi::EspWifi;
use esp_idf_svc::wifi::WifiWait;
use log::*;

const SSID: &str = "maxu";
const PASS: &str = "mx123456";
const SERVER: &str = "192.168.3.118:1090";

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

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        password: PASS.into(),
        channel,
        ..Default::default()
    }))?;

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

    ping(ip_info.subnet.gateway).unwrap();

    info!("Wifi DHCP info: {:?}", ip_info);

    Ok(wifi)
}

fn ping(ip: ipv4::Ipv4Addr) -> Result<()> {
    info!("About to do some pings for {:?}", ip);

    let ping_summary = ping::EspPing::default().ping(
        ip,
        &embedded_svc::ping::Configuration {
            count: 1,
            ..Default::default()
        },
    )?;
    if ping_summary.transmitted != ping_summary.received {
        bail!("Pinging IP {} resulted in timeouts", ip);
    }

    info!("Pinging done");

    Ok(())
}

pub fn tcp_service(sysloop: EspEventLoop<System>, modem: Modem) {
    std::thread::Builder::new()
        .name("tcp_server".into())
        .stack_size(4096)
        .spawn(move || {
            let _wifi = wifi(modem, sysloop).unwrap();
            let mut buf = Vec::new();
            let server_addr = match SERVER.parse() {
                Ok(addr) => SocketAddr::V4(addr),
                Err(e) => {
                    error!("server addr format wrong, e: {:?}", e);
                    return;
                }
            };

            let start = Instant::now();

            loop {
                info!("be ready to open a tcp connection to {}", SERVER);

                let previous = start.elapsed();
                match TcpStream::connect_timeout(&server_addr, Duration::from_secs(3)) {
                    Ok(mut stream) => {
                        info!("connected to {}", SERVER);
                        loop {
                            match stream.read(&mut buf) {
                                Ok(size) => {
                                    if size == 0 {
                                        break;
                                    }
                                    info!("recv: {:?}", &buf[..size]);
                                }
                                Err(e) => {
                                    error!("read failed, e: {:?}", e);
                                    break;
                                }
                            }
                        }
                        info!("disconnected from {}", SERVER);
                    }
                    Err(e) => {
                        error!("connect failed, e: {}", &e);
                    }
                }

                std::thread::sleep(Duration::from_secs(5));
                info!("elapsed: {:?}", start.elapsed() - previous);
            }
        })
        .unwrap();
}
