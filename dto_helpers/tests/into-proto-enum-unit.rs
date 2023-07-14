#[allow(dead_code)]
use dto_helpers::IntoProto;

mod sub {
    #[repr(i32)]
    pub enum HttpMethod {
        Get,
        Post,
        Put,
    }
}

#[derive(IntoProto)]
#[into_proto(into = "sub::HttpMethod")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
}

fn main() {}
