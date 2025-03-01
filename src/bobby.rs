use hyper::{header, service::service_fn};
use hyper_util::{
    rt::TokioIo,
    server::conn::auto::{self},
};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::net::TcpListener;

#[derive(Clone)]
struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}

impl TokioExecutor {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct Request {
    method: hyper::Method,
    uri: hyper::Uri,
    params: HashMap<String, String>,
}

impl Request {
    pub fn new(request: &hyper::Request<hyper::body::Incoming>) -> Self {
        Request {
            method: request.method().clone(),
            uri: request.uri().clone(),
            params: HashMap::new(),
        }
    }

    pub fn method(&self) -> &hyper::Method {
        &self.method
    }

    pub fn uri(&self) -> &hyper::Uri {
        &self.uri
    }

    pub fn param(&self, name: &str) -> Option<&String> {
        self.params.get(name)
    }
}

pub enum ResponseError {
    CannotGetHeaders,
    InvalidHeaderName,
    InvalidHeaderValue,
    FailedToCreateHeader,
}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseError::CannotGetHeaders => write!(f, "Cannot get response headers"),
            ResponseError::InvalidHeaderName => write!(f, "Invalid header name"),
            ResponseError::InvalidHeaderValue => write!(f, "Invalid header value"),
            ResponseError::FailedToCreateHeader => write!(f, "Failed to create header"),
        }
    }
}

impl std::fmt::Debug for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for ResponseError {}

#[derive(Clone)]
pub struct Response {
    body: String,
    status: u16,
    headers: HashMap<String, String>,
}

impl Response {
    pub fn html(body: impl Into<String>) -> Self {
        Response {
            body: body.into(),
            status: 200,
            headers: HashMap::from([(String::from("Content-Type"), String::from("text/html"))]),
        }
    }

    pub fn with_status(self, status: u16) -> Self {
        let mut response = self.clone();

        response.status = status;

        response
    }

    pub fn with_header(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut response = self.clone();

        response.headers.insert(key.into(), value.into());

        response
    }

    pub fn build(self) -> Result<hyper::Response<String>, ResponseError> {
        let mut builder = hyper::Response::builder().status(self.status);
        let headers = builder
            .headers_mut()
            .ok_or_else(|| ResponseError::CannotGetHeaders)?;

        // construct headers
        for (k, v) in self.headers.into_iter() {
            let header_name = header::HeaderName::from_bytes(k.as_bytes())
                .map_err(|_| ResponseError::InvalidHeaderName)?;

            let header_value =
                header::HeaderValue::from_str(&v).map_err(|_| ResponseError::InvalidHeaderValue)?;

            headers.insert(header_name, header_value);
        }

        // add content length
        headers.insert(
            header::HeaderName::from_static("content-length"),
            header::HeaderValue::from_str(&self.body.len().to_string())
                .map_err(|_| ResponseError::FailedToCreateHeader)?,
        );

        // add body and return
        Ok(builder.body(self.body).unwrap())
    }
}

#[derive(Clone)]
pub struct Route {
    method: hyper::Method,
    path: String,
    callable: fn(req: Request) -> Response,
}

#[derive(Clone)]
pub struct Bobby {
    ip: IpAddr,
    port: u16,
    routes: Vec<Route>,
}

impl Bobby {
    pub fn new() -> Bobby {
        Bobby {
            ip: IpAddr::from([127, 0, 0, 1]),
            port: 8080,
            routes: vec![],
        }
    }

    pub fn with_address(&mut self, ip: impl Into<IpAddr>, port: u16) {
        self.ip = ip.into();
        self.port = port;
    }

    pub fn get(&mut self, path: impl Into<String>, callable: fn(req: Request) -> Response) {
        self.routes.push(Route {
            method: hyper::Method::GET,
            path: path.into(),
            callable,
        });
    }

    pub fn post(&mut self, path: impl Into<String>, callable: fn(req: Request) -> Response) {
        self.routes.push(Route {
            method: hyper::Method::POST,
            path: path.into(),
            callable,
        });
    }

    pub fn put(&mut self, path: impl Into<String>, callable: fn(req: Request) -> Response) {
        self.routes.push(Route {
            method: hyper::Method::PUT,
            path: path.into(),
            callable,
        });
    }

