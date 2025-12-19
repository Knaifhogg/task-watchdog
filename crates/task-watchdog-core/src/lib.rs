//! # task-watchdog core
//!
//! Platform-agnostic core logic for the task-watchdog library.
//!
//! This crate provides the fundamental traits and generic watchdog implementation
//! that enables multiple task watchdogs to be multiplexed into a single hardware
//! watchdog timer.
//!
//! Platform-specific implementations (Embassy, HALs) are provided in separate crates:
//! - `task-watchdog-rp` - RP2040/RP2350
//! - `task-watchdog-stm32` - STM32
//! - `task-watchdog-nrf` - nRF
//! - `task-watchdog-esp32` - ESP32

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// Apache 2.0 or MIT licensed, at your option.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;

#[cfg(feature = "defmt")]
#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

// A replacement for the defmt logging macros, when defmt is not provided
#[cfg(not(feature = "defmt"))]
mod log_impl {
    #![allow(unused_macros)]
    #![allow(unused_imports)]
    // Macros are defined as _ to avoid conflicts with built-in attribute names
    macro_rules! _trace {
        ($($arg:tt)*) => {};
    }
    macro_rules! _debug {
        ($($arg:tt)*) => {};
    }
    macro_rules! _info {
        ($($arg:tt)*) => {};
    }
    macro_rules! _warn {
        ($($arg:tt)*) => {};
    }
    macro_rules! _error {
        ($($arg:tt)*) => {};
    }
    pub(crate) use _debug as debug;
    pub(crate) use _error as error;
    pub(crate) use _info as info;
    pub(crate) use _trace as trace;
    pub(crate) use _warn as warn;
}
#[cfg(not(feature = "defmt"))]
use log_impl::*;

/// Represents a hardware-level watchdog that can be fed and reset the system.
pub trait HardwareWatchdog<C: Clock> {
    /// Start the hardware watchdog with the given timeout.
    fn start(&mut self, timeout: C::Duration);

    /// Feed the hardware watchdog to prevent a system reset.
    fn feed(&mut self);

    /// Trigger a hardware reset.
    fn trigger_reset(&mut self) -> !;

    /// Get the reason for the last reset, if available.
    fn reset_reason(&self) -> Option<ResetReason>;
}

/// Represents the reason for a system reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ResetReason {
    /// Reset was forced by software.
    Forced,

    /// Reset was caused by watchdog timeout.
    TimedOut,
}

/// Configuration for the watchdog.
#[derive(Debug, Clone, Copy)]
pub struct WatchdogConfig<C: Clock> {
    /// Timeout to start the hardware watchdog with.
    pub hardware_timeout: C::Duration,

    /// Interval at which to check if tasks have fed the watchdog.  Must be
    /// less than the hardware timeout, or the hardware watchdog will reset
    /// the system, before the task-watchdog has a chance to check tasks and
    /// feed it.
    pub check_interval: C::Duration,
}

impl<C: Clock> WatchdogConfig<C> {
    /// Create a new configuration with specified timeout values
    pub fn new(hardware_timeout_ms: u64, check_interval_ms: u64, clock: &C) -> Self {
        Self {
            hardware_timeout: clock.duration_from_millis(hardware_timeout_ms),
            check_interval: clock.duration_from_millis(check_interval_ms),
        }
    }

    /// Create a default configuration with standard timeout values:
    /// - Hardware timeout: 5000ms
    /// - Check interval: 1000ms
    pub fn default(clock: &C) -> Self {
        Self::new(5000, 1000, clock)
    }
}

#[cfg(all(feature = "alloc", feature = "defmt"))]
/// Trait for task identifiers.
pub trait Id: PartialEq + Eq + Ord + defmt::Format + Clone + Copy {}
#[cfg(all(feature = "alloc", not(feature = "defmt")))]
/// Trait for task identifiers.
pub trait Id: PartialEq + Eq + Ord + core::fmt::Debug + Clone + Copy {}
#[cfg(all(not(feature = "alloc"), feature = "defmt"))]
/// Trait for task identifiers.
///
/// You need an object implementing this trait (likely by using derive()) in
/// order to identify tasks to the watchdog.  This can be any object you
/// like implementing this trait, although an `enum` would probably be a
/// good choice.
pub trait Id: PartialEq + Eq + defmt::Format + Clone + Copy {}
#[cfg(all(not(feature = "alloc"), not(feature = "defmt")))]
/// Trait for task identifiers.
pub trait Id: PartialEq + Eq + core::fmt::Debug + Clone + Copy {}

/// Represents a task monitored by the watchdog.
#[derive(Debug, Clone)]
pub struct Task<I: Id, C: Clock> {
    /// The task identifier.
    #[allow(dead_code)]
    id: I,

