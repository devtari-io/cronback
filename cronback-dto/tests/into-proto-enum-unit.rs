use cronback_dto::IntoProto;

mod sub {
    #[repr(i32)]
    pub enum HttpMethod {
        Get,
        Post,
        Put,
    }
}

#[derive(IntoProto)]
#[proto(target = "sub::HttpMethod")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
}

fn main() {}
