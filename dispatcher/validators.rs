use std::net::ToSocketAddrs;

use anyhow::{anyhow, Result};
use proto::trigger_proto::{endpoint::Endpoint, Webhook};
use url::Url;

fn validate_endpoint_url_parsable(url: &String) -> Result<Url> {
    Url::parse(url).map_err(|e| anyhow!("Failed to parse endpoint URL '{}': {} ", url, e))
}

fn validate_endpoint_scheme(scheme: &str) -> Result<()> {
    if scheme == "http" || scheme == "https" {
        Ok(())
    } else {
        Err(anyhow!("Endpoint scheme '{scheme}' not supported"))
    }
}

fn validate_endpoint_url_public_ip(host: Option<&str>) -> Result<()> {
    let host = host.ok_or(anyhow!("The endpoint must contain a host"))?;

    // This function does the DNS resolution. Unfortunately, it's synchronous.
    let _addrs = format!("{host}:80")
        // TODO: Replace with non-blocking nameservice lookup
        .to_socket_addrs()
        .map_err(|e| anyhow!("Failed to resolve DNS for endpoint: {}", e))?;

    // To error on the safe side, a hostname is valid if ALL its IPs are publicly addressable.

    // for addr in addrs {
    // match addr.ip() {
    //     std::net::IpAddr::V4(ip) => {
    //         if !ip.is_global() {
    //             return Err(anyhow!(
    //                 "The endpoint's IP is not globally reachable (e.g. in private IP space)"
    //             ));
    //         }
    //     }
    //     std::net::IpAddr::V6(ip) => {
    //         if !ip.is_global() {
    //             return Err(anyhow!(
    //                 "The endpoint's IP is not globally reachable (e.g. in private IP space)"
    //             ));
    //         }
    //     }
    // }
    // }
    Ok(())
}

pub(crate) fn validate_dispatch_request(request: &proto::event_proto::Request) -> Result<()> {
    let url_string = match request
        .endpoint
        .as_ref()
        .unwrap()
        .endpoint
        .as_ref()
        .unwrap()
    {
        Endpoint::Webhook(Webhook { url, .. }) => url,
    };

    let url = validate_endpoint_url_parsable(url_string)?;
    validate_endpoint_scheme(url.scheme())?;
    validate_endpoint_url_public_ip(url.host_str())?;

    Ok(())
}

#[cfg(test)]
mod tests {

    // use std::assert_matches::assert_matches;
    //
    // use prost_types::Duration;
    // use proto::trigger_proto::{
    //     endpoint::Endpoint, Endpoint as EndpointStruct, HttpMethod, Webhook,
    // };
    //
    // use super::validate_dispatch_request;
    //
    // fn build_request_from_url(url: &str) -> proto::event_proto::Request {
    //     proto::event_proto::Request {
    //         endpoint: Some(EndpointStruct {
    //             endpoint: Some(Endpoint::Webhook(Webhook {
    //                 http_method: HttpMethod::Get.into(),
    //                 url: url.to_string(),
    //             })),
    //         }),
    //         request_payload: None,
    //         timeout: Some(Duration::default()),
    //     }
    // }

    // #[test]
    // fn valid_urls() {
    //     let urls = vec![
    //         "https://google.com/url",
    //         "https://example.com:3030/url",
    //         "https://1.1.1.1/url",
    //         "http://[2606:4700:4700::1111]/another_url/path",
    //         "http://[2606:4700:4700::1111]:5050/another_url/path",
    //         "http://user:pass@google.com/another_url/path",
    //     ];
    //
    //     for url in urls {
    //         assert_matches!(
    //             validate_dispatch_request(&build_request_from_url(url)),
    //             Ok(()),
    //             "URL: {}",
    //             url
    //         );
    //     }
    // }
    //
    // #[test]
    // fn invalid_urls() {
    //     let urls = vec![
    //         // Private IPs
    //         "https://10.0.10.1",
    //         "https://192.168.1.1",
    //         "https://[::1]:80",
    //         // Non-http url
    //         "ftp://google.com",
    //         // Lookback address
    //         "https://localhost/url",
    //         // Scheme-less
    //         "google.com/url",
    //         // Unparsable URL
    //         "http---@goog.com",
    //         // Non-existent domains
    //         "https://ppqqzonlnp.io/url/url",
    //     ];
    //
    //     for url in urls {
    //         assert_matches!(
    //             validate_dispatch_request(&build_request_from_url(url)),
    //             Err(_),
    //             "URL: {}",
    //             url
    //         );
    //     }
    // }
}
