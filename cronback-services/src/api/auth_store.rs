use async_trait::async_trait;
use lib::database::models::api_keys;
use lib::database::models::prelude::ApiKeys;
use lib::database::{Database, DatabaseError};
use lib::model::ValidShardedId;
use lib::types::ProjectId;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub type AuthStoreError = DatabaseError;

#[async_trait]
pub trait AuthStore {
    async fn save_key(
        &self,
        key: api_keys::Model,
    ) -> Result<(), AuthStoreError>;

    async fn get_key(
        &self,
        key: &str,
    ) -> Result<Option<api_keys::Model>, AuthStoreError>;

    /// Returns true if the key got deleted, false if the key didn't exist
    async fn delete_key(
        &self,
        key_id: &str,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<bool, AuthStoreError>;

    async fn list_keys(
        &self,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<Vec<api_keys::Model>, AuthStoreError>;
}

pub struct SqlAuthStore {
    db: Database,
}

impl SqlAuthStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AuthStore for SqlAuthStore {
    async fn save_key(
        &self,
        key: api_keys::Model,
    ) -> Result<(), AuthStoreError> {
        let active_model: api_keys::ActiveModel = key.into();
        api_keys::Entity::insert(active_model)
            .exec(&self.db.orm)
            .await?;
        Ok(())
    }

    async fn get_key(
        &self,
        key_id: &str,
    ) -> Result<Option<api_keys::Model>, AuthStoreError> {
        let res = ApiKeys::find_by_id(key_id).one(&self.db.orm).await?;
        Ok(res)
    }

    async fn delete_key(
        &self,
        key_id: &str,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<bool, AuthStoreError> {
        let res = ApiKeys::delete_many()
            .filter(api_keys::Column::KeyId.eq(key_id))
            .filter(api_keys::Column::ProjectId.eq(project.clone()))
            .exec(&self.db.orm)
            .await?;
        Ok(res.rows_affected > 0)
    }

    async fn list_keys(
        &self,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<Vec<api_keys::Model>, AuthStoreError> {
        let results = ApiKeys::find()
            .filter(api_keys::Column::ProjectId.eq(project.clone()))
            .all(&self.db.orm)
            .await?;
        Ok(results)
    }
}

#[cfg(test)]
mod tests {

    use chrono::Utc;
    use lib::database::models::api_keys::{self, Metadata};
    use lib::database::Database;
    use lib::prelude::ValidShardedId;
    use lib::types::ProjectId;

    use super::{AuthStore, SqlAuthStore};

    fn build_model(
        key_id: &str,
        project: &ValidShardedId<ProjectId>,
    ) -> api_keys::Model {
        api_keys::Model {
            key_id: key_id.to_string(),
            hash: "hashash".to_string(),
            hash_version: "v1".to_string(),
            project_id: project.clone(),
            name: key_id.to_string(),
            created_at: Utc::now(),
            metadata: Metadata {
                creator_user_id: None,
            },
        }
    }

    #[tokio::test]
    async fn test_sql_auth_store() -> anyhow::Result<()> {
        let db = Database::in_memory().await?;
        let store = SqlAuthStore::new(db);

        let owner1 = ProjectId::generate();
        let owner2 = ProjectId::generate();

        let key1 = build_model("key1", &owner1);
        let key2 = build_model("key2", &owner2);
        let key3 = build_model("key3", &owner1);
        let key4 = build_model("key4", &owner2);

        // Test save keys
        store.save_key(key1.clone()).await?;
        store.save_key(key2.clone()).await?;
        store.save_key(key3.clone()).await?;
        store.save_key(key4.clone()).await?;

        // Test find owner by key
        assert_eq!(Some(&key1), store.get_key("key1").await?.as_ref());
        assert_eq!(Some(&key2), store.get_key("key2").await?.as_ref());
        assert_eq!(Some(&key3), store.get_key("key3").await?.as_ref());
        assert_eq!(Some(&key4), store.get_key("key4").await?.as_ref());
        assert_eq!(None, store.get_key("notfound").await?);

        // Test delete key
        assert!(store.delete_key("key1", &owner1).await?);
        assert_eq!(None, store.get_key("key1").await?);

        // After deletion, other keys should continue to work
        assert_eq!(Some(key2), store.get_key("key2").await?);

        // Deleting an already deleted key should return false.
        assert!(!store.delete_key("key1", &owner1).await?);

        // Deleting a key with the wrong project returns false
        assert!(!store.delete_key("key4", &owner1).await?);
        assert_eq!(Some(key4), store.get_key("key4").await?);

        // Test List keys
        assert_eq!(
            store
                .list_keys(&owner1)
                .await?
                .into_iter()
                .map(|k| k.name)
                .collect::<Vec<_>>(),
            vec!["key3".to_string()]
        );
        assert_eq!(
            store
                .list_keys(&owner2)
                .await?
                .into_iter()
                .map(|k| k.name)
                .collect::<Vec<_>>(),
            vec!["key2".to_string(), "key4".to_string()]
        );

        Ok(())
    }
}
