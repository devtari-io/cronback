use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use lib::types::TriggerId;
use tracing::{info, trace};

use crate::db_model::schedule::ScheduleIter;
use crate::db_model::triggers::Status;
use crate::db_model::Trigger;
use crate::error::TriggerError;

///
/// Maintains the set of `active` triggers in memory. Expired triggers are
/// evicted to save space.
///
/// dirty_triggers holds the set of triggers that has been updated since the
/// last time the spinner has looked at it. The spinner resets the set after
/// reloading.
#[derive(Default)]
pub(crate) struct ActiveTriggerMap {
    state: HashMap<TriggerId, ActiveTrigger>,
    /// The set of trigger Ids that has been updated
    dirty: bool,
    awaiting_db_flush: HashSet<TriggerId>,
}

impl ActiveTriggerMap {
    /// Inserts or updates a trigger if exists
    pub fn add_or_update(
        &mut self,
        trigger: Trigger,
        fast_forward: bool,
    ) -> Result<Trigger, TriggerError> {
        // TODO: We should instead convert active_trigger back to Trigger to
        // get the updated status
        let cloned_trigger = trigger.clone();
        let trigger_id = trigger.id.clone();
        let active_trigger = ActiveTrigger::try_from(trigger, fast_forward)?;
        self.state.insert(trigger_id, active_trigger);
        self.mark_dirty();
        Ok(cloned_trigger)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn awaiting_db_flush(&self) -> HashSet<TriggerId> {
        self.awaiting_db_flush.clone()
    }

    pub fn clear_db_flush(&mut self) {
        self.awaiting_db_flush.clear();
    }

    pub fn add_to_awaiting_db_flush(&mut self, trigger_id: TriggerId) {
        // Mark this trigger as dirty so that we can persist it
        self.awaiting_db_flush.insert(trigger_id);
    }

    pub fn is_trigger_retired(&self, trigger_id: &TriggerId) -> bool {
        self.state
            .get(trigger_id)
            .map(|t| {
                [Status::Cancelled, Status::Expired].contains(&t.get().status)
            })
            .unwrap_or(false)
    }

    // Can be used to remove an expired/cancelled trigger after flush
    pub fn pop_trigger(
        &mut self,
        trigger_id: &TriggerId,
    ) -> Option<ActiveTrigger> {
        let res = self.state.remove(trigger_id);
        if res.is_some() {
            // not strictly necessary, but without it the spinner will report
            // the wrong number of active triggers to metrics.
            self.mark_dirty();
        }
        res
    }

    // Should be used to only push dead/retired triggers.
    pub fn push_trigger(
        &mut self,
        trigger_id: TriggerId,
        trigger: ActiveTrigger,
    ) {
        self.state.insert(trigger_id, trigger);
        // not strictly necessary, but without it the spinner will report
        // the wrong number of active triggers to metrics.
        self.mark_dirty();
    }

    pub fn clear(&mut self) {
        self.state.clear();
        self.reset_dirty();
    }

    /// Scans the set of active triggers and creates temporal states without
    /// advancing any triggers.
    pub fn build_temporal_state(
        &mut self,
    ) -> BinaryHeap<Reverse<TriggerTemporalState>> {
        let mut new_state = BinaryHeap::new();
        let mut expired_triggers = Vec::new();
        trace!("Building temporal state for {} triggers", self.state.len());
        for trigger in self.state.values_mut() {
            if let Some(tick) = trigger.peek() {
                let state = TriggerTemporalState {
                    trigger_id: trigger.get().id.clone(),
                    next_tick: tick,
                };
                info!(
                    "Trigger '{}' next trigger point is {}",
                    state.trigger_id, tick
                );
                new_state.push(Reverse(state));
            } else {
                trace!(
                    "Trigger '{}' has no timepoints in the future, marking as \
                     expired until it gets retired",
                    trigger.get().id,
                );
                expired_triggers.push(trigger.get().id.clone());
            }
        }
        for trigger in expired_triggers {
            self.update_status(&trigger, Status::Expired, &[]).unwrap();
        }
        self.reset_dirty();
        new_state
    }

    pub fn get(&self, id: &TriggerId) -> Option<&Trigger> {
        self.state.get(id).map(|t| t.get())
    }

    /*
     * Advance ensures that the next tick is not the same as current tick.
     * I hear you asking, why do we need that?
     *
     * We need this because:
     * - We don't want to advance everything when we build a new temporal
     *   states, as this will incorrectly advance triggers that are still
     *   due.
     * - When temporal state is created, we only peek(), this makes the first
     *   iteration a bit awkward, when we advance after executing the
     *   trigger, advance() will return the same time point because we have
     *   never next()ed it. This _hack_ ensures that we will fast-forward in
     *   this rare case.
     * - This also ensures that, for any reason, we skip duplicates in the
     *   run_at list if we didn't catch this in validation.
     */
    pub fn advance(&mut self, trigger_id: &TriggerId) -> Option<DateTime<Tz>> {
        let Some(trigger) = self.state.get_mut(trigger_id) else {
            return None;
        };

        if !trigger.alive() {
            return None;
        }

        let next_tick = trigger.advance();

        if next_tick.is_some() {
            return next_tick;
        }

        // We should not be active anymore.
        self.update_status(trigger_id, Status::Expired, &[])
            .unwrap();
        None
    }

    pub fn pause(
        &mut self,
        trigger_id: &TriggerId,
    ) -> Result<(), TriggerError> {
        self.update_status(
            trigger_id,
            Status::Paused,
            &[Status::Cancelled, Status::Expired],
        )
    }

    pub fn resume(
        &mut self,
        trigger_id: &TriggerId,
    ) -> Result<(), TriggerError> {
        self.update_status(
            trigger_id,
            Status::Scheduled,
            &[Status::Cancelled, Status::Expired],
        )
    }

    pub fn cancel(
        &mut self,
        trigger_id: &TriggerId,
    ) -> Result<(), TriggerError> {
        self.update_status(trigger_id, Status::Cancelled, &[])
    }

    fn update_status(
        &mut self,
        trigger_id: &TriggerId,
        new_status: Status,
        reject_statuses: &[Status],
    ) -> Result<(), TriggerError> {
        let Some(trigger) = self.state.get_mut(trigger_id) else {
            return Err(TriggerError::NotFound(trigger_id.to_string()));
        };

        if reject_statuses.contains(&trigger.get().status) {
            return Err(TriggerError::InvalidStatus(
                new_status.as_operation(),
                trigger.get().status.clone(),
            ));
        }

        if trigger.update_status(new_status) {
            self.add_to_awaiting_db_flush(trigger_id.clone());
        }
        Ok(())
    }

    pub fn update_last_ran_at(
        &mut self,
        trigger_id: &TriggerId,
        ran_at: DateTime<Utc>,
    ) {
        let Some(trigger) = self.state.get_mut(trigger_id) else {
             return;
        };
        // Keep the last known run time (if set) unless we are seeing
        // a more recent one.
        let new_val = match trigger.last_ran_at() {
            // If we have never seen a run time, set it
            | None => Some(ran_at),
            // If we are seeing a more recent run time, update it
            | Some(last_known) if ran_at > last_known => Some(ran_at),
            // Use the last known run time
            | Some(last_known) => Some(last_known),
        };

        // We are guarding this because we should not add this trigger to
        // `awaiting_db_flush` if we didn't really update it.
        if trigger.inner.last_ran_at != new_val {
            trigger.inner.last_ran_at = new_val;
            self.add_to_awaiting_db_flush(trigger_id.clone());
        }
    }

    /// Removes a trigger from the active map. This also removes it from the
    /// awaiting_db_flush map.
    pub fn remove(&mut self, trigger_id: &TriggerId) {
        let trigger = self.state.remove(trigger_id);
        if trigger.is_some() {
            self.awaiting_db_flush.remove(trigger_id);
            self.mark_dirty();
        }
    }

    //// PRIVATE
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn reset_dirty(&mut self) {
        self.dirty = false;
    }
}

/// Metadata exclusively owned by the spinner, keeps the Id of the installed
/// trigger along with its next tick.
///
/// The spinner maintains a max-heap of those jobs to determine which Triggers
/// need to be evaluated at each loop.
#[derive(Debug)]
pub(crate) struct TriggerTemporalState {
    #[allow(unused)]
    pub trigger_id: TriggerId,
    // time of the next tick
    pub next_tick: DateTime<Tz>,
}

impl PartialOrd for TriggerTemporalState {
    fn partial_cmp(
        &self,
        other: &TriggerTemporalState,
    ) -> Option<std::cmp::Ordering> {
        self.next_tick.partial_cmp(&other.next_tick)
    }
}
impl Ord for TriggerTemporalState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.next_tick.cmp(&other.next_tick)
    }
}

