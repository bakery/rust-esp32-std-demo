#![allow(unused_imports)]
#![allow(clippy::single_component_path_imports)]
//#![feature(backtrace)]

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Condvar, Mutex};
use std::{cell::RefCell, env, sync::atomic::*, sync::Arc, thread, time::*};

use anyhow::bail;

use log::*;

use url;

use embedded_hal::adc::OneShot;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::OutputPin;

use embedded_svc::eth;
use embedded_svc::eth::{Eth, TransitionalState};
use embedded_svc::httpd::registry::*;
use embedded_svc::httpd::*;
use embedded_svc::io;
use embedded_svc::ipv4;
use embedded_svc::mqtt::client::{Publish, QoS};
use embedded_svc::ping::Ping;
use embedded_svc::sys_time::SystemTime;
use embedded_svc::timer::TimerService;
use embedded_svc::timer::*;
use embedded_svc::wifi::*;

use esp_idf_svc::eth::*;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::httpd as idf;
use esp_idf_svc::httpd::ServerRegistry;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::netif::*;
use esp_idf_svc::nvs::*;
use esp_idf_svc::ping;
use esp_idf_svc::sntp;
use esp_idf_svc::sysloop::*;
use esp_idf_svc::systime::EspSystemTime;
use esp_idf_svc::timer::*;
use esp_idf_svc::wifi::*;

use esp_idf_hal::adc;
use esp_idf_hal::delay;
use esp_idf_hal::gpio;
use esp_idf_hal::i2c;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi;

use esp_idf_sys::{self, c_types};
use esp_idf_sys::{esp, EspError};

thread_local! {
    static TLS: RefCell<u32> = RefCell::new(13);
}

#[allow(dead_code)]
const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
#[allow(dead_code)]
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");
const version: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    #[allow(unused)]
    let peripherals = Peripherals::take().unwrap();
    #[allow(unused)]
    let pins = peripherals.pins;

    #[allow(unused)]
    let netif_stack = Arc::new(EspNetifStack::new()?);
    #[allow(unused)]
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    #[allow(unused)]
    let default_nvs = Arc::new(EspDefaultNvs::new()?);

    let request_restart = Arc::new(Mutex::new(false));

    info!("WIFI setup information: {:?} with {:?}", SSID, PASS);
    info!("Current version {:?}", version);

    #[allow(clippy::redundant_clone)]
    #[allow(unused_mut)]
    let mut wifi = wifi(
        netif_stack.clone(),
        sys_loop_stack.clone(),
        default_nvs.clone(),
    )?;


    let mutex = Arc::new((Mutex::new(None), Condvar::new()));

    let httpd = httpd(mutex.clone(), request_restart.clone())?;

    let mut wait = mutex.0.lock().unwrap();

    #[allow(unused)]
    let cycles = loop {
        if let Some(cycles) = *wait {
            break cycles;
        } else {
            wait = mutex
                .1
                .wait_timeout(wait, Duration::from_secs(1))
                .unwrap()
                .0;

            log::info!(
                "Request restart: {:?}", *request_restart.lock().unwrap()
            );

            if *request_restart.lock().unwrap() {
                log::info!("Restart requested");
                thread::sleep(Duration::from_secs(2));
                break 0;
            }
        }
    };

    for s in 0..3 {
        info!("Shutting down in {} secs", 3 - s);
        thread::sleep(Duration::from_secs(1));
    }

    drop(httpd);
    info!("Httpd stopped");

    {
        drop(wifi);
        info!("Wifi stopped");
    }

    if *request_restart.lock().unwrap() {
        unsafe {
            info!("Restarting...");
            esp_idf_sys::esp_restart();
        }
    }

    Ok(())
}

