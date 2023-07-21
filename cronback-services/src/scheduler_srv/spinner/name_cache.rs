#![allow(unused)]
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::RwLock;

use dashmap::DashMap;
use lib::types::{ProjectId, TriggerId};
use tracing::trace;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Fetcher: Send + Sync /* + 'static */ {
    type Error;
    fn call(
        &self,
        project_id: ProjectId,
        name: String,
    ) -> BoxFuture<'static, Result<Option<TriggerId>, Self::Error>>;
}

impl<T, F, E> Fetcher for T
where
    T: FnOnce(ProjectId, String) -> F + Send + Sync + Clone + 'static,
    F: Future<Output = Result<Option<TriggerId>, E>> + Send + 'static,
{
    type Error = E;

    fn call(
        &self,
        project_id: ProjectId,
        name: String,
    ) -> BoxFuture<'static, Result<Option<TriggerId>, Self::Error>> {
        // We clone the closure on every call, so it needs to capture only
        // cloneable things.
        let cloned = self.clone();
        Box::pin(cloned(project_id, name))
    }
}

pub struct NameCache<E> {
    projects: DashMap<ProjectId, RwLock<HashMap<String, TriggerId>>>,
    fetcher: Box<dyn Fetcher<Error = E>>,
}

#[cfg(test)]
fn null_fetcher<E>(
    _project_id: ProjectId,
    _name: String,
) -> BoxFuture<'static, Result<Option<TriggerId>, E>> {
    Box::pin(async { Ok(None) })
}

#[cfg(test)]
impl NameCache<anyhow::Error> {
    pub fn without_fetcher() -> Self {
        Self {
            projects: DashMap::new(),
            fetcher: Box::new(null_fetcher),
        }
    }
}

impl<E> NameCache<E> {
    pub fn new(fetcher: Box<dyn Fetcher<Error = E>>) -> Self {
        Self {
            projects: DashMap::new(),
            fetcher,
        }
    }

    /// Returns the trigger ID for the given project and name, if it exists.
    pub fn get_if_cached(
        &self,
        project_id: &ProjectId,
        name: impl AsRef<str>,
    ) -> Option<TriggerId> {
        let project_map = self.projects.get(project_id)?;
        let project_map = project_map.read().unwrap();
        project_map.get(name.as_ref()).cloned()
    }

    /// Returns the trigger ID for the given project and name. It fetches the
    /// TriggerId from the store if not cached.
    pub async fn get(
        &self,
        project_id: &ProjectId,
        name: impl AsRef<str>,
    ) -> Result<Option<TriggerId>, E> {
        // is cached?
        let cached = self.get_if_cached(project_id, name.as_ref());
        if let Some(trigger_id) = cached {
            trace!(
                project_id = ?project_id,
                name = ?name.as_ref(),
                trigger_id = ?trigger_id,
                "Cache hit! found cached trigger name"
            );
            Ok(Some(trigger_id))
        } else {
            // Use fetcher and insert into cache.
            let trigger_id = self
                .fetcher
                .call(project_id.clone(), name.as_ref().to_owned())
                .await?;

            // Cache it.
            let response = trigger_id.clone();

            if let Some(trigger_id) = trigger_id {
                trace!(
                    project_id = ?project_id,
                    name = ?name.as_ref(),
                    trigger_id = ?trigger_id,
                    "Cache miss! caching trigger name"
                );
                self.insert(
                    project_id.clone(),
                    name.as_ref().to_owned(),
                    trigger_id,
                );
            };
            Ok(response)
        }
    }

    /// Removes an entry from the cache and returns the previous value if it was
    /// present.
    pub fn remove(
        &self,
        project_id: &ProjectId,
        name: impl AsRef<str>,
    ) -> Option<TriggerId> {
        let project_map = self.projects.get(project_id)?;
        let mut project_map = project_map.write().unwrap();
        project_map.remove(name.as_ref())
    }

    /// Removes an entire project from cache.
    pub fn remove_project(&self, project_id: &ProjectId) {
        self.projects.remove(project_id);
    }

    /// Cleans up the cache by removing all projects that have no cached
    /// triggers
    pub fn compact(&self) {
        self.projects.retain(|_, v| !v.read().unwrap().is_empty());
    }

    /// The number of projects known to the cache.
    pub fn num_projects(&self) -> usize {
        self.projects.len()
    }

    /// Total size of all cached names.
    pub fn total_size(&self) -> usize {
        self.projects
            .iter()
            .map(|m| m.value().read().unwrap().len())
            .sum()
    }

    /// Inserts a new entry into the cache and returns the previous value if it
    /// was present.
    fn insert(
        &self,
        project_id: ProjectId,
        name: String,
        trigger_id: TriggerId,
    ) -> Option<TriggerId> {
        // Do we know this project already?
        let project_map =
            self.projects.entry(project_id).or_default().downgrade();

        let mut project_map = project_map.write().unwrap();
        project_map.insert(name, trigger_id)
    }
}

