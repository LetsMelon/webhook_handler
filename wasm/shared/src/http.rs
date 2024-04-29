use std::fmt::Debug;

use http::{Method, Version};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(C)]
/// Enum to differentiate between all possible http methods
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

impl TryFrom<&Method> for HttpMethod {
    type Error = anyhow::Error;

    fn try_from(value: &Method) -> Result<Self, Self::Error> {
        match value.as_str() {
            "OPTIONS" => Ok(HttpMethod::OPTIONS),
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            "PUT" => Ok(HttpMethod::PUT),
            "DELETE" => Ok(HttpMethod::DELETE),
            "HEAD" => Ok(HttpMethod::HEAD),
            "TRACE" => Ok(HttpMethod::TRACE),
            "CONNECT" => Ok(HttpMethod::CONNECT),
            "PATCH" => Ok(HttpMethod::PATCH),
            name => Err(anyhow::anyhow!("Unknown http method: '{:?}'", name)),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(C)]
/// Enum to differentiate between all possible http versions
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

impl TryFrom<Version> for HttpVersion {
    type Error = anyhow::Error;

    fn try_from(value: Version) -> Result<Self, Self::Error> {
        match value {
            Version::HTTP_09 => Ok(HttpVersion::Http0_9),
            Version::HTTP_10 => Ok(HttpVersion::Http1_0),
            Version::HTTP_11 => Ok(HttpVersion::Http1_1),
            Version::HTTP_2 => Ok(HttpVersion::Http2),
            Version::HTTP_3 => Ok(HttpVersion::Http3),
            version => Err(anyhow::anyhow!("Unknown http version: '{:?}'", version)),
        }
    }
}
