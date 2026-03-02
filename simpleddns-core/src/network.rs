/// Check if an IPv6 address is a unicast global address.
pub fn is_unicast_global(ip: &std::net::Ipv6Addr) -> bool {
    !ip.is_loopback() && !ip.is_multicast() && (ip.segments()[0] & 0xffc0) != 0xfe80
}

/// Pick the first network interface that has a global unicast IPv6 address.
pub fn pick_default_ipv6_interface() -> Option<String> {
    let addrs = get_if_addrs::get_if_addrs().ok()?;
    let mut names: Vec<String> = addrs
        .into_iter()
        .filter_map(|iface| match iface.addr.ip() {
            std::net::IpAddr::V6(ip) if !ip.is_loopback() && is_unicast_global(&ip) => {
                Some(iface.name)
            }
            _ => None,
        })
        .collect();
    names.sort();
    names.dedup();
    names.into_iter().next()
}

/// Parse a fully-qualified domain name into `(zone_name, sub_domain)`.
///
/// Examples:
/// - `"www.example.com"` → `("example.com", "www")`
/// - `"example.com"` → `("example.com", "@")`
/// - `"sub.deep.example.com"` → `("example.com", "sub.deep")`
pub fn parse_domain_parts(domain: &str) -> (String, String) {
    let parts: Vec<&str> = domain.rsplitn(3, '.').collect();
    if parts.len() >= 3 {
        let zone = format!("{}.{}", parts[1], parts[0]);
        let sub = parts[2..].iter().copied().rev().collect::<Vec<_>>().join(".");
        (zone, sub)
    } else if parts.len() >= 2 {
        let zone = format!("{}.{}", parts[1], parts[0]);
        (zone, "@".to_string())
    } else {
        (domain.to_string(), "@".to_string())
    }
}
