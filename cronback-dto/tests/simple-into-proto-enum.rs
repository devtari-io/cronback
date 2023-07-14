#[allow(dead_code)]
use cronback_dto::{FromProto, IntoProto};

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

#[derive(Debug, IntoProto, FromProto)]
#[proto(target = "sub::Cron")]
pub struct Cron {
    pub pattern: String,
}

#[derive(Debug, IntoProto, FromProto)]
#[proto(target = "sub::RunAt")]
pub struct RunAt {
    #[proto(name = "run_at")]
    pub timepoints: Vec<String>,
}

#[derive(Debug, IntoProto, FromProto)]
#[proto(target = "sub::Schedule", oneof = "schedule")]
pub enum Schedule {
    #[proto(name = "Cron")]
    Recurring(Cron),
    RunAt(RunAt),
}

fn main() {}
