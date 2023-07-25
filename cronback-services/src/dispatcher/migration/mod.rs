pub use sea_orm_migration::prelude::*;

mod m20230520_213613_create_attempts;
mod m20230521_221728_create_runs;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20230520_213613_create_attempts::Migration),
            Box::new(m20230521_221728_create_runs::Migration),
        ]
    }
}
