use std::time::Duration;

use bytes::BytesMut;

use tokio_io::codec::Encoder;

use hyper::{self, Url};
use hyper::mime::Mime;
use hyper::client::{Client, RedirectPolicy};
use hyper::header::{Headers, ContentType};

use zipkin;

use errors::{Error, ErrorKind, Result};

pub struct HttpConfig {
    pub content_type: Mime,
    pub redirect_policy: RedirectPolicy,
    pub read_timeout: Option<Duration>,
    pub write_timeout: Option<Duration>,
    pub max_message_size: usize,
}

impl HttpConfig {
    pub fn new(content_type: Mime) -> Self {
        HttpConfig {
            content_type: content_type,
            redirect_policy: RedirectPolicy::FollowAll,
            read_timeout: Some(Duration::from_secs(15)),
            write_timeout: Some(Duration::from_secs(15)),
            max_message_size: 4096,
        }
    }

    pub fn headers(&self) -> Headers {
        let mut headers = Headers::new();

        headers.set(ContentType(self.content_type.clone()));

        headers
    }
}

pub struct HttpCollector<E> {
    encoder: E,
    base: Url,
    config: HttpConfig,
}

impl<'a, E> HttpCollector<E>
    where E: Encoder<Item = zipkin::Span<'a>, Error = Error>
{
    pub fn new(encoder: E, base: Url, config: HttpConfig) -> Self {
        HttpCollector {
            encoder: encoder,
            base: base,
            config: config,
        }
    }
}

impl<'a, E> zipkin::Collector<'a> for HttpCollector<E>
    where E: Encoder<Item = zipkin::Span<'a>, Error = Error>
{
    type Error = Error;

    fn submit(&mut self, span: zipkin::Span<'a>) -> Result<()> {
        let mut buf = BytesMut::with_capacity(self.config.max_message_size);

        self.encoder.encode(span, &mut buf)?;

        let mut client = Client::new();

        client.set_redirect_policy(self.config.redirect_policy);
        client.set_read_timeout(self.config.read_timeout);
        client.set_write_timeout(self.config.write_timeout);

        let res = client.post(self.base.clone())
            .body(&buf[..])
            .headers(self.config.headers())
            .send()?;

        if res.status != hyper::Ok {
            bail!(ErrorKind::ResponseError(res.status))
        } else {
            Ok(())
        }
    }
}