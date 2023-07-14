use std::collections::HashMap;
use std::sync::Arc;

use lib::config::{ConfigLoader, Role};
use lib::grpc_client_provider::GrpcClientFactory;
use lib::service::ServiceContext;
use lib::shutdown::Shutdown;
use lib::types::*;
use proto::scheduler_proto::{GetTriggerRequest, InstallTriggerRequest};
use proto::trigger_proto::{self, Schedule, TriggerStatus};
use proto::webhook_proto;
use scheduler::test_helpers;
use tonic::Request;
use tracing::info;
use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn install_trigger_valid_test() {
    let shutdown = Shutdown::default();
    let config_loader = Arc::new(ConfigLoader::from_path(&None));
    let context = ServiceContext::new(
        format!("{:?}", Role::Scheduler),
        config_loader,
        shutdown,
    );
    let project = ProjectId::generate();
    info!("Initialising test...");
    let (_serve_future, client_provider) =
        test_helpers::test_server_and_client(context).await;
    let install_trigger = InstallTriggerRequest {
        id: None,
        fail_if_exists: false,
        reference: None,
        name: "sample-trigger".to_owned(),
        description: None,
        action: Some(trigger_proto::Action {
            action: Some(trigger_proto::action::Action::Webhook(
                webhook_proto::Webhook {
                    url: "http://google.com".to_owned(),
                    http_method: webhook_proto::HttpMethod::Get.into(),
                    timeout_s: 30.0,
                    retry: None,
                },
            )),
        }),
        payload: Some(proto::trigger_proto::Payload {
            content_type: "application/json".to_owned(),
            headers: HashMap::new(),
            body: "Hello World".into(),
        }),
        schedule: Some(Schedule {
            schedule: Some(trigger_proto::schedule::Schedule::Recurring(
                trigger_proto::Recurring {
                    cron: "0 * * * * *".to_owned(),
                    timezone: "Europe/London".into(),
                    limit: Some(4),
                    remaining: None,
                },
            )),
        }),
    };
    let installed_trigger = client_provider
        .get_client(&RequestId::new(), &project)
        .await
        .unwrap()
        .install_trigger(Request::new(install_trigger))
        .await
        .unwrap()
        .into_inner();
    assert!(installed_trigger.trigger.is_some());
    assert!(!installed_trigger.already_existed);
    let created_trigger = installed_trigger.trigger.unwrap();
    assert!(created_trigger.id.clone().unwrap().value.len() > 5);
    let created_trigger: Trigger = created_trigger.into();
    // Validate that the cron pattern is what we have set.
    // No errors. Let's try and get it from server.
    let response = client_provider
        .get_client(&RequestId::new(), &project)
        .await
        .unwrap()
        .get_trigger(Request::new(GetTriggerRequest {
            id: Some(created_trigger.id.clone().into()),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(response.trigger.is_some());
    let trigger_retrieved: Trigger = response.trigger.unwrap().into();
    assert_eq!(trigger_retrieved.id, created_trigger.id);
}

#[traced_test]
#[tokio::test]
async fn install_trigger_reference_test() {
    let shutdown = Shutdown::default();
    let config_loader = Arc::new(ConfigLoader::from_path(&None));
    let context = ServiceContext::new(
        format!("{:?}", Role::Scheduler),
        config_loader,
        shutdown,
    );
    let project = ProjectId::generate();
    let (_serve_future, client_provider) =
        test_helpers::test_server_and_client(context).await;
    let install_trigger = InstallTriggerRequest {
        id: None,
        fail_if_exists: false,
        reference: Some("some-meaningful-reference".to_owned()),
        name: "sample-trigger".to_owned(),
        description: None,
        action: Some(trigger_proto::Action {
            action: Some(trigger_proto::action::Action::Webhook(
                webhook_proto::Webhook {
                    url: "http://google.com".to_owned(),
                    http_method: webhook_proto::HttpMethod::Get.into(),
                    timeout_s: 30.0,
                    retry: None,
                },
            )),
        }),
        payload: Some(proto::trigger_proto::Payload {
            content_type: "application/json".to_owned(),
            headers: HashMap::new(),
            body: "Hello World".into(),
        }),
        schedule: Some(Schedule {
            schedule: Some(trigger_proto::schedule::Schedule::Recurring(
                trigger_proto::Recurring {
                    cron: "0 * * * * *".to_owned(),
                    timezone: "Europe/London".into(),
                    limit: Some(4),
                    remaining: None,
                },
            )),
        }),
    };
    let installed_trigger = client_provider
        .get_client(&RequestId::new(), &project)
        .await
        .unwrap()
        .install_trigger(Request::new(install_trigger))
        .await
        .unwrap()
        .into_inner();
    assert!(installed_trigger.trigger.is_some());
    // new trigger.
    assert!(!installed_trigger.already_existed);
    // updated at is NOT set.
    assert_eq!(None, installed_trigger.trigger.unwrap().updated_at);
    // let's update the description.
    let install_trigger = InstallTriggerRequest {
        // We rely on the reference to match the trigger.
        id: None,
        fail_if_exists: false,
        // UPDATED
        reference: Some("some-meaningful-reference".to_owned()),
        name: "sample-trigger".to_owned(),
        description: Some("new description is here".to_owned()),
        action: Some(trigger_proto::Action {
            action: Some(trigger_proto::action::Action::Webhook(
                webhook_proto::Webhook {
                    url: "http://google.com".to_owned(),
                    http_method: webhook_proto::HttpMethod::Get.into(),
                    timeout_s: 30.0,
                    retry: None,
                },
            )),
        }),
        payload: Some(proto::trigger_proto::Payload {
            content_type: "application/json".to_owned(),
            headers: HashMap::new(),
            body: "Hello World".into(),
        }),
        schedule: Some(Schedule {
            schedule: Some(trigger_proto::schedule::Schedule::Recurring(
                trigger_proto::Recurring {
                    cron: "0 * * * * *".to_owned(),
                    timezone: "Europe/London".into(),
                    limit: Some(4),
                    remaining: None,
                },
            )),
        }),
    };

    let installed_trigger = client_provider
        .get_client(&RequestId::new(), &project)
        .await
        .unwrap()
        .install_trigger(Request::new(install_trigger))
        .await
        .unwrap()
        .into_inner();
    assert!(installed_trigger.trigger.is_some());
    // updated trigger.
    assert!(installed_trigger.already_existed);
    let updated_trigger = installed_trigger.trigger.unwrap();
    assert_eq!(
        Some("new description is here".to_owned()),
        updated_trigger.description
    );
    assert_eq!(TriggerStatus::Scheduled, updated_trigger.status());

    // let's switch this to on-demand
    let install_trigger = InstallTriggerRequest {
        id: updated_trigger.id,
        fail_if_exists: false,
        // Unset the reference.
        reference: None,
        name: "sample-trigger".to_owned(),
        description: None,
        action: Some(trigger_proto::Action {
            action: Some(trigger_proto::action::Action::Webhook(
                webhook_proto::Webhook {
                    url: "http://google.com".to_owned(),
                    http_method: webhook_proto::HttpMethod::Get.into(),
                    timeout_s: 30.0,
                    retry: None,
                },
            )),
        }),
        payload: Some(proto::trigger_proto::Payload {
            content_type: "application/json".to_owned(),
            headers: HashMap::new(),
            body: "Hello World".into(),
        }),
        // Unset the schedule
        schedule: None,
    };

    let installed_trigger = client_provider
        .get_client(&RequestId::new(), &project)
        .await
        .unwrap()
        .install_trigger(Request::new(install_trigger))
        .await
        .unwrap()
        .into_inner();
    assert!(installed_trigger.trigger.is_some());
    // updated trigger.
    assert!(installed_trigger.already_existed);
    let updated_trigger = installed_trigger.trigger.unwrap();
    assert_eq!(None, updated_trigger.description);
    assert_eq!(None, updated_trigger.reference);
    // updated at is set.
    assert_ne!(None, updated_trigger.updated_at);
    assert_eq!(TriggerStatus::OnDemand, updated_trigger.status());
}
