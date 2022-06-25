use super::{Id, Interval, PortDef, Range};
use crate::interval_checking::SExp;
use linked_hash_map::LinkedHashMap;
use std::fmt::Display;

/// An interval time expression that denotes a max of sums expression.
#[derive(Default, Hash, Clone, PartialEq, Eq)]
pub struct FsmIdxs {
    fsms: LinkedHashMap<Id, u64>,
}

impl PartialOrd for FsmIdxs {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;

        if self.fsms.len() != self.fsms.len() {
            return None;
        }
        let mut cur_order: Option<Ordering> = None;
        for (ev, st1) in &self.fsms {
            if let Some(st2) = other.fsms.get(ev) {
                // If there is an ordering defined, check that the current
                // comparison maintains it.
                if let Some(ord) = cur_order {
                    if ord != st1.cmp(st2) {
                        return None;
                    }
                } else {
                    cur_order = Some(st1.cmp(st2))
                }
            } else {
                // Other hashmap is missing an event
                return None;
            }
        }
        log::debug!("Comparing {self} & {other}: {cur_order:?}");

        cur_order
    }
}

impl Display for FsmIdxs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fs = self.fsms.iter();
        let (ev0, st0) = fs.next().unwrap();

        let out = fs.fold(format!("{ev0}+{st0}"), |acc, (ev, st)| {
            format!("max({acc}, {ev}+{st})")
        });

        write!(f, "{}", out)
    }
}

impl FsmIdxs {
    /// Construct an index with exactly one FSM
    pub fn unit(name: Id, state: u64) -> Self {
        let mut hs = LinkedHashMap::with_capacity(1);
        hs.insert(name, state);
        FsmIdxs { fsms: hs }
    }

    /// Attempts to convert this FsmIdxs to a single (event, state) pair if possible.
    pub fn as_unit(&self) -> Option<(&Id, &u64)> {
        if self.fsms.len() == 1 {
            self.fsms.iter().next()
        } else {
            None
        }
    }

    /// Add a new fsm state T+n to the expression
    pub fn insert(&mut self, name: Id, state: u64) {
        self.fsms.insert(name, state);
    }

    /// Return the names of all events used in the max expression
    pub fn events(&self) -> impl Iterator<Item = &Id> {
        self.fsms.iter().map(|(ev, _)| ev)
    }

    /// Increment all the the FSM states by the provided value
    pub fn increment(self, n: u64) -> Self {
        let fsms = self
            .fsms
            .into_iter()
            .map(|(name, state)| (name, state + n))
            .collect();
        FsmIdxs { fsms }
    }
}

impl super::TimeRep for FsmIdxs {
    fn resolve(&self, bindings: &std::collections::HashMap<Id, &Self>) -> Self {
        let mut out = LinkedHashMap::with_capacity(self.fsms.len());
        for (name, state) in &self.fsms {
            let idxs = (*bindings
                .get(name)
                .unwrap_or_else(|| panic!("No binding for {}", name)))
            .clone()
            .increment(*state);
            out.extend(&mut idxs.fsms.into_iter());
        }
        FsmIdxs { fsms: out }
    }
}

impl From<&FsmIdxs> for SExp {
    fn from(idxs: &FsmIdxs) -> Self {
        idxs.fsms
            .iter()
            .map(|(name, state)| {
                SExp(if *state == 0 {
                    format!("{name}")
                } else {
                    format!("(+ {name} {state})")
                })
            })
            .reduce(|acc, fsm| SExp(format!("(max {} {})", acc, fsm)))
            .unwrap()
    }
}

impl Interval<FsmIdxs> {
    /// Attempts to convert interval into (event, start, end). Only possible
    /// when the interval uses exactly the one event for both start and end.
    pub fn as_exact_offset(&self) -> Option<(&Id, u64, u64)> {
        self.exact.as_ref().and_then(|inv| inv.as_offset())
    }
}

impl Range<FsmIdxs> {
    /// Convert this interval into an offset. Only possible when interval uses
    /// exactly one event for both start and end.
    pub fn as_offset(&self) -> Option<(&Id, u64, u64)> {
        let Range { start, end } = &self;

        start.as_unit().and_then(|(s_ev, s_st)| {
            end.as_unit().and_then(|(e_ev, e_st)| {
                if s_ev == e_ev {
                    Some((s_ev, *s_st, *e_st))
                } else {
                    None
                }
            })
        })
    }
}

impl PortDef<FsmIdxs> {
    /// Attempts to convert this port definition into an interface signal and return the associated
    /// event.
    pub fn as_interface_port(&self) -> Option<&Id> {
        self.liveness
            .as_ref()
            .and_then(|inv| inv.as_exact_offset().map(|off| off.0))
    }
}
