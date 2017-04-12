#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate clap;
#[macro_use]
extern crate hyper;
extern crate native_tls;
extern crate hyper_native_tls;
#[macro_use]
extern crate mime;
#[macro_use]
extern crate serde_json;
extern crate num_cpus;
#[macro_use]
extern crate zipkin;

use std::io::prelude::*;
use std::net::{TcpStream, Shutdown};
use std::sync::Arc;
use std::thread;
use std::marker::PhantomData;

use clap::{Arg, App};

use hyper::method::Method;
use hyper::status::StatusCode;
use hyper::header::{Headers, ContentType, TransferEncoding, Encoding};
use hyper::server::{Handler, Server, Request, Response};
use hyper::client::{Body, Client, RedirectPolicy};
use hyper::uri::RequestUri;
use hyper::net::{HttpStream, HttpsConnector};
use hyper_native_tls::NativeTlsClient;

use zipkin::prelude::*;

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
        IoError(::std::io::Error);
        EnvVarError(::std::env::VarError);
        ParseIntError(::std::num::ParseIntError);
        JsonError(::serde_json::Error);
        HyperError(::hyper::Error);
        TlsError(::native_tls::Error);
        TlsServerError(::hyper_native_tls::ServerError);
    }
}

struct SimpleProxy<'a, S, C>
    where S: zipkin::Sampler<Item = zipkin::Span<'a>>
{
    addr: String,
    proto: String,
    tracer: Arc<zipkin::Tracer<S, C>>,
}

impl<'a, S, C> Handler for SimpleProxy<'a, S, C>
    where S: 'static + zipkin::Sampler<Item = zipkin::Span<'a>>,
          C: 'static + zipkin::Collector<Item = zipkin::Span<'a>, Output = (), Error = Error>
{
    fn handle(&self, req: Request, mut res: Response) {
        debug!("request from {}: {} {} {}",
               req.version,
               req.remote_addr,
               req.method,
               req.uri);
        debug!("received headers:\n{}", req.headers);

        let mut span = self.tracer.span("request");

        annotate!(span, zipkin::SERVER_RECV);
        annotate!(span, zipkin::CLIENT_ADDR, req.remote_addr.to_string());
        annotate!(span, zipkin::HTTP_METHOD, req.method.to_string());
        annotate!(span, zipkin::HTTP_URL, req.uri.to_string());

        match req.method {
            Method::Get | Method::Post => {
                if req.headers.has::<ProxyConnection>() {
                    self.request_proxy(req, res, span).unwrap();
                } else {
                    self.serve_http_request(req, res, span).unwrap()
                }
            }

            Method::Connect => self.connection_proxy(req, res, span).unwrap(),

            _ => *res.status_mut() = StatusCode::MethodNotAllowed,
        }
    }
}

