use dto_helpers::IntoProto;

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
#[into_proto(into = "sub::Recurring")]
pub struct Recurring {
    #[into_proto(required)]
    pub cron: Option<String>,
    pub timezone: String,
    pub limit: u64,
    // restricted but will still be converted.
    pub(crate) remaining: u64,
    #[into_proto(skip)]
    pub stuff: String,
    #[into_proto(map_fn = "subsub::to_string")]
    pub data: Vec<i32>,
    // non-public will not be included in the proto
    internal: String,
}

fn main() {}
