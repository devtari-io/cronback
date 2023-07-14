use std::sync::Arc;

use dto::traits::ProstOptionExt;
use lib::config::{ConfigLoader, Role};
use lib::grpc_client_provider::GrpcClientFactory;
use lib::service::ServiceContext;
use lib::shutdown::Shutdown;
use lib::types::{ProjectId, RequestId};
use proto::common::{
    action,
    Action,
    HttpMethod,
    Payload,
    UpsertEffect,
    Webhook,
};
use proto::scheduler_proto::{GetTriggerRequest, UpsertTriggerRequest};
use proto::trigger_proto::{
    schedule,
    Recurring,
    Schedule,
    Trigger,
    TriggerStatus,
};
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

    let install_trigger = UpsertTriggerRequest {
        precondition: None,
        trigger_name: None,
        trigger: Some(Trigger {
            payload: Some(Payload {
                body: "Hello World".into(),
                ..Default::default()
            }),
            name: "sample-trigger".to_owned(),
            action: Some(Action {
                action: Some(action::Action::Webhook(Webhook {
                    url: "http://google.com".to_owned(),
                    http_method: HttpMethod::Get.into(),
                    timeout_s: 30.0,
                    retry: None,
                })),
            }),
            status: Default::default(),
            schedule: Some(Schedule {
                schedule: Some(schedule::Schedule::Recurring(Recurring {
                    cron: "0 * * * * *".to_owned(),
                    timezone: "Europe/London".into(),
                    limit: Some(4),
                    ..Default::default()
                })),
            }),
            ..Default::default()
        }),
    };
    let installed_trigger = client_provider
        .get_client(&RequestId::new(), &project)
        .await
        .unwrap()
        .upsert_trigger(Request::new(install_trigger))
        .await
        .unwrap()
        .into_inner();
    assert!(installed_trigger.trigger.is_some());
    assert_eq!(UpsertEffect::Created, installed_trigger.effect());
    assert_eq!(
        "sample-trigger",
        installed_trigger.trigger.get_or_default().name
    );
    // Validate that the cron pattern is what we have set.
    // No errors. Let's try and get it from server.
    let response = client_provider
        .get_client(&RequestId::new(), &project)
        .await
        .unwrap()
        .get_trigger(Request::new(GetTriggerRequest {
            name: "sample-trigger".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(response.trigger.is_some());
    assert_eq!(response.trigger, installed_trigger.trigger);
}

#[traced_test]
#[tokio::test]
// TODO: Remove when update is fixed
#[ignore]
async fn install_trigger_uniqueness_test() {
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
    let install_trigger = UpsertTriggerRequest {
        // No precondition
        precondition: None,
        trigger_name: None,
        trigger: Some(Trigger {
            payload: Some(Payload {
                body: "Hello World".into(),
                ..Default::default()
            }),
            name: "sample-trigger-2".to_owned(),
            action: Some(Action {
                action: Some(action::Action::Webhook(Webhook {
                    url: "http://google.com".to_owned(),
                    http_method: HttpMethod::Get.into(),
                    timeout_s: 30.0,
                    retry: None,
                })),
            }),
            status: Default::default(),
            schedule: Some(Schedule {
                schedule: Some(schedule::Schedule::Recurring(Recurring {
                    cron: "0 * * * * *".to_owned(),
                    timezone: "Europe/London".into(),
                    limit: Some(4),
                    ..Default::default()
                })),
            }),

            ..Default::default()
        }),
    };
    let installed_trigger = client_provider
        .get_client(&RequestId::new(), &project)
        .await
        .unwrap()
        .upsert_trigger(Request::new(install_trigger))
        .await
        .unwrap()
        .into_inner();
    assert!(installed_trigger.trigger.is_some());
    // new trigger.
    assert_eq!(UpsertEffect::Created, installed_trigger.effect());
    // updated at is NOT set.
    assert!(installed_trigger
        .trigger
        .get_or_default()
        .updated_at
        .is_none());
    // let's update the description.

    let install_trigger = UpsertTriggerRequest {
        // We rely on the name to match the trigger.
        precondition: None,
        trigger_name: Some("sample-trigger-2".to_string()),
        trigger: Some(Trigger {
            payload: Some(Payload {
                body: "Hello World".into(),
                ..Default::default()
            }),
            name: "sample-trigger-2".to_owned(),
            description: Some("new description is here".to_owned()),
            action: Some(Action {
                action: Some(action::Action::Webhook(Webhook {
                    url: "http://google.com".to_owned(),
                    http_method: HttpMethod::Get.into(),
                    timeout_s: 30.0,
                    retry: None,
                })),
            }),
            status: Default::default(),
            schedule: Some(Schedule {
                schedule: Some(schedule::Schedule::Recurring(Recurring {
                    cron: "0 * * * * *".to_owned(),
                    timezone: "Europe/London".into(),
                    limit: Some(4),
                    ..Default::default()
                })),
            }),

            ..Default::default()
        }),
    };

    let installed_trigger = client_provider
        .get_client(&RequestId::new(), &project)
        .await
        .unwrap()
        .upsert_trigger(Request::new(install_trigger))
        .await
        .unwrap()
        .into_inner();
    assert!(installed_trigger.trigger.is_some());
    // updated trigger.
    assert_eq!(UpsertEffect::Modified, installed_trigger.effect());
    let updated_trigger = installed_trigger.trigger.get_or_default();
    assert_eq!(
        Some("new description is here".to_owned()),
        updated_trigger.description
    );
    assert_eq!(TriggerStatus::Scheduled, updated_trigger.status());

    // let's switch this to on-demand
    let install_trigger = UpsertTriggerRequest {
        // We rely on the name to match the trigger.
        precondition: None,
        trigger_name: Some("sample-trigger-2".to_string()),
        trigger: Some(Trigger {
            payload: Some(Payload {
                body: "Hello World".into(),
                ..Default::default()
            }),
            name: "sample-trigger-2".to_owned(),
            // Resetting the description to None
            description: None,
            action: Some(Action {
                action: Some(action::Action::Webhook(Webhook {
                    url: "http://google.com".to_owned(),
                    http_method: HttpMethod::Get.into(),
                    timeout_s: 30.0,
                    retry: None,
                })),
            }),
            status: Default::default(),
            schedule: None,
            ..Default::default()
        }),
    };

    let installed_trigger = client_provider
        .get_client(&RequestId::new(), &project)
        .await
        .unwrap()
        .upsert_trigger(Request::new(install_trigger))
        .await
        .unwrap()
        .into_inner();
    assert!(installed_trigger.trigger.is_some());
    // updated trigger.
    assert_eq!(UpsertEffect::Modified, installed_trigger.effect());
    let updated_trigger = installed_trigger.trigger.get_or_default();
    assert_eq!(None, updated_trigger.description);
    assert_eq!("sample-trigger-2", updated_trigger.name);
    // updated at is set.
    assert_ne!(None, updated_trigger.updated_at);
    assert_eq!(TriggerStatus::OnDemand, updated_trigger.status());
}