// Ensure that NameCache is Send + Sync. Compiler will fail if it's not.
const _: () = {
    fn assert_send<T: Send + Sync>() {}
    let _ = assert_send::<NameCache<anyhow::Error>>;
};

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use lib::types::TriggerId;

    use super::*;
    #[test]
    fn test_name_cache_basics() {
        let cache = NameCache::without_fetcher();
        let project_id1 = ProjectId::generate();
        let trigger_id1 = TriggerId::generate(&project_id1).into_inner();
        let trigger_id2 = TriggerId::generate(&project_id1).into_inner();
        let trigger_name1 = "test_trigger_name-1".to_string();
        let trigger_name2 = "test_trigger_name-2".to_string();
        assert_eq!(
            None,
            cache.insert(
                project_id1.inner().clone(),
                trigger_name1,
                trigger_id1.clone()
            )
        );
        assert_eq!(
            None,
            cache.insert(
                project_id1.inner().clone(),
                trigger_name2,
                trigger_id2.clone()
            )
        );

        assert_eq!(
            Some(trigger_id1),
            cache.get_if_cached(&project_id1, "test_trigger_name-1")
        );

        assert_eq!(
            Some(trigger_id2.clone()),
            cache.get_if_cached(&project_id1, "test_trigger_name-2")
        );

        assert_eq!(
            Some(trigger_id2),
            cache.get_if_cached(&project_id1, "test_trigger_name-2")
        );

        let project_id2 = ProjectId::generate();

        // We have nothing for project 2 yet.
        assert_eq!(
            None,
            cache.get_if_cached(&project_id2, "test_trigger_name-1")
        );
    }

    #[test]
    fn test_remove_and_compact() {
        let cache = NameCache::without_fetcher();
        let project_id1 = ProjectId::generate();
        let trigger_id1 = TriggerId::generate(&project_id1).into_inner();
        let trigger_name1 = "test_trigger_name-1".to_string();
        assert_eq!(0, cache.num_projects());
        cache.insert(
            project_id1.inner().clone(),
            trigger_name1.clone(),
            trigger_id1.clone(),
        );
        assert_eq!(1, cache.num_projects());
        cache.compact();
        // No impact, we still have a trigger.
        assert_eq!(1, cache.num_projects());

        // Put another project
        let project_id2 = ProjectId::generate();
        let trigger_id2 = TriggerId::generate(&project_id2).into_inner();
        cache.insert(
            project_id2.inner().clone(),
            // same name, that's okay!
            trigger_name1.clone(),
            trigger_id2,
        );
        assert_eq!(2, cache.num_projects());
        // also total size is 2
        assert_eq!(2, cache.total_size());
        cache.remove(&project_id2, &trigger_name1);
        assert_eq!(2, cache.num_projects());
        // Now compact, project2 should be gone.
        cache.compact();
        assert_eq!(1, cache.num_projects());

        // Making sure we still have the trigger for the first project
        assert_eq!(
            Some(trigger_id1),
            cache.get_if_cached(&project_id1, &trigger_name1)
        );
    }

    #[tokio::test]
    async fn test_name_cache_fether() -> Result<(), anyhow::Error> {
        let store = Arc::new(Mutex::new(HashMap::new()));
        let project_id1 = ProjectId::generate();
        let trigger_id1 = TriggerId::generate(&project_id1).into_inner();
        let trigger_name1 = "test_trigger_name-1".to_string();

        {
            // add trigger to the virtual store
            let mut w = store.lock().unwrap();
            w.insert(trigger_name1.clone(), trigger_id1.clone());
        }

        let store_cloned = store.clone();
        let fetcher = Box::new(|_project_id: ProjectId, name: String| {
            async move { Ok(store_cloned.lock().unwrap().get(&name).cloned()) }
        });

        let cache = NameCache::<anyhow::Error>::new(fetcher);

        assert_eq!(0, cache.total_size());
        // A miss first.
        assert_eq!(
            None,
            cache.get(project_id1.inner(), "does-not-exist").await?
        );
        // A cache miss, but fetcher hit.
        assert_eq!(
            Some(trigger_id1.clone()),
            cache.get(project_id1.inner(), &trigger_name1).await?
        );
        assert_eq!(1, cache.total_size());
        // Cache hit.
        assert_eq!(
            Some(trigger_id1.clone()),
            cache.get_if_cached(&project_id1, &trigger_name1)
        );

        // A cache hit.
        assert_eq!(
            Some(trigger_id1.clone()),
            cache.get(project_id1.inner(), &trigger_name1).await?
        );

        // Remove from cache and get again, it will hit the fetcher.
        //
        assert!(cache.remove(&project_id1, &trigger_name1).is_some());

        // A miss, but fetcher hit.
        assert_eq!(
            Some(trigger_id1),
            cache.get(project_id1.inner(), &trigger_name1).await?
        );

        Ok(())
    }
}
