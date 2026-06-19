//! Scheduler participant (spec 011 P6) — fires commands when their scheduled time
//! arrives. Time is injected (a `now` epoch-seconds value), so the firing logic
//! is deterministic under a fake clock; a thin real-time wrapper would call
//! [`SchedulerAgent::tick`] on an interval with the wall clock.
//!
//! This is the "schedule it" leg of "say it / click it / schedule it": a tick
//! dispatches a [`Command`] through the same validated intake (P3) any other
//! surface uses.

use crate::bus::{AgentContext, CommandAuthorizer, Command};

/// A command queued to fire once at or after `fire_at` (epoch seconds).
pub struct ScheduledCommand {
    pub fire_at: u64,
    pub command: Command,
    fired: bool,
}

impl ScheduledCommand {
    pub fn new(fire_at: u64, command: Command) -> Self {
        Self { fire_at, command, fired: false }
    }
}

/// Holds a schedule and dispatches due commands through the bus intake.
pub struct SchedulerAgent {
    ctx: AgentContext,
    schedule: Vec<ScheduledCommand>,
}

impl SchedulerAgent {
    pub fn new(ctx: AgentContext, schedule: Vec<ScheduledCommand>) -> Self {
        Self { ctx, schedule }
    }

    /// Dispatch every not-yet-fired command whose `fire_at <= now`. Returns how
    /// many fired this tick. Idempotent per entry — a fired command never fires
    /// again. (Recurring schedules re-queue; out of scope for v1.)
    pub fn tick(&mut self, now: u64, authz: &dyn CommandAuthorizer) -> usize {
        let Self { ctx, schedule } = self;
        let mut fired = 0;
        for entry in schedule.iter_mut() {
            if !entry.fired && entry.fire_at <= now && ctx.dispatch(entry.command.clone(), authz).is_ok() {
                entry.fired = true;
                fired += 1;
            }
        }
        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{AllowAll, Bus, RunCommand};
    use std::sync::Arc;

    #[test]
    fn fires_only_when_due_and_only_once() {
        let bus = Arc::new(Bus::new(8));
        let mut rx = bus.take_commands().expect("intake");
        let reg = crate::bus::AgentRegistry::new(Arc::clone(&bus));
        let ctx = reg.context(crate::bus::ParticipantId {
            node: crate::bus::NodeId("local".into()),
            kind: crate::bus::ParticipantKind::Scheduler,
            name: "scheduler".into(),
            instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
        });
        let mut sched = SchedulerAgent::new(
            ctx,
            vec![ScheduledCommand::new(100, Command::Run(RunCommand::Start { goal: "friday report".into() }))],
        );

        // Before the fire time → nothing dispatched.
        assert_eq!(sched.tick(50, &AllowAll), 0);
        assert!(rx.try_recv().is_err(), "not due, nothing on the intake");

        // At/after the fire time → the command is dispatched once.
        assert_eq!(sched.tick(150, &AllowAll), 1);
        let cmd = rx.try_recv().expect("command fired").command;
        assert_eq!(cmd, Command::Run(RunCommand::Start { goal: "friday report".into() }));

        // A later tick does not re-fire it.
        assert_eq!(sched.tick(200, &AllowAll), 0);
        assert!(rx.try_recv().is_err());
    }
}
