use std::collections::HashMap;
use std::sync::Arc;

use proto::scheduler_proto::{GetTriggerRequest, InstallTriggerRequest};
use proto::trigger_proto::{self, Cron, Schedule};
use scheduler::test_helpers;
use shared::config::{ConfigLoader, Role};
use shared::service::ServiceContext;
use shared::shutdown::Shutdown;
use shared::types::*;
use tonic::Request;

#[tokio::test]
async fn install_trigger_valid_test() {
    let shutdown = Shutdown::default();
    let config_loader = Arc::new(ConfigLoader::from_path(&None));
    let context = ServiceContext::new(
        format!("{:?}", Role::Scheduler),
        config_loader,
        shutdown,
    );
    let (serve_future, mut client) =
        test_helpers::test_server_and_client(context).await;
    let install_trigger = InstallTriggerRequest {
        cell_id: 0,
        owner_id: "asoli".to_owned(),
        reference_id: None,
        name: None,
        description: None,
        emit: Vec::default(),
        payload: Some(proto::trigger_proto::Payload {
            content_type: "application/json".to_owned(),
            headers: HashMap::new(),
            body: "Hello World".into(),
        }),
        schedule: Some(Schedule {
            schedule: Some(trigger_proto::schedule::Schedule::Cron(Cron {
                cron: format!("0 * * * * *"),
                timezone: "Europe/London".into(),
                events_limit: 4,
            })),
        }),
    };
    let request_future = async {
        let installed_trigger = client
            .install_trigger(Request::new(install_trigger.into()))
            .await
            .unwrap()
            .into_inner();
        assert!(installed_trigger.trigger.is_some());
        let created_trigger = installed_trigger.trigger.unwrap();
        assert!(created_trigger.id.len() > 5);
        println!("{}", created_trigger.id);
        let created_trigger: Trigger = created_trigger.into();
        // Validate that the cron pattern is what we have set.
        // No errors. Let's try and get it from server.
        let response = client
            .get_trigger(Request::new(GetTriggerRequest {
                id: created_trigger.id.clone().into(),
            }))
            .await
            .unwrap()
            .into_inner();
        assert!(response.trigger.is_some());
        let trigger_retrieved: Trigger = response.trigger.unwrap().into();
        assert_eq!(trigger_retrieved.id, created_trigger.id);
    };

    // Wait for completion, when the client request future completes
    tokio::select! {
        _ = serve_future => panic!("server returned first"),
        _ = request_future => (),
    }
}
