pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_api_keys_table;
mod m20230520_213613_create_attempts;
mod m20230521_221728_create_runs;
mod m20230521_233041_create_triggers;
mod m20230712_205649_add_projects_model;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_api_keys_table::Migration),
            Box::new(m20230520_213613_create_attempts::Migration),
            Box::new(m20230521_221728_create_runs::Migration),
            Box::new(m20230521_233041_create_triggers::Migration),
            Box::new(m20230712_205649_add_projects_model::Migration),
        ]
    }
}
