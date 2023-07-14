#[allow(dead_code)]
use dto_helpers::IntoProto;

mod sub {
    pub struct RunAt {
        pub run_at: Vec<String>,
    }
    pub struct Cron {
        pub pattern: String,
    }
    pub mod schedule {
        pub enum Schedule {
            Cron(super::Cron),
            RunAt(super::RunAt),
        }
    }
    pub struct Schedule {
        pub schedule: Option<schedule::Schedule>,
    }
}

#[derive(Debug, IntoProto)]
#[into_proto(into = "sub::Cron")]
pub struct Cron {
    pub pattern: String,
}

#[derive(Debug, IntoProto)]
#[into_proto(into = "sub::RunAt")]
pub struct RunAt {
    #[into_proto(into = "run_at")]
    pub timepoints: Vec<String>,
}

#[derive(Debug, IntoProto)]
#[into_proto(into = "sub::Schedule", oneof = "schedule")]
pub enum Schedule {
    #[into_proto(into = "Cron")]
    Recurring(Cron),
    RunAt(RunAt),
}

fn main() {}
