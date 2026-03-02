use crate::models::IpVersion;
use crate::network::is_unicast_global;
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
    if let Some(iface_name) = url.strip_prefix("interface://") {
        return resolve_interface(iface_name, expected_version);
    }
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
fn resolve_interface(iface_name: &str, version: IpVersion) -> Result<IpAddr, ResolverError> {
    let addrs = get_if_addrs::get_if_addrs().map_err(|_| ResolverError::NoLocalAddress)?;
    for iface in addrs {
        // Compare names case-insensitively just in case Windows GUIDs differ in casing
        if iface.name.eq_ignore_ascii_case(iface_name) && !iface.addr.ip().is_loopback() {
            match (version, iface.addr.ip()) {
                (IpVersion::IPv4, IpAddr::V4(ip)) => return Ok(IpAddr::V4(ip)),
                (IpVersion::IPv6, IpAddr::V6(ip)) => {
                    if is_unicast_global(&ip) {
                        return Ok(IpAddr::V6(ip));
                    }
                }
                _ => continue,
            }
        }
    }
    Err(ResolverError::NoLocalAddress)
}
fn resolve_local(version: IpVersion) -> Result<IpAddr, ResolverError> {
    // Use a UDP socket trick to find the default outbound address
    // This doesn't actually send anything over the network
    match version {
        IpVersion::IPv4 => {
            let socket = std::net::UdpSocket::bind("0.0.0.0:0")
                .map_err(|_| ResolverError::NoLocalAddress)?;
            let _ = socket.set_read_timeout(Some(std::time::Duration::from_secs(3)));
            let _ = socket.set_write_timeout(Some(std::time::Duration::from_secs(3)));
            socket
                .connect("8.8.8.8:80")
                .map_err(|_| ResolverError::NoLocalAddress)?;
            let addr = socket
                .local_addr()
                .map_err(|_| ResolverError::NoLocalAddress)?;
            Ok(addr.ip())
        }
        IpVersion::IPv6 => {
            let socket = std::net::UdpSocket::bind("[::]:0").map_err(|_| ResolverError::NoLocalAddress)?;
            let _ = socket.set_read_timeout(Some(std::time::Duration::from_secs(3)));
            let _ = socket.set_write_timeout(Some(std::time::Duration::from_secs(3)));
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