    /// The last time the task was fed.
    last_feed: C::Instant,

    /// Maximum duration between feeds.
    max_duration: C::Duration,
}

impl<I: Id, C: Clock> Task<I, C> {
    /// Creates a new Task object for registration with the watchdog.
    pub fn new(id: I, max_duration: C::Duration, clock: &C) -> Self {
        Self {
            id,
            last_feed: clock.now(),
            max_duration,
        }
    }

    /// Feed the task to indicate it's still active.
    pub(crate) fn feed(&mut self, clock: &C) {
        self.last_feed = clock.now();
    }

    /// Check if this task has starved the watchdog.
    pub(crate) fn is_starved(&self, clock: &C) -> bool {
        clock.has_elapsed(self.last_feed, &self.max_duration)
    }
}

/// A trait for time-keeping implementations.
pub trait Clock {
    /// A type representing a specific instant in time.
    type Instant: Copy;

    /// A type representing a duration of time
    type Duration: Copy;

    /// Get the current time.
    fn now(&self) -> Self::Instant;

    /// Calculate the duration elapsed since the given instant.
    fn elapsed_since(&self, instant: Self::Instant) -> Self::Duration;

    /// Check if a duration has passed since the given instant.
    fn has_elapsed(&self, instant: Self::Instant, duration: &Self::Duration) -> bool;

    /// Create a duration from milliseconds.
    fn duration_from_millis(&self, millis: u64) -> Self::Duration;
}

/// A watchdog that monitors multiple tasks and resets the system if any task fails to feed.
#[cfg(feature = "alloc")]
pub struct Watchdog<I: Id, W: HardwareWatchdog<C>, C: Clock> {
    /// The hardware watchdog.
    pub(crate) hw_watchdog: W,

    /// Tasks being monitored.
    pub(crate) tasks: BTreeMap<I, Task<I, C>>,

    /// Configuration.
    pub(crate) config: WatchdogConfig<C>,

    /// Clock for time-keeping.
    pub(crate) clock: C,
}

#[cfg(feature = "alloc")]
impl<I: Id, W: HardwareWatchdog<C>, C: Clock> Watchdog<I, W, C> {
    /// Create a new watchdog with the given hardware watchdog and configuration.
    pub fn new(hw_watchdog: W, config: WatchdogConfig<C>, clock: C) -> Self {
        Self {
            hw_watchdog,
            tasks: BTreeMap::new(),
            config,
            clock,
        }
    }

    /// Register a task with the watchdog.
    pub fn register_task(&mut self, id: &I, max_duration: C::Duration) {
        let task = Task::new(*id, max_duration, &self.clock);
        self.tasks.insert(*id, task);
        debug!("Registered task: {:?}", id);
    }

    /// Deregister a task from the watchdog.
    pub fn deregister_task(&mut self, id: &I) {
        #[allow(clippy::if_same_then_else)]
        if self.tasks.remove(id).is_some() {
            debug!("Deregistered task: {:?}", id);
        } else {
            debug!("Attempted to deregister unknown task: {:?}", id);
        }
    }

    /// Feed the watchdog for a specific task.
    pub fn feed(&mut self, id: &I) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.feed(&self.clock);
        } else {
            warn!("Attempt to feed unknown task: {:?}", id);
        }
    }

    /// Start the watchdog.
    pub fn start(&mut self) {
        // Feed all registered tasks
        for task in self.tasks.values_mut() {
            task.feed(&self.clock);
        }

        // Start the hardware watchdog
        self.hw_watchdog.start(self.config.hardware_timeout);

        info!("Watchdog started");
    }

    /// Check if any tasks have starved the watchdog and take appropriate action.
    pub fn check(&mut self) -> bool {
        // Check if any tasks have starved
        let mut starved = false;
        for task in self.tasks.values() {
            if task.is_starved(&self.clock) {
                error!("Task {:?} has starved the watchdog", task.id);
                starved = true;
            }
        }

        // Either feed the hardware watchdog or return that we have a starved task
        if !starved {
            self.hw_watchdog.feed();
        }

        starved
    }

    /// Trigger a system reset.
    pub fn trigger_reset(&mut self) -> ! {
        warn!("Triggering watchdog reset");
        self.hw_watchdog.trigger_reset()
    }

    /// Get the reason for the last reset.
    pub fn reset_reason(&self) -> Option<ResetReason> {
        self.hw_watchdog.reset_reason()
    }
}

