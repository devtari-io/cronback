use crate::model::define_model_id;

define_model_id! {
    #[prefix = "prj"]
    #[no_owner]
    pub struct ProjectId;
}

define_model_id! {
    #[prefix = "trig"]
    pub struct TriggerId;
}

define_model_id! {
    #[prefix = "inv"]
    pub struct InvocationId;
}

define_model_id! {
    #[prefix = "att"]
    pub struct AttemptLogId;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ShardedId;

    #[test]
    fn test_id_sharding() {
        let project = ProjectId::new();
        let project_shard = project.shard();

        let trigger = TriggerId::new(&project);
        assert_eq!(trigger.shard(), project_shard);
    }
}