impl PartialEq for TriggerTemporalState {
    fn eq(&self, other: &Self) -> bool {
        self.next_tick == other.next_tick
    }
}
impl Eq for TriggerTemporalState {}

// A wrapper around Trigger suitable for scheduler operations.
pub(crate) struct ActiveTrigger {
    inner: Trigger,
    ticks: ScheduleIter,
}

impl ActiveTrigger {
    fn try_from(
        trigger: Trigger,
        fast_forward: bool,
    ) -> Result<Self, TriggerError> {
        // Do we have a cron pattern or a set of time points?
        let k = trigger
            .schedule
            .as_ref()
            .ok_or_else(|| TriggerError::NotScheduled(trigger.id.clone()))?;
        // On fast forward, we ignore the last run time.
        let last_ran_at = if fast_forward {
            None
        } else {
            trigger.last_ran_at
        };
        let ticks = ScheduleIter::from_schedule(k, last_ran_at)?;
        // We assume that Trigger.schedule is never None
        Ok(Self {
            inner: trigger,
            ticks,
        })
    }

    pub fn get(&self) -> &Trigger {
        &self.inner
    }

    pub fn peek(&mut self) -> Option<DateTime<Tz>> {
        self.ticks.peek()
    }

    // Active means that it should continue to live in the spinner map. A paused
    // trigger is considered alive, but it won't run. We will advance
    // its clock as if it ran though.
    pub fn alive(&self) -> bool {
        self.inner.alive()
    }

