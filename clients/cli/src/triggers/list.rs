use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use colored::Colorize;
use cronback::{
    Pagination,
    Recurring,
    RunAt,
    Schedule,
    TriggerStatus,
    TriggersFilter,
};
use prettytable::{row, Table};

use crate::args::CommonOptions;
use crate::{emitln, RunCommand};

#[derive(Clone, Debug, Parser)]
pub struct List {
    /// Cursor to start listing from
    #[clap(long)]
    cursor: Option<String>,
    /// Limit the number of results
    #[clap(long, default_value = "20")]
    limit: Option<i32>,
    /// Filter by trigger status, by default we return `scheduled`, `paused`,
    /// and `on_demand` triggers only.
    #[clap(long)]
    status: Option<Vec<TriggerStatus>>,

    #[clap(long)]
    /// List all triggers, including `expired` and `cancelled` triggers.
    all: bool,
}

#[async_trait]
impl RunCommand for List {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        let client = common_options.new_client()?;
        let status: Vec<TriggerStatus> = match self.status {
            | Some(ref statuses) => {
                statuses.iter().map(|s| (*s).into()).collect()
            }

            | None => {
                vec![
                    TriggerStatus::Scheduled,
                    TriggerStatus::OnDemand,
                    TriggerStatus::Paused,
                ]
            }
        };

        let filter = if self.all {
            None
        } else {
            Some(TriggersFilter { status })
        };

        let pagination = Some(Pagination {
            cursor: self.cursor.clone(),
            limit: self.limit,
        });

        let response = client.list_triggers(pagination, filter).await?;
        common_options.show_meta(&response, out, err).await?;

        let response = response.into_inner()?;
        // Print Table
        if !response.data.is_empty() {
            let len = response.data.len();

            let mut table = Table::new();
            table.set_titles(row![
                "Name",
                "Status",
                "Schedule",
                "Latest Run At",
                "End Point",
                "Payload Size",
                "Created At",
            ]);
            for trigger in response.data {
                let endpoint = trigger
                    .webhook()
                    .and_then(|e| e.url.clone())
                    .unwrap_or_default();

                table.add_row(row![
                    trigger.name.expect("name should be present"),
                    fancy_status(
                        trigger.status.expect("status should be present")
                    ),
                    trigger.schedule.map(fancy_schedule).unwrap_or_default(),
                    trigger
                        .last_ran_at
                        .map(|x| x.to_rfc2822())
                        .unwrap_or_default(),
                    endpoint,
                    trigger
                        .payload
                        .map(|x| x.body)
                        .map(|y| format!("{} bytes", y.as_bytes().len()))
                        .unwrap_or_default(),
                    trigger
                        .created_at
                        .expect("created_at should be present")
                        .to_rfc2822(),
                ]);
            }

            emitln!(out, "{}", table);

            // Print Pagination Metadata
            emitln!(err, "{len} Triggers Shown");
            if let Some(next_page_cursor) = response.meta.next_cursor {
                emitln!(
                    err,
                    "View next page by {}{}",
                    "--cursor=".bold(),
                    next_page_cursor.bold()
                );
            }
        }

        Ok(())
    }
}

fn fancy_status(status: TriggerStatus) -> String {
    match status {
        | TriggerStatus::Scheduled => {
            format!("⏰ {}", status.to_string().green())
        }
        | TriggerStatus::OnDemand => format!("📍 {status}"),
        | TriggerStatus::Expired => {
            format!("〰 {}", status.to_string().italic())
        }
        | TriggerStatus::Cancelled => format!("✖️ {status}"),
        | TriggerStatus::Paused => {
            format!("🔸 {}", status.to_string().blink())
        }
        | s => s.to_string(),
    }
}

fn fancy_schedule(schedule: Schedule) -> String {
    use std::fmt::Write;

    fn fancy_recurring(r: Recurring) -> String {
        let mut buf = String::new();
        writeln!(buf, "{}", r.cron.unwrap_or_default()).unwrap();
        if let Some(tz) = r.timezone {
            writeln!(buf, "{}", tz).unwrap();
        }
        if let Some(limit) = r.limit {
            write!(buf, "{} limit", limit).unwrap();
            if let Some(remaining) = r.remaining {
                write!(buf, " ({} remaining)", remaining).unwrap();
            }
            writeln!(buf).unwrap();
        }

        buf
    }

    fn fancy_run_at(r: RunAt) -> String {
        let mut buf = String::new();
        let len = r.timepoints.len();

        let mut done_timepoints = r.timepoints.clone();
        done_timepoints.sort();

        let remaining_timepoints;
        if let Some(remaining) = r.remaining {
            writeln!(buf, "({} remaining)", remaining).unwrap();
            remaining_timepoints =
                done_timepoints.split_off(len - remaining as usize);
        } else {
            remaining_timepoints = done_timepoints.clone();
            done_timepoints.clear();
        }

        for t in done_timepoints {
            writeln!(buf, " - {}", t.to_rfc2822().strikethrough()).unwrap();
        }
        for t in remaining_timepoints {
            writeln!(buf, " - {}", t.to_rfc2822()).unwrap();
        }

        buf
    }

    match schedule {
        | Schedule::Recurring(s) => fancy_recurring(s),
        | Schedule::RunAt(s) => fancy_run_at(s),
        | _ => "Unknown Schedule Type!".to_string(),
    }
}
