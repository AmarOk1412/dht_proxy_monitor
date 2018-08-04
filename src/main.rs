/*
 *  Copyright (C) 2018 Savoir-faire Linux Inc.
 *  Author: Sébastien Blin <sebastien.blin@savoirfairelinux.com>
 *
 *  This program is free software; you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation; either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */
extern crate env_logger;
extern crate hyper_native_tls;
extern crate iron;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate router;
extern crate serde;
extern crate serde_json;

mod api;
mod statsmonitor;

use api::API;
use serde_json::{Value, from_str};
use statsmonitor::StatsMonitor;
use std::io::Read;
use std::fs::File;
use std::sync::Mutex;
use std::sync::Arc;
use std::time::Duration;
use std::thread;

fn main() {
    // This script load config from config.json
    let mut file = File::open("config.json").ok()
        .expect("Config file not found");
    let mut config = String::new();
    file.read_to_string(&mut config).ok().expect("failed to read!");
    let config: Value = from_str(&*config).ok()
    .expect("Incorrect config file. Please check config.json");

    // Init monitor
    let shared_monitor : Arc<Mutex<StatsMonitor>> = Arc::new(
        Mutex::new(
            StatsMonitor::new(
                String::from(config["proxy"].as_str().unwrap_or(""))
            )
        )
    );
    let shared_monitor_cloned = shared_monitor.clone();
    let update_stats_thread = thread::spawn(move || {
        loop {
            StatsMonitor::monitor(&shared_monitor_cloned).ok()
            .expect("An error occured when initializing the monitor, abort.");
            let two_min = Duration::from_millis(2 * 60 * 1000);
            thread::sleep(two_min);
        }
    });

    // Launch API
    let mut api = API::new(shared_monitor,
                           String::from(config["api_listener"].as_str().unwrap_or("")),
                           String::from(config["cert_path"].as_str().unwrap_or("")),
                           String::from(config["cert_pass"].as_str().unwrap_or(""))
                       );
    api.start();
    let _ = update_stats_thread.join();
}
