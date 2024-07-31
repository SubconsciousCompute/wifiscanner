#![allow(non_snake_case)]

use anyhow::Context;
use std::process::Command;

use crate::Wifi;

/// Returns a list of WiFi hotspots in your area.
pub(crate) fn scan() -> anyhow::Result<Vec<Wifi>> {
    let output = Command::new("system_profiler")
        .arg("SPAirPortDataType")
        .arg("-json")
        .output()?;
    parse_systemprofiler(String::from_utf8_lossy(&output.stdout).into())
}

fn parse_systemprofiler(text: String) -> anyhow::Result<Vec<Wifi>> {
    #[derive(serde::Deserialize, Debug)]
    struct SystemProfilerData {
        SPAirPortDataType: Vec<Interfaces>,
    }

    #[derive(serde::Deserialize, Debug)]
    struct Interfaces {
        spairport_airport_interfaces: Vec<Interface>,
    }

    #[derive(serde::Deserialize, Debug)]
    struct Interface {
        spairport_airport_other_local_wireless_networks: Option<Vec<WifiPoint>>,
        // spairport_wireless_mac_address: String,
    }

    #[derive(serde::Deserialize, Debug)]
    struct WifiPoint {
        _name: String,
        spairport_network_channel: String,
        // spairport_network_phymode: String,
        spairport_security_mode: String,
        spairport_signal_noise: String,
    }

    let data: SystemProfilerData = serde_json::from_str(&text)?;

    let mut wifis = vec![];
    for interface in data.SPAirPortDataType.into_iter().map(|x| x.spairport_airport_interfaces).flatten() {
        for wifi in interface.spairport_airport_other_local_wireless_networks.unwrap_or(vec![]) {
            let ssid = wifi._name;
            let channel = wifi.spairport_network_channel;
            let security = wifi.spairport_security_mode;
            let security = security.strip_prefix("spairport_security_mode_").unwrap_or(&security).to_string();
            let signal_level = wifi.spairport_signal_noise.split('/').nth(0).unwrap_or("").trim().to_string();

            wifis.push( crate::Wifi {
                mac: None,
                ssid,
                channel,
                security,
                signal_level,
            })
        }
    }

    Ok(wifis)
}

/// Returns a list of WiFi hotspots in your area - (OSX/MacOS) uses `airport`
#[allow(dead_code)]
pub(crate) fn scan_using_airport() -> anyhow::Result<Vec<Wifi>> {
    let output = Command::new(
        "/System/Library/PrivateFrameworks/Apple80211.\
         framework/Versions/Current/Resources/airport",
    )
    .arg("-s")
    .output()?;

    let data = String::from_utf8_lossy(&output.stdout);

    parse_airport(&data)
}

fn parse_airport(network_list: &str) -> anyhow::Result<Vec<Wifi>> {
    let mut wifis: Vec<Wifi> = Vec::new();
    let mut lines = network_list.lines();
    let headers = match lines.next() {
        Some(v) => v,
        // return an empty list of WiFi if the network_list is empty
        None => return Ok(vec![]),
    };

    let headers_string = String::from(headers);
    let col_headers = ["BSSID", "RSSI", "CHANNEL", "HT", "SECURITY"]
        .iter()
        .map(|header| {
            headers_string
                .find(header)
                .context("HeaderNotFound in {header:?}")
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let col_mac = col_headers[0];
    let col_rrsi = col_headers[1];
    let col_channel = col_headers[2];
    let col_ht = col_headers[3];
    let col_security = col_headers[4];

    for line in lines {
        let ssid = &line[..col_mac].trim();
        let mac = &line[col_mac..col_rrsi].trim();
        let signal_level = &line[col_rrsi..col_channel].trim();
        let channel = &line[col_channel..col_ht].trim();
        let security = &line[col_security..].trim();

        wifis.push(Wifi {
            mac: Some(mac.to_string()),
            ssid: ssid.to_string(),
            channel: channel.to_string(),
            signal_level: signal_level.to_string(),
            security: security.to_string(),
            ..Default::default()
        });
    }

    Ok(wifis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;

    #[test]
    fn should_parse_system_profiler() {
        let txt = include_str!("../../tests/fixtures/systemprofiler/output.txt");
        let _wifis = parse_systemprofiler(txt.to_string()).unwrap();
    }

    #[test]
    fn should_parse_airport() {
        let mut expected: Vec<Wifi> = Vec::new();
        expected.push(Wifi {
            mac: Some("00:35:1a:90:56:03".to_string()),
            ssid: "OurTest".to_string(),
            channel: "112".to_string(),
            signal_level: "-70".to_string(),
            security: "WPA2(PSK/AES/AES)".to_string(),
        });

        expected.push(Wifi {
            mac: Some("00:35:1a:90:56:00".to_string()),
            ssid: "TEST-Wifi".to_string(),
            channel: "1".to_string(),
            signal_level: "-67".to_string(),
            security: "WPA2(PSK/AES/AES)".to_string(),
        });

        let path = PathBuf::from("tests/fixtures/airport/airport01.txt");

        let file_path = path.as_os_str();

        let mut file = File::open(&file_path).unwrap();

        let mut filestr = String::new();
        let _ = file.read_to_string(&mut filestr).unwrap();

        let result = parse_airport(&filestr).unwrap();
        let last = result.len() - 1;
        assert_eq!(expected[0], result[0]);
        assert_eq!(expected[1], result[last]);
    }

    #[test]
    #[should_panic]
    fn should_not_parse_other() {
        let path = PathBuf::from("tests/fixtures/iw/iw_dev_01.txt");
        let file_path = path.as_os_str();
        let mut file = File::open(&file_path).unwrap();
        let mut filestr = String::new();
        file.read_to_string(&mut filestr).unwrap();
        parse_airport(&filestr).unwrap(); // must panic
    }
}
