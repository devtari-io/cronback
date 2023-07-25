pub use sea_orm_migration::prelude::*;

mod m20230712_205649_add_projects_model;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20230712_205649_add_projects_model::Migration)]
    }
}