    pub fn advance(&mut self) -> Option<DateTime<Tz>> {
        // Advances the iterator and peeks the following item
        let _ = self.ticks.next();
        let res = self.ticks.peek();
        let schedule = self.inner.schedule.as_mut().unwrap();
        schedule.set_remaining(self.ticks.remaining());
        res
    }

    pub fn last_ran_at(&self) -> Option<DateTime<Utc>> {
        self.inner.last_ran_at
    }

    // Returns true if state has changed.
    fn update_status(&mut self, new_status: Status) -> bool {
        if self.inner.status != new_status {
            self.inner.status = new_status;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lib::types::{Action, HttpMethod, ProjectId, TriggerId, Webhook};

    use super::*;
    use crate::db_model::schedule::{Recurring, Schedule};
    use crate::db_model::Trigger;

    fn create_cron_schedule(
        pattern: &str,
        cron_events_limit: Option<u64>,
    ) -> Schedule {
        Schedule::Recurring(Recurring {
            cron: pattern.to_string(),
            timezone: "Europe/London".into(),
            limit: cron_events_limit,
            remaining: None,
        })
    }

    fn create_trigger(sched: Schedule) -> Trigger {
        let project = ProjectId::generate();
        let id = TriggerId::generate(&project).into();
        Trigger {
            id,
            project_id: project,
            name: "sample-trigger".to_owned(),
            description: None,
            created_at: Utc::now(),
            updated_at: None,
            action: Action::Webhook(Webhook {
                url: "http://google.com".to_owned(),
                http_method: HttpMethod::Get,
                timeout_s: Duration::from_secs(30),
                retry: None,
            }),
            payload: None,
            status: Status::Scheduled,
            schedule: Some(sched),
            last_ran_at: None,
        }
    }

    #[test]
    fn temporal_state_advance() -> Result<(), TriggerError> {
        let cron = create_cron_schedule("* * * * * *", None);
        let trigger = create_trigger(cron);
        let trigger_id = trigger.id.clone();
        let mut map = ActiveTriggerMap::default();
        map.add_or_update(trigger, false)?;
        let mut temporal_states = map.build_temporal_state();
        assert_eq!(1, temporal_states.len());
        let tick1 = temporal_states.pop().unwrap().0.next_tick;
        assert_eq!(0, temporal_states.len());
        let tick2 = map.advance(&trigger_id).unwrap();
        assert!(tick2 > tick1);
        // we should get the same (tick2) if we rebuilt the state
        let mut temporal_states = map.build_temporal_state();
        assert_eq!(1, temporal_states.len());
        let tick2_again = temporal_states.pop().unwrap().0.next_tick;
        assert_eq!(tick2, tick2_again);

        // rebuilding the temporal state doesn't advance anything even if time
        // passes
        std::thread::sleep(Duration::from_secs(2));
        let mut temporal_states = map.build_temporal_state();
        assert_eq!(1, temporal_states.len());
        let tick2_again = temporal_states.pop().unwrap().0.next_tick;
        assert_eq!(tick2, tick2_again);
        Ok(())
    }
}