impl<'a, S, C> SimpleProxy<'a, S, C>
    where S: 'static + zipkin::Sampler<Item = zipkin::Span<'a>>,
          C: 'static + zipkin::Collector<Item = zipkin::Span<'a>, Output = (), Error = Error>
{
    fn serve_http_request(&self,
                          req: Request,
                          mut res: Response,
                          mut span: zipkin::Span<'a>)
                          -> Result<()> {
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

        res.headers_mut()
            .set(ContentType(mime!(Application / Json)));

        let mut stream = res.start()?;
        serde_json::to_writer_pretty(&mut stream, &out)?;
        stream.end()?;

        annotate!(span, zipkin::SERVER_SEND);

        self.tracer.submit(span)?;

        Ok(())
    }

    fn request_proxy(&self,
                     mut req: Request,
                     mut res: Response,
                     mut span: zipkin::Span<'a>)
                     -> Result<()> {
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

        req.read_to_end(&mut buf)?;

        annotate!(span, zipkin::HTTP_REQUEST_SIZE, buf.len());

        info!("sending request with {} bytes body to upstream: {} {}", buf.len(), req.method, req.uri);
        debug!("sending headers:\n{}", headers);

        let mut client = match req.uri {
            RequestUri::AbsoluteUri(ref url) if url.scheme() == "https" => {
                let ssl = NativeTlsClient::new()?;
                let connector = HttpsConnector::new(ssl);
                Client::with_connector(connector)
            }
            _ => Client::new(),
        };

        client.set_redirect_policy(RedirectPolicy::FollowNone);

        let creq = client
            .request(req.method.clone(), &req.uri.to_string())
            .headers(headers)
            .body(Body::BufBody(&buf, buf.len()));

        let mut upstream_span = span.child("request-proxy");

        annotate!(upstream_span, zipkin::CLIENT_SEND);
        annotate!(upstream_span, zipkin::HTTP_METHOD, req.method.to_string());
        annotate!(upstream_span, zipkin::HTTP_URL, req.uri.to_string());
        annotate!(upstream_span, zipkin::HTTP_REQUEST_SIZE, buf.len());

        let mut cres = creq.send()?;

        let mut buf = vec![];

        cres.read_to_end(&mut buf)?;

        annotate!(upstream_span, zipkin::HTTP_STATUS_CODE, cres.status.to_u16());
        annotate!(upstream_span, zipkin::HTTP_RESPONSE_SIZE, buf.len());
        annotate!(upstream_span, zipkin::CLIENT_RECV);

        self.tracer.submit(upstream_span)?;

        info!("received response with {} bytes body from upstream: {} {}", buf.len(), cres.version, cres.status);
        debug!("received headers:\n{}", cres.headers);

        *res.status_mut() = cres.status;

        for header in cres.headers.iter() {
            res.headers_mut()
                .append_raw(header.name().to_owned(),
                            header.value_string().as_bytes().to_owned());
        }

        match cres.headers.get::<TransferEncoding>() {
            Some(ref encodings) if encodings.contains(&Encoding::Chunked) => {
                let mut res = res.start()?;
                res.write_all(&buf)?;
                res.end()?;
            }
            _ => res.send(&buf)?,
        }

        annotate!(span, zipkin::SERVER_SEND);

        self.tracer.submit(span)?;

        Ok(())
    }

    fn connection_proxy(&self,
                        req: Request,
                        mut res: Response,
                        mut span: zipkin::Span<'a>)
                        -> Result<()> {
        info!("serve HTTP connection proxy to {}", req.uri);

        let stream = if let RequestUri::Authority(ref addr) = req.uri {
            Some(TcpStream::connect(addr)?)
        } else {
            None
        };

        *res.status_mut() = stream
            .as_ref()
            .map_or(StatusCode::BadRequest, |_| StatusCode::Ok);

        annotate!(span, zipkin::HTTP_STATUS_CODE, res.status().to_u16());

        res.headers_mut()
            .set(ProxyAgent(format!("{}/{}", APP_NAME, APP_VERSION)));
        res.send(b"")?;

        annotate!(span, zipkin::SERVER_SEND);

        if let (Some(ref upstream), Some(&HttpStream(ref client))) =
            (stream, req.downcast_ref::<HttpStream>()) {
            Pipe::new(client.try_clone()?, upstream.try_clone()?)
                .run(self.tracer.clone(), span.id)?;
        }

        self.tracer.submit(span)?;

        Ok(())
    }
}

struct Pipe {
    client: TcpStream,
    upstream: TcpStream,
}

impl Pipe {
    fn new(client: TcpStream, upstream: TcpStream) -> Pipe {
        Pipe {
            client: client,
            upstream: upstream,
        }
    }

    fn run<'a, S, C>(&mut self,
                     tracer: Arc<zipkin::Tracer<S, C>>,
                     parent_id: zipkin::SpanId)
                     -> Result<()>
        where S: 'static + zipkin::Sampler<Item = zipkin::Span<'a>>,
              C: 'static + zipkin::Collector<Item = zipkin::Span<'a>, Output = (), Error = Error>
    {
        self.upstream.set_nodelay(true)?;
        self.client.set_nodelay(true)?;

        {
            let tracer = tracer.clone();
            let upstream = self.upstream.try_clone()?;
            let client = self.client.try_clone()?;

            thread::spawn(move || {
                              Self::copy(upstream, client, tracer, parent_id, false).unwrap();
                          });
        }

        {
            let tracer = tracer.clone();
            let upstream = self.upstream.try_clone()?;
            let client = self.client.try_clone()?;

            thread::spawn(move || {
                              Self::copy(client, upstream, tracer, parent_id, true).unwrap();
                          });
        }

        Ok(())
    }

    fn copy<'a, S, C>(mut from: TcpStream,
                      mut to: TcpStream,
                      tracer: Arc<zipkin::Tracer<S, C>>,
                      parent_id: zipkin::SpanId,
                      to_upstream: bool)
                      -> Result<()>
        where S: 'static + zipkin::Sampler<Item = zipkin::Span<'a>>,
              C: 'static + zipkin::Collector<Item = zipkin::Span<'a>, Output = (), Error = Error>
    {
        let mut buf = [0; 4096];

        let mut span = tracer
            .span(if to_upstream { "upstream" } else { "client" })
            .with_parent_id(parent_id);

        loop {
            match from.read(&mut buf) {
                Ok(0) => {
                    debug!("shutdow writing to {}", to.peer_addr()?);

                    to.shutdown(Shutdown::Write)?;

                    annotate!(span, if to_upstream { zipkin::CLIENT_RECV } else { zipkin::SERVER_RECV });

                    break;
                }

                Ok(read) => {
                    debug!("received {} bytes from {}", read, from.peer_addr()?);

                    annotate!(span, if to_upstream { zipkin::CLIENT_RECV_FRAGMENT } else { zipkin::SERVER_RECV_FRAGMENT });

                    match to.write(&buf[..read]) {
                        Ok(wrote) => {
                            debug!("sent {} bytes to {}", wrote, to.peer_addr()?);

                            annotate!(span, if to_upstream { zipkin::CLIENT_SEND_FRAGMENT } else { zipkin::SERVER_SEND_FRAGMENT });
                        }
                        Err(err) => {
                            warn!("fail to send to {}, {}", to.peer_addr()?, err);

                            bail!(err);
                        }
                    }
                }

                Err(err) => {
                    warn!("fail to receive from {}, {}", from.peer_addr()?, err);

                    bail!(err);
                }
            }
        }

        tracer.submit(span)?;

        Ok(())
    }
}

struct DummyCollector<'a, T: 'a>(PhantomData<&'a T>);

unsafe impl<'a, T> Sync for DummyCollector<'a, T> {}
unsafe impl<'a, T> Send for DummyCollector<'a, T> {}

impl<'a> Default for DummyCollector<'a, zipkin::Span<'a>> {
    fn default() -> Self {
        DummyCollector(PhantomData)
    }
}

impl<'a> zipkin::Collector for DummyCollector<'a, zipkin::Span<'a>> {
    type Item = zipkin::Span<'a>;
    type Output = ();
    type Error = Error;

    fn submit(&self, span: zipkin::Span<'a>) -> Result<()> {
        info!("{:?}", span);

        Ok(())
    }
}

