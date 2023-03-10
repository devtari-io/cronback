use std::sync::Arc;

use proto::trigger_proto::{self, Cron, Schedule, Trigger, TriggerStatus};
use tonic::{Request, Status};

use proto::scheduler_proto::{
    GetTriggerRequest, InstallTrigger, InstallTriggerRequest,
};
use scheduler::test_helpers;
use shared::config::{ConfigLoader, Role};
use shared::service::ServiceContext;
use shared::shutdown::Shutdown;

#[tokio::test]
async fn install_trigger_invalid_test() {
    let shutdown = Shutdown::default();
    let config_loader = Arc::new(ConfigLoader::from_path(&None));
    let context = ServiceContext::new(
        format!("{:?}", Role::Scheduler),
        config_loader,
        shutdown,
    );
    let (serve_future, mut client) =
        test_helpers::test_server_and_client(context).await;

    let request_future = async {
        let response = client
            .install_trigger(Request::new(InstallTriggerRequest {
                install_trigger: None,
            }))
            .await;
        assert!(matches!(response, Err(Status { .. })));
    };

    // Wait for completion, when the client request future completes
    tokio::select! {
        _ = serve_future => panic!("server returned first"),
        _ = request_future => (),
    }
}

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
    let install_trigger = Some(InstallTrigger {
        cell_id: 0,
        owner_id: "asoli".to_owned(),
        reference_id: None,
        name: None,
        description: None,
        emit: None,
        payload: None,
        schedule: Some(Schedule {
            schedule: Some(trigger_proto::schedule::Schedule::Cron(Cron {
                cron: format!("0 * * * * *"),
                timezone: "Europe/London".into(),
                events_limit: 4,
            })),
        }),
    });
    let request_future = async {
        client
            .install_trigger(Request::new(InstallTriggerRequest {
                install_trigger: install_trigger.clone(),
            }))
            .await
            .unwrap()
            .into_inner();
        // No errors. Let's try and get it from server.
        let response = client
            .get_trigger(Request::new(GetTriggerRequest {
                id: "trig_12345".to_owned(),
            }))
            .await
            .unwrap()
            .into_inner();
        todo!();
        //assert_eq!(response.trigger, trigger);
    };

    // Wait for completion, when the client request future completes
    tokio::select! {
        _ = serve_future => panic!("server returned first"),
        _ = request_future => (),
    }
}
