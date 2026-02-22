use crate::models::IpVersion;
use reqwest::Client;
use std::net::IpAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] std::net::AddrParseError),
    #[error("Invalid IP version returned")]
    VersionMismatch,
    #[error("No suitable local address found")]
    NoLocalAddress,
}

pub async fn resolve_ip(
    client: &Client,
    url: &str,
    expected_version: IpVersion,
) -> Result<IpAddr, ResolverError> {
    if url == "local://ipv4" || url == "local://ipv6" {
        return resolve_local(expected_version);
    }

    let response = client.get(url).send().await?.text().await?;
    let ip: IpAddr = response.trim().parse()?;

    match (expected_version, ip) {
        (IpVersion::IPv4, IpAddr::V4(_)) => Ok(ip),
        (IpVersion::IPv6, IpAddr::V6(_)) => Ok(ip),
        _ => Err(ResolverError::VersionMismatch),
    }
}

fn resolve_local(version: IpVersion) -> Result<IpAddr, ResolverError> {
    // Use a UDP socket trick to find the default outbound address
    // This doesn't actually send anything over the network
    match version {
        IpVersion::IPv4 => {
            let socket = std::net::UdpSocket::bind("0.0.0.0:0")
                .map_err(|_| ResolverError::NoLocalAddress)?;
            socket
                .connect("8.8.8.8:80")
                .map_err(|_| ResolverError::NoLocalAddress)?;
            let addr = socket
                .local_addr()
                .map_err(|_| ResolverError::NoLocalAddress)?;
            Ok(addr.ip())
        }
        IpVersion::IPv6 => {
            let socket =
                std::net::UdpSocket::bind("[::]:0").map_err(|_| ResolverError::NoLocalAddress)?;
            socket
                .connect("[2001:4860:4860::8888]:80")
                .map_err(|_| ResolverError::NoLocalAddress)?;
            let addr = socket
                .local_addr()
                .map_err(|_| ResolverError::NoLocalAddress)?;
            Ok(addr.ip())
        }
    }
}
