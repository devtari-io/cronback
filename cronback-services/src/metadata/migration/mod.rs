pub use sea_orm_migration::prelude::*;

mod m20230712_205649_add_projects_model;
mod m20230726_115454_add_notification_settings;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20230712_205649_add_projects_model::Migration),
            Box::new(m20230726_115454_add_notification_settings::Migration),
        ]
    }
}
