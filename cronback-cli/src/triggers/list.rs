use std::fmt::Write as FmtWrite;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use cling::prelude::*;
use colored::Colorize;
use cronback_client::{
    Pagination,
    Recurring,
    RunAt,
    Schedule,
    TriggerStatus,
    TriggersFilter,
};
use prettytable::{row, Table};

use crate::args::CommonOptions;
use crate::ui::FancyToString;

#[derive(CliRunnable, CliParam, Clone, Debug, Parser)]
#[cling(run = "list")]
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

    #[clap(long, short)]
    /// List all triggers, including `expired` and `cancelled` triggers.
    all: bool,
}

async fn list(common_options: &CommonOptions, opts: &List) -> Result<()> {
    let client = common_options.new_client()?;
    let status: Vec<TriggerStatus> = match opts.status {
        | Some(ref statuses) => statuses.to_vec(),

        | None => {
            vec![
                TriggerStatus::Scheduled,
                TriggerStatus::OnDemand,
                TriggerStatus::Paused,
            ]
        }
    };

    let filter = if opts.all {
        None
    } else {
        Some(TriggersFilter { status })
    };

    let pagination = Some(Pagination {
        cursor: opts.cursor.clone(),
        limit: opts.limit,
    });

    let response =
        cronback_client::triggers::list(&client, pagination, filter).await?;

    let response = response.into_inner()?;
    // Print Table
    if !response.data.is_empty() {
        let len = response.data.len();

        let mut table = Table::new();
        table.set_titles(row![
            "Name",
            "Status",
            "Schedule",
            "Runs",
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
                trigger.status.fancy(),
                trigger.schedule.map(fancy_schedule).unwrap_or_default(),
                fancy_runs(trigger.last_ran_at, trigger.estimated_future_runs),
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

        println!("{}", table);

        // Print Pagination Metadata
        eprintln!("{len} Triggers Shown");
        if let Some(next_page_cursor) = response.meta.next_cursor {
            eprintln!(
                "View next page by {}{}",
                "--cursor=".bold(),
                next_page_cursor.bold()
            );
        }
    }

    Ok(())
}

fn fancy_runs(
    last_ran_at: Option<DateTime<Utc>>,
    estimated_future_runs: Vec<DateTime<Utc>>,
) -> String {
    let mut runs = String::new();

    let last_ran_at = last_ran_at.map(|x| x.to_rfc2822());
    if let Some(last_ran_at) = last_ran_at {
        writeln!(&mut runs, "Last run was at:").unwrap();
        writeln!(&mut runs, " - {}", last_ran_at).unwrap();
    };
    let future_runs: Vec<_> = estimated_future_runs
        .into_iter()
        .map(|x| x.to_rfc2822())
        .collect();

    if !future_runs.is_empty() {
        writeln!(&mut runs, "Next runs:").unwrap();
        for run in future_runs {
            writeln!(&mut runs, " - {}", run).unwrap();
        }
    };

    runs
}

fn fancy_schedule(schedule: Schedule) -> String {
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
