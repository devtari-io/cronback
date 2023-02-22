use std::collections::{HashMap, HashSet};

use proto::trigger_proto::Trigger;

// message Trigger {
//   string id = 1;
//   string owner_id = 2;
//   /// [Future] User supplied identifier, unique per owner account
//   optional string reference_id = 3;
//   optional string name = 4;
//   optional string description = 5;
//   google.protobuf.Timestamp created_at = 6;
//   // TODO: Explore having multiple endpoints with independent status
//   Endpoint endpoint = 7;
//   Payload payload = 8;
//   google.protobuf.Duration timeout = 9;
//   Schedule schedule = 10;
//   TriggerStatus status = 11;
//   OnStatusHandler on_success = 12;
//   OnStatusHandler on_failure = 13;
//   RetryPolicy event_retry_policy = 14;
// }

///
/// Maintains the set of `active` triggers in memory. Expired triggers are
/// evicted to save space.
///
/// dirty_triggers holds the set of triggers that has been updated since the
/// last time the spinner has looked at it. The spinner resets the set after
/// reloading.
#[derive(Default, Debug)]
pub(crate) struct ActiveTriggerMap {
    state: HashMap<String, Trigger>,
    /// The set of trigger Ids that has been updated
    dirty_triggers: HashSet<String>,
}

impl ActiveTriggerMap {
    /// Inserts or updates a trigger if exists
    pub fn add_or_update(&mut self, trigger: Trigger) {
        let trigger_id = trigger.id.clone();
        self.state.insert(trigger_id.clone(), trigger);
        self.trigger_updated(trigger_id);
    }

    pub fn is_dirty(&self) -> bool {
        !self.dirty_triggers.is_empty()
    }

    pub fn reset_dirty(&mut self) {
        self.dirty_triggers.clear();
    }
    pub fn clear(&mut self) {
        self.state.clear();
        self.dirty_triggers.clear();
    }

    //// PRIVATE
    fn trigger_updated(&mut self, trigger_id: String) {
        self.dirty_triggers.insert(trigger_id);
    }
}

/// Metadata exclusively owned by the spinner, keeps the Id of the installed
/// trigger along with its next tick.
///
/// The spinner maintains a max-heap of those jobs to determine which Triggers
/// need to be evaluated at each loop.
pub(crate) struct TemporalState {
    #[allow(unused)]
    pub trigger_id: String,
    // timestamp in ms of the next tick
    pub next_tick: u64,
}

impl PartialOrd for TemporalState {
    fn partial_cmp(&self, other: &TemporalState) -> Option<std::cmp::Ordering> {
        self.next_tick.partial_cmp(&other.next_tick)
    }
}
impl Ord for TemporalState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.next_tick.cmp(&other.next_tick)
    }
}

impl PartialEq for TemporalState {
    fn eq(&self, other: &Self) -> bool {
        self.next_tick == other.next_tick
    }
}
impl Eq for TemporalState {}
