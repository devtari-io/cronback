use anyhow::Result;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

pub fn parse_addr<A>(address: A, port: u16) -> Result<SocketAddr>
where
    A: AsRef<str>,
{
    let addr = IpAddr::from_str(address.as_ref())?;

    Ok(SocketAddr::from((addr, port)))
}
