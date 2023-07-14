use once_cell::sync::Lazy;
use url::Url;

pub static BASE_URL_ENV: &str = "CRONBACK_BASE_URL";
pub static DEFAULT_BASE_URL: Lazy<Url> = Lazy::new(|| {
    // Default in build is (jungle), production URL is only set if the build
    // explicitly sets CRONBACK_DEFAULT_BASE_URL at compile time.
    let url_str = std::option_env!("CRONBACK_DEFAULT_BASE_URL")
        .unwrap_or("https://api.jungle.cronback.me");
    Url::parse(url_str).expect("DEFAULT_BASE_URL")
});
