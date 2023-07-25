///  e!(context = ctx, TriggerRunCreated { meta: run.meta().into() });
#[macro_export]
macro_rules! e {
    (context = $context:expr, $kind:ident { $($body:tt)* } ) => {
     e!(project_id = $context.project_id, $kind { $($body)* });
    };
    (project_id = $project_id:expr, $kind:ident { $($body:tt)* } ) => {
        {
            let _e = ::proto::events::Event::from_project(
                $project_id.clone(),
                ::proto::events::Events::$kind(
                ::proto::events::$kind { $(
                        $body
                        )* }
            ));
            $crate::events::log_event(_e);
        }
    };
    ($kind:ident { $($body:tt)* } ) => {
        {
            let _e = ::proto::events::Event::new(
                ::proto::events::Events::$kind(
                ::proto::events::$kind { $(
                        $body
                        )* }
            ));
            $crate::events::log_event(_e);
        }
    };
}

/// Emits the event to the current events subscriber
pub fn log_event(event: proto::events::Event) {
    // serialize the event to JSON and log it to target `events`
    let event = serde_json::to_string(&event).unwrap();
    tracing::info!(target: "events", "{}", event);
}

use std::fmt;

use tracing_core::Subscriber;
use tracing_subscriber::fmt::format::{self, FormatEvent, FormatFields};
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::registry::LookupSpan;
pub struct Formatter;

impl<S, N> FormatEvent<S, N> for Formatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &tracing_core::Event<'_>,
    ) -> fmt::Result {
        // Write fields directly to the writer with no metadata
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

pub use e;
