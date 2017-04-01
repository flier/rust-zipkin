#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate clap;
#[macro_use]
extern crate hyper;
extern crate hyper_native_tls;
#[macro_use]
extern crate mime;
#[macro_use]
extern crate serde_json;
extern crate num_cpus;

use std::io::prelude::*;

use clap::{Arg, App};

use hyper::method::Method;
use hyper::status::StatusCode;
use hyper::header::{Headers, ContentType, TransferEncoding, Encoding};
use hyper::server::{Handler, Server, Request, Response};
use hyper::client::{Body, Client, RedirectPolicy};
use hyper::uri::RequestUri;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;

const APP_NAME: &'static str = "simple_proxy";
const APP_VERSION: &'static str = "0.1.0";

header! { (ProxyConnection, "Proxy-Connection") => [String] }
header! { (ProxyAgent, "Proxy-Agent") => [String] }
header! { (Forwarded, "Forwarded") => [String] }
header! { (XForwardedFor, "X-Forwarded-For") => [String] }
header! { (XForwardedPort, "X-Forwarded-Port") => [String] }
header! { (XForwardedProto, "X-Forwarded-Proto") => [String] }

error_chain!{
    foreign_links {
        EnvVarError(::std::env::VarError);
        ParseIntError(::std::num::ParseIntError);
    }
}

struct Config {
    addr: String,
    threads: usize,
}

fn parse_cmd_line() -> Result<Config> {
    let default_threads = num_cpus::get().to_string();

    let opts = App::new(APP_NAME)
        .version(APP_VERSION)
        .author("Flier Lu <flier.lu@gmail.com>")
        .arg(Arg::with_name("listen")
            .short("l")
            .long("listen")
            .help("Start listening to an address over HTTP.")
            .value_name("HOST[:PORT]")
            .takes_value(true)
            .default_value("127.0.0.1:8080"))
        .arg(Arg::with_name("threads")
            .short("t")
            .long("threads")
            .help("Number of multiple threads to handle requests")
            .value_name("NUM")
            .takes_value(true)
            .default_value(&default_threads))
        .get_matches();

    Ok(Config {
        addr: opts.value_of("listen").unwrap().to_owned(),
        threads: opts.value_of("threads").unwrap().parse()?,
    })
}

struct SimpleProxy {
    addr: String,
    proto: String,
}

impl Handler for SimpleProxy {
    fn handle(&self, req: Request, mut res: Response) {
        info!("request from {}: {} {} {}",
              req.version,
              req.remote_addr,
              req.method,
              req.uri);
        debug!("received headers:\n{}", req.headers);

        if req.headers.has::<ProxyConnection>() {
            self.request_proxy(req, res);
        } else {
            let method = req.method.clone();

            match method {
                Method::Get | Method::Post => self.serve_http_request(req, res),
                Method::Extension(ref method) if method == "CONNECT" => {
                    self.connect_proxy(req, res)
                }
                _ => *res.status_mut() = StatusCode::MethodNotAllowed,
            }
        }
    }
}

impl SimpleProxy {
    fn serve_http_request(&self, req: Request, mut res: Response) {
        info!("serve HTTP request");

        let mut headers = serde_json::Map::new();

        for header in req.headers.iter() {
            headers.insert(header.name().to_owned(), header.value_string().into());
        }

        let out = json!({
            "remote_addr": req.remote_addr.to_string(),
            "version": req.version.to_string(),
            "method": req.method.to_string(),
            "uri": req.uri.to_string(),
            "headers": serde_json::Value::Object(headers),
        });

        res.headers_mut().set(ContentType(mime!(Application / Json)));

        let mut stream = res.start().unwrap();
        serde_json::to_writer_pretty(&mut stream, &out).unwrap();
        stream.end().unwrap();
    }

    fn request_proxy(&self, mut req: Request, mut res: Response) {
        info!("serve HTTP request proxy");

        let mut headers = Headers::new();

        for header in req.headers.iter() {
            if !header.name().starts_with("Proxy-") {
                headers.append_raw(header.name().to_owned(),
                                   header.value_string().as_bytes().to_owned());
            }
        }

        headers.set(ProxyAgent(format!("{}/{}", APP_NAME, APP_VERSION)));
        headers.set(Forwarded(format!("for={};proto={};by={}", req.remote_addr.to_string(), self.proto, self.addr)));
        headers.set(XForwardedFor(req.remote_addr.ip().to_string()));
        headers.set(XForwardedPort(req.remote_addr.port().to_string()));
        headers.set(XForwardedProto(self.proto.clone()));

        let mut buf = vec![];

        req.read_to_end(&mut buf).unwrap();

        info!("sending request with {} bytes body to upstream: {} {}", buf.len(), req.method, req.uri);
        debug!("sending headers:\n{}", headers);

        let mut client = match req.uri {
            RequestUri::AbsoluteUri(ref url) if url.scheme() == "https" => {
                let ssl = NativeTlsClient::new().unwrap();
                let connector = HttpsConnector::new(ssl);
                Client::with_connector(connector)
            }
            _ => Client::new(),
        };

        client.set_redirect_policy(RedirectPolicy::FollowNone);

        let creq = client.request(req.method.clone(), &req.uri.to_string())
            .headers(headers)
            .body(Body::BufBody(&buf, buf.len()));

        let mut cres = creq.send().unwrap();

        let mut buf = vec![];

        cres.read_to_end(&mut buf).unwrap();

        info!("received response with {} bytes body from upstream: {} {}", buf.len(), cres.version, cres.status);
        debug!("received headers:\n{}", cres.headers);

        *res.status_mut() = cres.status;

        for header in cres.headers.iter() {
            res.headers_mut().append_raw(header.name().to_owned(),
                                         header.value_string().as_bytes().to_owned());
        }

        match cres.headers.get::<TransferEncoding>() {
            Some(ref encodings) if encodings.contains(&Encoding::Chunked) => {
                let mut res = res.start().unwrap();
                res.write_all(&buf).unwrap();
                res.end().unwrap();
            }
            _ => res.send(&buf).unwrap(),
        }
    }

    fn connect_proxy(&self, mut req: Request, mut res: Response) {
        info!("serve HTTP connection proxy");
    }
}

fn main() {
    pretty_env_logger::init().unwrap();

    let cfg = parse_cmd_line().unwrap();

    info!("listening on {}", cfg.addr);

    let proxy = SimpleProxy {
        addr: cfg.addr.clone(),
        proto: "http".to_owned(),
    };

    let server = Server::http(cfg.addr.clone()).unwrap();

    if cfg.threads > 1 {
        info!("starting {} v{} with {} threads to handle requests", APP_NAME, APP_VERSION, cfg.threads);

        server.handle_threads(proxy, cfg.threads).unwrap();
    } else {
        info!("starting {} v{} to handle requests", APP_NAME, APP_VERSION);

        server.handle(proxy).unwrap();
    }
}