fn main() {
    let networks = wifiscanner::scan().expect("Cannot scan network");
    println!("== List of networks");
    for network in networks {
        println!(
            "{} {:20} {:10} {:4} {}",
            network.mac.unwrap_or("NA".to_string()), network.ssid, network.channel, network.signal_level, network.security
        );
    }
}
