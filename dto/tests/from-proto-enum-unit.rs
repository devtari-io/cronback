#[allow(dead_code)]
use dto::FromProto;

mod sub {
    #[repr(i32)]
    pub enum HttpMethod {
        Unknown = 0,
        Get = 1,
        Post = 2,
        Put = 3,
    }

    impl HttpMethod {
        pub fn from_i32(input: i32) -> Option<Self> {
            match input {
                | 0 => Some(Self::Unknown),
                | 1 => Some(Self::Get),
                | 2 => Some(Self::Post),
                | 3 => Some(Self::Put),
                | _ => None,
            }
        }
    }
}

#[derive(FromProto)]
#[proto(target = "sub::HttpMethod")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
}

fn main() {}
