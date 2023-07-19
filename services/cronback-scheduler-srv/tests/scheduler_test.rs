use std::sync::Arc;

use cronback_scheduler_srv::test_helpers;
use dto::traits::ProstOptionExt;
use lib::clients::scheduler_client::ScopedSchedulerClient;
use lib::config::{ConfigLoader, Role};
use lib::grpc_client_provider::test_helpers::TestGrpcClientProvider;
use lib::grpc_client_provider::GrpcClientFactory;
use lib::prelude::*;
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
use proto::scheduler_proto::{
    DeleteProjectTriggersRequest,
    GetTriggerRequest,
    GetTriggerResponse,
    ListTriggersRequest,
    ListTriggersResponse,
    UpsertTriggerRequest,
    UpsertTriggerResponse,
};
use proto::trigger_proto::{
    schedule,
    Recurring,
    Schedule,
    Trigger,
    TriggerStatus,
};
use tonic::Request;
use tracing::info;
use tracing_test::traced_test;

fn make_trigger(
    name: impl Into<String>,
    scheduled: bool,
    description: impl Into<String>,
) -> Trigger {
    let schedule = if scheduled {
        Some(Schedule {
            schedule: Some(schedule::Schedule::Recurring(Recurring {
                cron: "0 * * * * *".to_owned(),
                timezone: "Europe/London".into(),
                limit: Some(4),
                ..Default::default()
            })),
        })
    } else {
        None
    };

    Trigger {
        payload: Some(Payload {
            body: "Hello World".into(),
            ..Default::default()
        }),
        name: name.into(),
        description: Some(description.into()),
        action: Some(Action {
            action: Some(action::Action::Webhook(Webhook {
                url: "http://localhost:3000".to_owned(),
                http_method: HttpMethod::Get.into(),
                timeout_s: 30.0,
                retry: None,
            })),
        }),
        status: Default::default(),
        schedule,
        ..Default::default()
    }
}

async fn install_trigger(
    client_provider: &TestGrpcClientProvider<ScopedSchedulerClient>,
    project: &ValidShardedId<ProjectId>,
    trigger: Trigger,
) -> UpsertTriggerResponse {
    let upsert = UpsertTriggerRequest {
        precondition: None,
        trigger_name: None,
        trigger: Some(trigger),
    };
    client_provider
        .get_client(&RequestId::new(), project)
        .await
        .unwrap()
        .upsert_trigger(Request::new(upsert))
        .await
        .unwrap()
        .into_inner()
}

async fn list_triggers(
    client_provider: &TestGrpcClientProvider<ScopedSchedulerClient>,
    project: &ValidShardedId<ProjectId>,
) -> ListTriggersResponse {
    let list = ListTriggersRequest::default();
    client_provider
        .get_client(&RequestId::new(), project)
        .await
        .unwrap()
        .list_triggers(Request::new(list))
        .await
        .unwrap()
        .into_inner()
}

async fn get_trigger(
    client_provider: &TestGrpcClientProvider<ScopedSchedulerClient>,
    project: &ValidShardedId<ProjectId>,
    trigger_name: &str,
) -> Option<GetTriggerResponse> {
    let get = GetTriggerRequest {
        name: trigger_name.to_owned(),
    };
    client_provider
        .get_client(&RequestId::new(), project)
        .await
        .unwrap()
        .get_trigger(Request::new(get))
        .await
        .ok()
        .map(|r| r.into_inner())
}

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

#[traced_test]
#[tokio::test]
async fn delete_project_triggers_test() {
    let shutdown = Shutdown::default();
    let config_loader = Arc::new(ConfigLoader::from_path(&None));
    let context = ServiceContext::new(
        format!("{:?}", Role::Scheduler),
        config_loader,
        shutdown,
    );

    let project1 = ProjectId::generate();
    let project2 = ProjectId::generate();

    info!("Initialising test server...");
    let (_serve_future, client_provider) =
        test_helpers::test_server_and_client(context).await;

    let mut project1_triggers = vec![];
    let mut project2_triggers = vec![];

    // ** Project 1**
    // 3 scheduled triggers
    for i in 0..3 {
        let trigger_name = format!("project-trigger-{}", i);
        let install_resp = install_trigger(
            &client_provider,
            &project1,
            make_trigger(
                trigger_name,
                /* scheduled = */ true,
                /* description = */ "project1",
            ),
        )
        .await;
        assert_eq!(UpsertEffect::Created, install_resp.effect());
        project1_triggers.push(install_resp.trigger.unwrap().name);
    }
    // 2 on-demand triggers
    for i in 0..2 {
        let trigger_name = format!("project-on-demand-trigger-{}", i);
        let install_resp = install_trigger(
            &client_provider,
            &project1,
            make_trigger(
                trigger_name,
                /* scheduled = */ false,
                /* description = */ "project1",
            ),
        )
        .await;
        assert_eq!(UpsertEffect::Created, install_resp.effect());
        project1_triggers.push(install_resp.trigger.unwrap().name);
    }

    // validate that all triggers are installed
    let list_resp = list_triggers(&client_provider, &project1).await;
    assert_eq!(5, list_resp.triggers.len());

    // ** Project 2**
    // Adding 3 scheduled triggers to project2 to validate they are not deleted.
    for i in 0..3 {
        let trigger_name = format!("project-trigger-{}", i);
        let install_resp = install_trigger(
            &client_provider,
            &project2,
            make_trigger(
                &trigger_name,
                /* scheduled = */ true,
                /* description = */ "project2",
            ),
        )
        .await;
        assert_eq!(UpsertEffect::Created, install_resp.effect());
        project2_triggers.push(install_resp.trigger.unwrap().name);
    }

    // Project1 triggers exist and correct
    for name in &project1_triggers {
        info!(name = name, "Getting trigger");
        let get_resp = get_trigger(&client_provider, &project1, &name).await;
        assert!(get_resp.is_some());
        let get_resp = get_resp.unwrap();
        assert_eq!("project1", get_resp.trigger.unwrap().description());
    }

    // Project2 triggers exist and correct
    // BUG: https://github.com/devtari-io/cronback/issues/6
    // for name in &project2_triggers {
    //     info!("Getting trigger {}", name);
    //     let get_resp = get_trigger(&client_provider, &project2, &name).await;
    //     assert!(get_resp.is_some());
    //     let get_resp = get_resp.unwrap();
    //     assert_eq!("project2", get_resp.trigger.unwrap().description());
    // }

    // Now, delete all triggers for this project
    let list = DeleteProjectTriggersRequest::default();
    client_provider
        .get_client(&RequestId::new(), &project1)
        .await
        .unwrap()
        .delete_project_triggers(Request::new(list))
        .await
        .unwrap()
        .into_inner();

    // Project 1 triggers should be deleted
    let list_resp = list_triggers(&client_provider, &project1).await;
    assert_eq!(0, list_resp.triggers.len());

    // even getting individual scheduled triggers should return nothing.
    for name in &project1_triggers {
        info!(name = name, "Getting trigger");
        let get_resp = get_trigger(&client_provider, &project1, &name).await;
        assert_eq!(None, get_resp);
    }

    // Project2 are intact
    let list_resp = list_triggers(&client_provider, &project2).await;
    assert_eq!(3, list_resp.triggers.len());
    for name in &project2_triggers {
        info!(name = name, "Getting trigger");
        let get_resp = get_trigger(&client_provider, &project2, &name).await;
        assert!(get_resp.is_some());
        let get_resp = get_resp.unwrap();
        assert_eq!("project2", get_resp.trigger.unwrap().description());
    }
}
