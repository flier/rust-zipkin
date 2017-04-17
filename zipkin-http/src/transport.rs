use std::time::Duration;
use std::marker::PhantomData;

use hyper;
use hyper::mime::Mime;
use hyper::client::{pool, Client, RedirectPolicy};
use hyper::header::{Headers, ContentType};

use zipkin_core::Transport;

use errors::{Error, ErrorKind, Result};

pub struct HttpConfig {
    pub content_type: Mime,
    pub redirect_policy: RedirectPolicy,
    pub read_timeout: Option<Duration>,
    pub write_timeout: Option<Duration>,
    pub max_message_size: usize,
    pub max_idle_connections: Option<usize>,
}

impl HttpConfig {
    pub fn new(content_type: Mime) -> Self {
        HttpConfig {
            content_type: content_type,
            redirect_policy: RedirectPolicy::FollowAll,
            read_timeout: Some(Duration::from_secs(15)),
            write_timeout: Some(Duration::from_secs(15)),
            max_message_size: 4096,
            max_idle_connections: None,
        }
    }

    pub fn headers(&self) -> Headers {
        let mut headers = Headers::new();

        headers.set(ContentType(self.content_type.clone()));

        headers
    }
}

pub struct HttpTransport<E> {
    base: hyper::Url,
    config: HttpConfig,
    phantom: PhantomData<E>,
}

impl<E> HttpTransport<E> {
    pub fn new(base: &str, config: HttpConfig) -> Result<Self> {
        Ok(HttpTransport {
               base: hyper::Url::parse(base)?,
               config: config,
               phantom: PhantomData,
           })
    }
}

impl<B: AsRef<[u8]>, E> Transport<B> for HttpTransport<E>
    where E: 'static + From<::hyper::Error> + From<Error> + Send
{
    type Output = ();
    type Error = E;

    fn send(&mut self, buf: &B) -> ::std::result::Result<(), Self::Error> {
        let mut client =
            self.config
                .max_idle_connections
                .map_or_else(|| Client::new(), |max_idle| {
                    Client::with_pool_config(pool::Config { max_idle: max_idle })
                });

        client.set_redirect_policy(self.config.redirect_policy);
        client.set_read_timeout(self.config.read_timeout);
        client.set_write_timeout(self.config.write_timeout);

        let res = client
            .post(self.base.clone())
            .body(buf.as_ref())
            .headers(self.config.headers())
            .send()?;

        if res.status != hyper::Ok {
            let err: Error = ErrorKind::ResponseError(res.status).into();

            bail!(err)
        } else {
            Ok(())
        }
    }
}
