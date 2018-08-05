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
use reqwest;
use reqwest::Method::Extension;
use serde_json::{Value, from_str};
use std::io::Read;
use std::sync::Mutex;
use std::sync::Arc;
use std::collections::LinkedList;

pub struct StatsMonitor {
    pub listens_op_cnt: LinkedList<usize>,
    pub put_op_cnt: LinkedList<usize>,
    pub subscribe_op_cnt: LinkedList<usize>,
    pub request_rate_hist: LinkedList<f64>,
    pub ipv4_size_hist: LinkedList<usize>,
    pub ipv6_size_hist: LinkedList<usize>,
    pub address_proxy: String,

    hist_size: usize
}

impl StatsMonitor
{
    pub fn new(address: String) -> Self {
        StatsMonitor {
            listens_op_cnt: LinkedList::new(),
            put_op_cnt: LinkedList::new(),
            subscribe_op_cnt: LinkedList::new(),
            request_rate_hist: LinkedList::new(),
            ipv4_size_hist: LinkedList::new(),
            ipv6_size_hist: LinkedList::new(),
            address_proxy: address,

            hist_size: 1000
        }
    }

    pub fn monitor(m: &Arc<Mutex<StatsMonitor>>) -> Result<(), reqwest::Error> {
        let client = reqwest::ClientBuilder::new().build()?;
        let stats_method = Extension("STATS".to_string());
        let mut res = client.request(stats_method, &*m.lock().unwrap().address_proxy).send()?;
        let mut body: String = String::new();
        let _ = res.read_to_string(&mut body);
        let v: Value = from_str(&body).unwrap();
        let mut sm = m.lock().unwrap();
        if sm.listens_op_cnt.len() > sm.hist_size {
            sm.listens_op_cnt.pop_front();
            sm.put_op_cnt.pop_front();
            sm.subscribe_op_cnt.pop_front();
            sm.request_rate_hist.pop_front();
            sm.ipv4_size_hist.pop_front();
            sm.ipv6_size_hist.pop_front();
        }
        sm.listens_op_cnt.push_back(v["listenCount"].as_u64().unwrap_or(0) as usize);
        sm.put_op_cnt.push_back(v["putCount"].as_u64().unwrap_or(0) as usize);
        sm.subscribe_op_cnt.push_back(v["pushListenersCount"].as_u64().unwrap_or(0) as usize);
        sm.request_rate_hist.push_back(v["requestRate"].as_f64().unwrap_or(0.));
        sm.ipv4_size_hist.push_back(v["nodeInfo"]["ipv4"]["network_size_estimation"].as_u64().unwrap_or(0) as usize);
        sm.ipv6_size_hist.push_back(v["nodeInfo"]["ipv6"]["network_size_estimation"].as_u64().unwrap_or(0) as usize);
        Ok(())
    }
}