    pub fn delete(&mut self, path: impl Into<String>, callable: fn(req: Request) -> Response) {
        self.routes.push(Route {
            method: hyper::Method::DELETE,
            path: path.into(),
            callable,
        });
    }

    pub fn patch(&mut self, path: impl Into<String>, callable: fn(req: Request) -> Response) {
        self.routes.push(Route {
            method: hyper::Method::PATCH,
            path: path.into(),
            callable,
        });
    }

    pub fn options(&mut self, path: impl Into<String>, callable: fn(req: Request) -> Response) {
        self.routes.push(Route {
            method: hyper::Method::OPTIONS,
            path: path.into(),
            callable,
        });
    }

    pub fn head(&mut self, path: impl Into<String>, callable: fn(req: Request) -> Response) {
        self.routes.push(Route {
            method: hyper::Method::HEAD,
            path: path.into(),
            callable,
        });
    }

    fn log_request(&self, request: &hyper::Request<hyper::body::Incoming>) {
        println!(
            "{http:?} {method} {path}",
            http = request.version(),
            method = request.method(),
            path = request.uri()
        );
    }

    fn uri_matches_path(&self, uri: &hyper::Uri, path: &str) -> bool {
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let uri_parts: Vec<&str> = uri.path().split('/').filter(|s| !s.is_empty()).collect();

        if uri_parts.len() > path_parts.len() {
            return false;
        }

        for (i, path_part) in path_parts.iter().enumerate() {
            let is_param = path_part.starts_with('{') && path_part.ends_with('}');
            let is_optional_param = is_param && path_part.ends_with("?}");

            if i >= uri_parts.len() {
                return is_optional_param;
            }

            if !is_param && uri_parts[i] != *path_part {
                return false;
            }

            if is_param && !is_optional_param && uri_parts[i].is_empty() {
                return false;
            }
        }

        uri_parts.len() <= path_parts.len()
    }

    fn route(
        &self,
        _req: &hyper::Request<hyper::body::Incoming>,
    ) -> Result<hyper::Response<String>, ResponseError> {
        // attempt to find a matching route
        for route in &self.routes {
            if _req.method() == route.method && self.uri_matches_path(_req.uri(), &route.path) {
                let mut req = Request::new(_req);

                if let Some(params) = self.extract_params(_req.uri(), &route.path) {
                    req.params = params;
                }

                let response = (route.callable)(req);

                return response.build();
            }
        }

        // no matching route found
        Response::html("Not found.").with_status(404).build()
    }

    fn extract_params(&self, uri: &hyper::Uri, path: &str) -> Option<HashMap<String, String>> {
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let uri_parts: Vec<&str> = uri.path().split('/').filter(|s| !s.is_empty()).collect();
        let mut params = HashMap::new();

        for (i, path_part) in path_parts.iter().enumerate() {
            if path_part.starts_with('{') && path_part.ends_with('}') {
                let param_name = if path_part.ends_with("?}") {
                    &path_part[1..path_part.len() - 2]
                } else {
                    &path_part[1..path_part.len() - 1]
                };

                if i < uri_parts.len() {
                    params.insert(String::from(param_name), String::from(uri_parts[i]));
                }
            }
        }

        Some(params)
    }

    async fn listen(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = SocketAddr::from((self.ip, self.port));
        let listener = TcpListener::bind(addr).await?;
        let bobby_arc = Arc::new(self.clone());

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let bobby = Arc::clone(&bobby_arc);

            tokio::task::spawn(async move {
                let service = service_fn(move |request| {
                    let bobby_ref = Arc::clone(&bobby);

                    async move {
                        bobby_ref.log_request(&request);
                        bobby_ref.route(&request)
                    }
                });

                if let Err(err) = auto::Builder::new(TokioExecutor::new())
                    .serve_connection(io, service)
                    .await
                {
                    eprintln!("Error: {}", err);
                }
            });
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let rt = tokio::runtime::Runtime::new()?;
        println!("Listening on {}:{} ...", self.ip, self.port);
        rt.block_on(self.listen())
    }
}
