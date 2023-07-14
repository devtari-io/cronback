use cronback_dto::FromProto;

mod sub {
    pub struct Output {
        pub name: String,
    }
}

#[derive(FromProto)]
#[proto(target = "sub::Output")]
pub struct Output {
    #[from_proto(map = "perform")]
    pub name: String,
}

fn perform(name: String) -> String {
    String::to_string(&name)
}

fn main() {}
