use anyhow::Result;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::str::FromStr;

pub fn parse_addr(address: &Option<String>, port: u16) -> Result<SocketAddr> {
    let addr = match address {
        Some(addr) => IpAddr::from_str(addr)?,
        _ => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    };

    Ok(SocketAddr::from((addr, port)))
}