#[allow(unused_variables)]
fn httpd(mutex: Arc<(Mutex<Option<u32>>, Condvar)>, request_restart: Arc<Mutex<bool>>) -> Result<idf::Server> {
    let server = idf::ServerRegistry::new()
        .at("/")
        .get(|_| Ok(r#"
        <!DOCTYPE html>
        <html>
            <body>
                <h1>Firmware OTA updates</h1>
                <form method="post" action="/api/ota" enctype="application/x-www-form-urlencoded">
                    Firmware to use
                    <select name="firmware" disabled="">
                        <option>Loading...</option>
                    </select>
                    <input type="submit" value="Use this firmware">
                </form>
                <script>
                    const createOption = (text, value, disabled = false) => {
                        const option = document.createElement("option");
                        option.setAttribute("value", value);
                        if (disabled) {
                            option.setAttribute("disabled", "");
                        }
                        option.innerHTML = text;
                        return option;
                    };

                    const getCurrentVersion = async () => {
                        const d = await fetch("/api/version"); 
                        return d.text();
                    };

                    const getAvailableReleases = async () => {
                        const r = await fetch("https://raw.githubusercontent.com/bakery/rust-esp32-std-demo/feature/ota-updates/bin/releases.json");
                        return r.json();
                    };

                    const main = async () => {
                        const releases = await getAvailableReleases();
                        const version = await getCurrentVersion();
                        const $firmwareSelector = document.querySelector("select[name=firmware]");

                        // clean up
                        $firmwareSelector.children[0].remove();
                        $firmwareSelector.removeAttribute("disabled");

                        releases.forEach(({ tag_name: release, assets }) => {
                            const asset = assets.find(a => a.name.match(/bin$/ig));
                            const isCurrent = release === version;
                            $firmwareSelector.add(createOption(`${release}${isCurrent ? ' [CURRENT]' : ''}`, asset.browser_download_url, isCurrent));
                        });
                    };

                    main();
                </script>
            </body>
        </html>
        "#.into()))?
        .at("/foo")
        .get(|_| bail!("Boo, something happened!"))?
        .at("/bar")
        .get(|_| {
            Response::new(403)
                .status_message("No permissions")
                .body("You have no permissions to access this page".into())
                .into()
        })?
        .at("/api/version").get(move |_| {
            Ok(embedded_svc::httpd::Response::from(version))
        })?
        .at("/api/ota").post(move |mut request| {
            use embedded_svc::http::{self, client::*, status, Headers, Status};
            use embedded_svc::io::Bytes;
            use esp_idf_svc::http::client::*;
            use embedded_svc::ota::{Ota, OtaSlot, OtaUpdate};
            use esp_idf_svc::ota::{EspOta};
            use embedded_svc::io::{Write};
    
            let body = request.as_bytes()?;

            let firmware = url::form_urlencoded::parse(&body)
                .filter(|p| p.0 == "firmware")
                .map(|p| p.1)
                .next()
                .ok_or(anyhow::anyhow!("No parameter firmware"))?;
            
            info!("Gonna use firmware from: {:?}", firmware);

            let mut ota = EspOta::new().unwrap();
            let mut client = EspHttpClient::new(&EspHttpClientConfiguration {
                crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
                buffer_size_tx: Some(1024),
                ..Default::default()
            })?;
            let response = client.get(firmware)?.submit()?;
    
            info!(">>>>>>>>>>>>>>>> initiating OTA update");
    
            let mut ota_update = ota.initiate_update().unwrap();
            let mut firmware_update_ok = true;
    
            loop {
                let bytes_to_take = 10 * 1024;
                let body: Result<Vec<u8>, _> = Bytes::<_, 64>::new(response.reader()).take(bytes_to_take).collect();
                let body = body?;
                info!(">>>>>>>>>>>>>> got new firmware batch {:?}", body.len());
    
                match ota_update.do_write(&body) {
                    Ok(buff_len) => info!("wrote update: {:?}", buff_len),
                    Err(err) => {
                        info!("failed to write update with: {:?}", err);
                        firmware_update_ok = false;
                        break;
                    }
                }
    
                if body.len() < bytes_to_take {
                    break;
                }
            }
    
            info!(">>>>>>>>>>>>>>>> firmware update ok says: {:?}", firmware_update_ok);
    
            if firmware_update_ok {
                ota_update.complete().unwrap();
                *request_restart.lock().unwrap() = true;
                info!(">>>>>>>>>>>>>>>> completed firmware update");
            } else {
                ota_update.abort().unwrap();
            }

            Ok(embedded_svc::httpd::Response::from("Firmware updated. Restarting device..."))
        })?
        .at("/panic")
        .get(|_| panic!("User requested a panic!"))?;

    #[cfg(esp32s2)]
    let server = httpd_ulp_endpoints(server, mutex)?;

    server.start(&Default::default())
}

#[cfg(not(feature = "qemu"))]
#[allow(dead_code)]
fn wifi(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>> {
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

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

    // wifi.set_configuration(&Configuration::Mixed(
    //     ClientConfiguration {
    //         ssid: SSID.into(),
    //         password: PASS.into(),
    //         channel,
    //         ..Default::default()
    //     },
    //     AccessPointConfiguration {
    //         ssid: "aptest".into(),
    //         channel: channel.unwrap_or(1),
    //         ..Default::default()
    //     },
    // ))?;
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        password: PASS.into(),
        channel,
        ..Default::default()
    }))?;

    info!("Wifi configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
                .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        _
        // ApStatus::Started(ApIpStatus::Done),
    ) = status
    {
        info!("Wifi connected");

        ping(&ip_settings)?;
    } else {
        bail!("Unexpected Wifi status: {:?}", status);
    }

    Ok(wifi)
}

fn ping(ip_settings: &ipv4::ClientSettings) -> Result<()> {
    info!("About to do some pings for {:?}", ip_settings);

    let ping_summary =
        ping::EspPing::default().ping(ip_settings.subnet.gateway, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        bail!(
            "Pinging gateway {} resulted in timeouts",
            ip_settings.subnet.gateway
        );
    }

    info!("Pinging done");

    Ok(())
}
