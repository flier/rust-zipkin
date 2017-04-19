#![recursion_limit = "16384"]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate clap;
extern crate url;
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
use std::net::{TcpStream, Shutdown, ToSocketAddrs};
use std::sync::Arc;
use std::thread;
use std::marker::PhantomData;

use clap::{Arg, App};

use url::Url;

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
        UrlError(::url::ParseError);
        HyperError(::hyper::Error);
        TlsError(::native_tls::Error);
        TlsServerError(::hyper_native_tls::ServerError);
    }

    links {
        Zipkin(::zipkin::Error, ::zipkin::ErrorKind);
        Core(::zipkin::core::Error, ::zipkin::core::ErrorKind);
        Async(::zipkin::async::Error, ::zipkin::async::ErrorKind);
        Json(::zipkin::json::Error, ::zipkin::json::ErrorKind);
        Thrift(::zipkin::thrift::Error, ::zipkin::thrift::ErrorKind);
        Kafka(::zipkin::kafka::Error, ::zipkin::kafka::ErrorKind);
        Http(::zipkin::http::Error, ::zipkin::http::ErrorKind);
    }
}

struct SimpleProxy<'a, S, C>
    where S: 'static + zipkin::Sampler<'a>,
          C: 'static + zipkin::Collector<'a>
{
    addr: String,
    proto: String,
    tracer: Arc<zipkin::Tracer<S, C>>,
    phantom: PhantomData<&'a S>,
}

impl<'a, S, C> Handler for SimpleProxy<'a, S, C>
    where S: 'static + zipkin::Sampler<'a>,
          C: 'static + zipkin::Collector<'a>
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
    where S: 'static + zipkin::Sampler<'a>,
          C: 'static + zipkin::Collector<'a>
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
        where S: 'static + zipkin::Sampler<'a>,
              C: 'static + zipkin::Collector<'a>
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
        where S: 'static + zipkin::Sampler<'a>,
              C: 'static + zipkin::Collector<'a>
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

                    annotate!(span,
                              if to_upstream {
                                  zipkin::CLIENT_RECV
                              } else {
                                  zipkin::SERVER_RECV
                              });

                    break;
                }

                Ok(read) => {
                    debug!("received {} bytes from {}", read, from.peer_addr()?);

                    annotate!(span,
                              if to_upstream {
                                  zipkin::CLIENT_RECV_FRAGMENT
                              } else {
                                  zipkin::SERVER_RECV_FRAGMENT
                              });

                    match to.write(&buf[..read]) {
                        Ok(wrote) => {
                            debug!("sent {} bytes to {}", wrote, to.peer_addr()?);

                            annotate!(span,
                                      if to_upstream {
                                          zipkin::CLIENT_SEND_FRAGMENT
                                      } else {
                                          zipkin::SERVER_SEND_FRAGMENT
                                      });
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

impl<'a> Default for DummyCollector<'a, Vec<zipkin::Span<'a>>> {
    fn default() -> Self {
        DummyCollector(PhantomData)
    }
}

impl<'a> zipkin::core::Collector for DummyCollector<'a, Vec<zipkin::Span<'a>>> {
    type Item = Vec<zipkin::Span<'a>>;
    type Output = ();
    type Error = zipkin::Error;

    fn submit(&self, spans: Self::Item) -> zipkin::Result<()> {
        info!("{:?}", spans);

        Ok(())
    }
}

struct Config {
    addr: String,
    threads: Option<usize>,
    sample_rate: usize,
    format: String,
    collector_uri: Option<Url>,
}

fn parse_cmd_line() -> Result<Config> {
    let default_threads = num_cpus::get().to_string();
    let default_sample_rate = 1.to_string();
    let default_format = "pretty_json";

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
        .arg(Arg::with_name("format")
                 .short("f")
                 .long("format")
                 .value_name("FMT")
                 .takes_value(true)
                 .default_value(&default_format)
                 .help("encode span in format (json, pretty_json, thrift)"))
        .arg(Arg::with_name("collector-uri")
                 .short("u")
                 .long("collector-uri")
                 .value_name("URI")
                 .takes_value(true)
                 .help("Collector URI for tracing"))
        .get_matches();

    Ok(Config {
           addr: opts.value_of("listen").unwrap().to_owned(),
           threads: {
               let threads = opts.value_of("threads").unwrap().parse()?;

               if threads > 1 { Some(threads) } else { None }
           },
           sample_rate: opts.value_of("sample-rate").unwrap().parse()?,
           format: opts.value_of("format").unwrap().to_owned(),
           collector_uri: opts.value_of("collector-uri")
               .and_then(|uri| Url::parse(uri).ok()),
       })
}

macro_rules! trace {
    ($format:expr, $url:ident, $callback:expr) => {
        match $format {
            "json" => {
                info!("use JSON encoder");

                trace_with_encoder!(zipkin::codec::json(), $url, $callback)
            }
            "pretty" |"pretty_json" => {
                info!("use pretty JSON encoder");

                trace_with_encoder!(zipkin::codec::pretty_json(), $url, $callback)
            },
            "thrift" => {
                info!("use thrift encoder");

                trace_with_encoder!(zipkin::codec::thrift(), $url, $callback)
            },
            _ => panic!("unknown message format: {}", $format)
        }
    };
}

macro_rules! trace_with_encoder {
    ($encoder:expr, $url:ident, $callback:expr) => {
        if let Some(url) = $url {
            match url.scheme() {
                "kafka" => {
                    let addr = url.with_default_port(|_| Ok(9092))
                                  .unwrap()
                                  .to_socket_addrs()
                                  .unwrap()
                                  .next()
                                  .unwrap()
                                  .to_string();
                    let topic = url.fragment().unwrap_or("zipkin");
                    let config = zipkin::kafka::Config::new(&[addr], topic);
                    let transport = zipkin::kafka::Transport::new(config).unwrap();
                    let collector = zipkin::collector::new($encoder, transport);

                    $callback(collector);
                }
                "http" => {
                    let config = zipkin::http::Config::new($encoder.mime_type());
                    let transport = zipkin::http::Transport::new(url.as_str(), config).unwrap();
                    let collector = zipkin::collector::new($encoder, transport);

                    $callback(collector);
                }
                _ => panic!("unknown collector type: {}", url.scheme())
            }
        } else {
            let collector = DummyCollector::default();

            $callback(collector);
        }
    }
}

fn main() {
    pretty_env_logger::init().unwrap();

    let cfg = parse_cmd_line().unwrap();

    let format = cfg.format.as_str();
    let uri = cfg.collector_uri.as_ref();

    let sampler = zipkin::FixedRate::new(cfg.sample_rate);
    let addr = cfg.addr;
    let threads = cfg.threads;

    trace!(format, uri, |collector| {
        let tracer = Arc::new(zipkin::Tracer::with_sampler(sampler, collector));

        let server = Server::http(&addr).unwrap();

        info!("listening on {}", addr);

        let proxy = SimpleProxy {
            addr: addr,
            proto: "http".to_owned(),
            tracer: tracer,
            phantom: PhantomData,
        };

        if let Some(threads) = threads {
            info!("starting {} v{} with {} threads to handle requests",
                  APP_NAME,
                  APP_VERSION,
                  threads);

            server.handle_threads(proxy, threads).unwrap();
        } else {
            info!("starting {} v{} to handle requests", APP_NAME, APP_VERSION);

            server.handle(proxy).unwrap();
        }
    });
}