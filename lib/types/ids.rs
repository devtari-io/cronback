use crate::model::define_model_id;

define_model_id! {
    @prefix = "prj",
    @no_owner,
    @proto = proto::common::ProjectId,
    pub struct ProjectId;
}

define_model_id! {
    @prefix = "trig",
    @proto = proto::common::TriggerId,
    pub struct TriggerId;
}

define_model_id! {
    @prefix = "inv",
    @proto = proto::common::RunId,
    pub struct RunId;
}

define_model_id! {
    @prefix = "att",
    @proto = proto::common::AttemptLogId,
    pub struct AttemptLogId;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_sharding() -> Result<(), crate::model::ModelIdError> {
        let project = ProjectId::generate();
        let project_shard = project.shard();

        let trigger = TriggerId::generate(&project);
        assert_eq!(trigger.shard(), project_shard);
        Ok(())
    }
}
