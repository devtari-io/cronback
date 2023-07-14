use dto::IntoProto;

mod sub {
    pub struct Recurring {
        pub cron: String,
        pub timezone: String,
        pub limit: u64,
        pub remaining: u64,
        pub data: Vec<String>,
    }
}

mod subsub {
    pub fn to_string(d: &i32) -> String {
        d.to_string()
    }
}

#[derive(IntoProto, Debug, Clone, PartialEq)]
#[proto(target = "sub::Recurring")]
pub struct Recurring {
    #[proto(required)]
    pub cron: Option<String>,
    pub timezone: String,
    pub limit: u64,
    // restricted but will still be converted.
    pub(crate) remaining: u64,
    #[proto(skip)]
    pub stuff: String,
    #[into_proto(map = "subsub::to_string", map_by_ref)]
    pub data: Vec<i32>,
    // non-public will not be included in the proto
    internal: String,
}

fn main() {}