struct Config {
    addr: String,
    threads: usize,
    sample_rate: usize,
}

fn parse_cmd_line() -> Result<Config> {
    let default_threads = num_cpus::get().to_string();
    let default_sample_rate = 1.to_string();

    let opts = App::new(APP_NAME)
        .version(APP_VERSION)
        .author("Flier Lu <flier.lu@gmail.com>")
        .arg(Arg::with_name("listen")
                 .short("l")
                 .long("listen")
                 .value_name("HOST[:PORT]")
                 .takes_value(true)
                 .default_value("127.0.0.1:8080")
                 .help("Start listening to an address over HTTP."))
        .arg(Arg::with_name("threads")
                 .short("t")
                 .long("threads")
                 .value_name("NUM")
                 .takes_value(true)
                 .default_value(&default_threads)
                 .help("Number of multiple threads to handle requests"))
        .arg(Arg::with_name("sample-rate")
                 .short("s")
                 .long("sample-rate")
                 .value_name("NUM")
                 .takes_value(true)
                 .default_value(&default_sample_rate)
                 .help("Sample rate for span tracing"))
        .get_matches();

    Ok(Config {
           addr: opts.value_of("listen").unwrap().to_owned(),
           threads: opts.value_of("threads").unwrap().parse()?,
           sample_rate: opts.value_of("sample-rate").unwrap().parse()?,
       })
}

fn main() {
    pretty_env_logger::init().unwrap();

    let cfg = parse_cmd_line().unwrap();

    info!("listening on {}", cfg.addr);

    let tracer = Arc::new(zipkin::Tracer::with_sampler(zipkin::FixedRate::new(cfg.sample_rate),
                                                       DummyCollector::default()));

    let proxy = SimpleProxy {
        addr: cfg.addr.clone(),
        proto: "http".to_owned(),
        tracer: tracer,
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