/// A version of the Watchdog that doesn't require heap allocation.
/// This uses a fixed-size array for task storage.
#[cfg(not(feature = "alloc"))]
pub struct Watchdog<I, const N: usize, W, C>
where
    I: Id,
    W: HardwareWatchdog<C>,
    C: Clock,
{
    /// The hardware watchdog.
    pub(crate) hw_watchdog: W,

    /// Tasks being monitored.
    pub(crate) tasks: [Option<Task<I, C>>; N],

    /// Configuration.
    pub(crate) config: WatchdogConfig<C>,

    /// Clock for time-keeping.
    pub(crate) clock: C,
}

/// Errors that can occur when interacting with the watchdog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// No slots available to register a task.
    NoSlotsAvailable,
}

#[cfg(not(feature = "alloc"))]
impl<I: Id, W: HardwareWatchdog<C>, C: Clock, const N: usize> Watchdog<I, N, W, C> {
    /// Create a new watchdog with the given hardware watchdog and configuration.
    ///
    /// Arguments:
    /// * `hw_watchdog` - The hardware watchdog to use.
    /// * `config` - The configuration for the watchdog.
    /// * `clock` - The clock implementation to use for time-keeping.
    pub fn new(hw_watchdog: W, config: WatchdogConfig<C>, clock: C) -> Self {
        Self {
            hw_watchdog,
            tasks: [const { None }; N],
            config,
            clock,
        }
    }

    /// Register a task with the watchdog.
    ///
    /// Arguments:
    /// * `id` - The task identifier.
    /// * `max_duration` - The maximum duration between feeds.
    ///
    /// # Errors
    /// If there are no available slots to register the task, an error will be returned.
    pub fn register_task(&mut self, id: &I, max_duration: C::Duration) -> Result<(), Error> {
        for slot in &mut self.tasks {
            if slot.is_none() {
                *slot = Some(Task::new(*id, max_duration, &self.clock));
                debug!("Registered task: {:?}", id);
                return Ok(());
            }
        }

        error!("Failed to register task: {:?} - no slots available", id);
        Err(Error::NoSlotsAvailable)
    }

    /// Deregister a task from the watchdog.
    pub fn deregister_task(&mut self, id: &I) {
        for slot in &mut self.tasks {
            if let Some(task) = slot {
                if core::mem::discriminant(&task.id) == core::mem::discriminant(id) {
                    *slot = None;
                    debug!("Deregistered task: {:?}", id);
                    return;
                }
            }
        }

        info!("Attempted to deregister unknown task: {:?}", id);
    }

    /// Feed the watchdog for a specific task.
    pub fn feed(&mut self, id: &I) {
        let fed = self.tasks.iter_mut().flatten().any(|task| {
            if core::mem::discriminant(&task.id) == core::mem::discriminant(id) {
                task.feed(&self.clock);
                true
            } else {
                false
            }
        });

        if !fed {
            warn!("Attempt to feed unknown task: {:?}", id);
        }
    }

    /// Start the watchdog and enable the hardware watchdog.
    pub fn start(&mut self) {
        // Feed all registered tasks
        self.tasks.iter_mut().flatten().for_each(|task| {
            task.feed(&self.clock);
        });

        // Start the hardware watchdog
        self.hw_watchdog.start(self.config.hardware_timeout);

        info!("Watchdog started");
    }

    /// Check if any tasks have starved the watchdog and take appropriate action.
    pub fn check(&mut self) -> bool {
        let mut starved = false;
        self.tasks.iter_mut().flatten().for_each(|task| {
            if task.is_starved(&self.clock) {
                error!("Task {:?} has starved the watchdog", task.id);
                starved = true;
            }
        });

        if !starved {
            self.hw_watchdog.feed();
        }

        starved
    }

    /// Trigger a system reset.
    pub fn trigger_reset(&mut self) -> ! {
        warn!("Triggering watchdog reset");
        self.hw_watchdog.trigger_reset()
    }

    /// Get the reason for the last reset.
    pub fn reset_reason(&self) -> Option<ResetReason> {
        self.hw_watchdog.reset_reason()
    }
}

/// A system clock implementation using core time types.
pub struct CoreClock;

impl Clock for CoreClock {
    type Instant = u64;
    type Duration = core::time::Duration;

    fn now(&self) -> Self::Instant {
        // In real code, this would use a hardware timer
        // This is just a simple example
        static mut MILLIS: u64 = 0;
        unsafe {
            MILLIS += 1;
            MILLIS
        }
    }

    fn elapsed_since(&self, instant: Self::Instant) -> Self::Duration {
        let now = self.now();
        let elapsed_ms = now.saturating_sub(instant);
        core::time::Duration::from_millis(elapsed_ms)
    }

    fn has_elapsed(&self, instant: Self::Instant, duration: &Self::Duration) -> bool {
        self.elapsed_since(instant) >= *duration
    }

    fn duration_from_millis(&self, millis: u64) -> Self::Duration {
        core::time::Duration::from_millis(millis)
    }
}
