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
use hyper_native_tls::NativeTlsServer;
use iron::Handler;
use iron::mime::Mime;
use iron::prelude::*;
use iron::status;
use router::Router;
use statsmonitor::StatsMonitor;
use std::io::Read;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct API {
    address: String,
    cert_path: String,
    cert_pass: String,
    monitor: Arc<Mutex<StatsMonitor>>
}

impl API {
    pub fn new(monitor: Arc<Mutex<StatsMonitor>>, address: String, cert_path: String, cert_pass: String) -> API {
        API {
            address,
            cert_path,
            cert_pass,
            monitor
        }
    }

    /**
     * Launch an API instance
     * @param self
     */
    pub fn start(&mut self) {
        let mut router = Router::new();
        let monitor_handler = MonitorHandler {
            monitor: self.monitor.clone(),
            last_size: Arc::new(Mutex::new(0)),
            cache: Arc::new(Mutex::new(None))
        };
        router.get("/", monitor_handler, "monitor");
        router.get("/*", API::load_file, "path");
        let ssl = NativeTlsServer::new(&*self.cert_path, &*self.cert_pass).unwrap();
        info!("start API endpoint at {}", self.address);
        // Start router
        Iron::new(router).https(&*self.address, ssl).unwrap();
    }

    fn load_file(req: &mut Request) -> IronResult<Response> {
        let mut fullpath = String::from("web");
        for path in req.url.path() {
            fullpath += &format!("/{}", path);
        }
        let file = File::open(&*fullpath);
        if !file.is_ok() {
            warn!("Can't access to {}", fullpath);
            return Ok(Response::with((status::NotFound, "")))
        }

        let mut body = vec![];
        let _ = file.map(|mut f| f.read_to_end(&mut body));
        let extension = match Path::new(&*fullpath).extension() {
            Some(e) => e.to_str().unwrap_or(""),
            None => ""
        };
        let content_type_str = match extension {
            "html" => "text/html",
            "css" => "text/css",
            "js" => "text/javascript",
            "jpg" => "image/jpeg",
            "png" => "image/png",
            "woff" => "application/font-woff",
            "woff2" => "application/font-woff",
            "ttf" => "application/font-ttf",
            "otf" => "application/font-otf",
            _ => "application/octet-stream"
        };
        let content_type = content_type_str.parse::<Mime>().unwrap();
        Ok(Response::with((content_type, status::Ok, &*body)))
    }
}

struct MonitorHandler {
    monitor: Arc<Mutex<StatsMonitor>>,
    last_size: Arc<Mutex<usize>>,
    cache: Arc<Mutex<Option<String>>>
}

impl Handler for MonitorHandler {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        let m = self.monitor.lock().unwrap();
        let cache = self.cache.lock().unwrap().clone();
        if m.listens_op_cnt.len() == *self.last_size.lock().unwrap() && *self.cache.lock().unwrap() != None {
            let content_type = "text/html".parse::<Mime>().unwrap();
            return Ok(Response::with((content_type, status::Ok, &*cache.unwrap())));
        }
        *self.last_size.lock().unwrap() = m.listens_op_cnt.len();

        let file = File::open("web/index.html");
        if !file.is_ok() {
            warn!("Can't access to index.html");
            return Ok(Response::with((status::NotFound, "")))
        }

        let mut body = String::new();
        let _ = file.map(|mut f| f.read_to_string(&mut body));

        // General datas
        body = body.replace("%{address_proxy}", &*m.address_proxy);
        // Current datas
        body = body.replace("%{value_listen}", &*m.listens_op_cnt.back().unwrap_or(&0).to_string());
        body = body.replace("%{value_put}", &*m.put_op_cnt.back().unwrap_or(&0).to_string());
        body = body.replace("%{value_push}", &*m.subscribe_op_cnt.back().unwrap_or(&0).to_string());
        body = body.replace("%{value_ipv4}", &*m.ipv4_size_hist.back().unwrap_or(&0).to_string());
        body = body.replace("%{value_ipv6}", &*m.ipv6_size_hist.back().unwrap_or(&0).to_string());
        // Labels
        let mut labels = String::from("[");
        for i in 0..m.listens_op_cnt.len() {
            labels += &*i.to_string();
            if i != m.listens_op_cnt.len() {
                labels += ", ";
            }
        }
        labels += "]";
        body = body.replace("%{labels_listen}", &*labels);
        body = body.replace("%{labels_put}", &*labels);
        body = body.replace("%{labels_push}", &*labels);
        body = body.replace("%{labels_ipv6_size}", &*labels);
        body = body.replace("%{labels_ipv4_size}", &*labels);

        let mut labels = String::from("[");
        for i in 0..m.listens_op_cnt.len() {
            labels += &*i.to_string();
            if i != m.listens_op_cnt.len() {
                labels += ", ";
            }
        }
        labels += "]";

        // Data listen
        let mut datas = String::from("[");
        let mut idx = 0;
        for el in &m.listens_op_cnt {
            datas += &*el.to_string();
            idx += 1;
            if idx != m.listens_op_cnt.len() {
                datas += ", ";
            }
        }
        datas += "]";
        body = body.replace("%{data_listen}", &*datas);

        // Data Put
        let mut datas = String::from("[");
        let mut idx = 0;
        for el in &m.put_op_cnt {
            datas += &*el.to_string();
            idx += 1;
            if idx != m.put_op_cnt.len() {
                datas += ", ";
            }
        }
        datas += "]";
        body = body.replace("%{data_put}", &*datas);

        // Data Push
        let mut datas = String::from("[");
        let mut idx = 0;
        for el in &m.subscribe_op_cnt {
            datas += &*el.to_string();
            idx += 1;
            if idx != m.subscribe_op_cnt.len() {
                datas += ", ";
            }
        }
        datas += "]";
        body = body.replace("%{data_push}", &*datas);

        // Data IPv4
        let mut datas = String::from("[");
        let mut idx = 0;
        for el in &m.ipv4_size_hist {
            datas += &*el.to_string();
            idx += 1;
            if idx != m.ipv4_size_hist.len() {
                datas += ", ";
            }
        }
        datas += "]";
        body = body.replace("%{data_ipv4_size}", &*datas);

        // Data IPv6
        let mut datas = String::from("[");
        let mut idx = 0;
        for el in &m.ipv6_size_hist {
            datas += &*el.to_string();
            idx += 1;
            if idx != m.ipv6_size_hist.len() {
                datas += ", ";
            }
        }
        datas += "]";
        body = body.replace("%{data_ipv6_size}", &*datas);
        *self.cache.lock().unwrap() = Some(body.clone());

        let content_type = "text/html".parse::<Mime>().unwrap();
        Ok(Response::with((content_type, status::Ok, &*body)))
    }
}
