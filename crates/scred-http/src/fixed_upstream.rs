use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedUpstream {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub base_path: String,
}

impl FixedUpstream {
    pub fn parse(url: &str) -> Result<Self> {
        let (scheme, rest) = url
            .split_once("://")
            .ok_or_else(|| anyhow!("upstream URL must include scheme (http:// or https://)"))?;

        let default_port = match scheme {
            "http" => 80,
            "https" => 443,
            _ => return Err(anyhow!("unsupported upstream scheme: {}", scheme)),
        };

        let (host_port, path) = match rest.find('/') {
            Some(idx) => (&rest[..idx], &rest[idx..]),
            None => (rest, "/"),
        };

        let (host, port) = match host_port.rsplit_once(':') {
            Some((host, port)) if !host.is_empty() => {
                let port = port.parse::<u16>().map_err(|_| anyhow!("invalid upstream port"))?;
                (host.to_string(), port)
            }
            _ => (host_port.to_string(), default_port),
        };

        if host.is_empty() {
            return Err(anyhow!("upstream host cannot be empty"));
        }

        Ok(Self {
            scheme: scheme.to_string(),
            host,
            port,
            base_path: normalize_base_path(path),
        })
    }

    pub fn authority(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn rewrite_request_line(&self, request_line: &str) -> Result<String> {
        let mut parts = request_line.split_whitespace();
        let method = parts.next().ok_or_else(|| anyhow!("invalid request line"))?;
        let target = parts.next().ok_or_else(|| anyhow!("invalid request line"))?;
        let version = parts.next().ok_or_else(|| anyhow!("invalid request line"))?;

        let rewritten_target = self.join_target(target);
        Ok(format!("{} {} {}", method, rewritten_target, version))
    }

    fn join_target(&self, target: &str) -> String {
        if target.starts_with("http://") || target.starts_with("https://") {
            return target.to_string();
        }

        let relative = if target.is_empty() { "/" } else { target };
        if self.base_path == "/" {
            return relative.to_string();
        }

        if relative == "/" {
            return self.base_path.clone();
        }

        format!("{}{}", self.base_path.trim_end_matches('/'), relative)
    }
}

fn normalize_base_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.trim_end_matches('/').to_string().if_empty_then_root()
    } else {
        format!("/{}", path).trim_end_matches('/').to_string().if_empty_then_root()
    }
}

trait RootPathExt {
    fn if_empty_then_root(self) -> String;
}

impl RootPathExt for String {
    fn if_empty_then_root(self) -> String {
        if self.is_empty() { "/".to_string() } else { self }
    }
}

