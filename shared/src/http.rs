use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub enum HttpMethod {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
    PATCH,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]

pub enum HttpVersion {
    Http0_9,
    Http1_0,
    Http1_1,
    Http2,
    Http3,
}

impl Debug for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http0_9 => write!(f, "HTTP/0.9"),
            Self::Http1_0 => write!(f, "HTTP/1.0"),
            Self::Http1_1 => write!(f, "HTTP/1.1"),
            Self::Http2 => write!(f, "HTTP/2"),
            Self::Http3 => write!(f, "HTTP/3"),
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub enum MiddlewareResult {
    Continue = 0,
    Error,
}
