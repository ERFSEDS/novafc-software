#[cfg(test)]
mod tests;

pub mod traits;
use traits::{GenericTimestamp, Timestamp};

use crate::control::Controls;
use crate::data_acquisition::DataWorkspace;
use crate::data_format::FloatCondition;
use crate::data_format::{
    reference::{Check, Command, State, StateTransition},
    CheckData, CommandObject, ObjectState, MAX_CHECKS_PER_STATE, MAX_COMMANDS_PER_STATE,
};
use heapless::Vec;

pub struct StateMachine<'a, 'b, 'c> {
    current_state: &'a State<'a>,
    start_time: Timestamp,

    /// The instant the last state was activated
    last_transition_time: Timestamp,
    data_workspace: &'b DataWorkspace,
    controls: &'c mut Controls,
}

impl<'a, 'b, 'c> StateMachine<'a, 'b, 'c> {
    pub fn new(
        begin: &'a State<'a>,
        data_workspace: &'b DataWorkspace,
        controls: &'c mut Controls,
    ) -> Self {
        let time = Timestamp::now();
        panic!("fix time");

        #[cfg(feature = "std")]
        println!("State machine starting in state: {}", begin.id);

        Self {
            current_state: begin,
            start_time: time,
            last_transition_time: time,
            data_workspace,
            controls,
        }
    }

    pub fn execute(&mut self) {
        if let Some(transition) = self.execute_state() {
            self.transition(transition);
        }
    }

    fn execute_state(&mut self) -> Option<StateTransition<'a>> {
        // Execute commands
        for command in self.current_state.commands.iter() {
            self.execute_command(command);
        }

        // Execute checks
        for check in self.current_state.checks.iter() {
            if let Some(transition) = self.execute_check(check) {
                return Some(transition);
            }
        }

        // Check for timeout
        if let Some(timeout) = &self.current_state.timeout.get() {
            // Checks if the state has timed out
            panic!("");
            /*if self.state_time.elapsed().unwrap().as_secs_f32() >= timeout.time {
                Some(timeout.transition)
            } else {
                None
            }*/
        } else {
            None
        }
    }

    fn execute_command(&mut self, command: &Command) {
        if !command.was_executed.get() {
            if self.last_transition_time.elapsed() >= command.delay {
                self.controls.set(command.object, command);
                command.was_executed.set(true);
            }
        }
    }

    fn execute_check(&self, check: &Check<'a>) -> Option<StateTransition<'a>> {
        let value = self.data_workspace.get_object(check.data.kind());

        let satisfied = match (check.data, value) {
            (CheckData::ApogeeFlag(expected), ObjectState::Bool(actual)) => expected == actual,
            (CheckData::Altitude(condition), ObjectState::Float(actual)) => match condition {
                FloatCondition::LessThan(expected) => actual < expected,
                FloatCondition::GreaterThan(expected) => actual > expected,
                FloatCondition::Between {
                    upper_bound,
                    lower_bound,
                } => (actual >= upper_bound && actual <= lower_bound),
            },
            (CheckData::Pyro1Continuity(expected), ObjectState::Bool(actual))
            | (CheckData::Pyro2Continuity(expected), ObjectState::Bool(actual))
            | (CheckData::Pyro3Continuity(expected), ObjectState::Bool(actual)) => {
                expected == actual
            }
            // Unreachable here since there would have to be a bug inside data workspace which
            // always returns the same type for a given CheckKind enum, so this would be found
            // deterministically in testing
            _ => unreachable!(
                "mismatched types while executing check with {:?} vs {:?}",
                check.data, value
            ),
        };

        satisfied.then(|| check.transition).flatten()
    }

    fn transition(&mut self, transition: StateTransition<'a>) {
        let new_state = match transition {
            StateTransition::Abort(state) => {
                #[cfg(feature = "std")]
                println!(
                    "[{}s] Aborted to state: {}",
                    self.start_time.elapsed(),
                    state.id
                );
                // Here we would have abort reporting of some kind like some "callback" to the data
                // acquisition module
                state
            }
            StateTransition::Transition(state) => {
                #[cfg(feature = "std")]
                println!(
                    "[{}s] Transitioned to state: {}",
                    self.start_time.elapsed().unwrap().as_secs_f32(),
                    state.id
                );
                // We may also put some kind of transition reporting here or just use state ID's
                state
            }
        };

        // Set the new state and reset the state time
        self.current_state = new_state;
        self.last_transition_time = Timestamp::now();
    }
}

pub struct Timeout<'a> {
    pub time: f32,
    pub transition: StateTransition<'a>,
}

impl<'a> Timeout<'a> {
    pub fn new(time: f32, transition: StateTransition<'a>) -> Self {
        Self { time, transition }
    }
}
