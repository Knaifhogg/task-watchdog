#![no_std]

#[cfg(feature = "defmt")]
#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

// Re-export logging no-ops if defmt not enabled
#[cfg(not(feature = "defmt"))]
pub(crate) mod log_impl {
    #![allow(unused_macros)]
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

use task_watchdog_core::{Clock, HardwareWatchdog, Id, ResetReason, Watchdog, WatchdogConfig};

/// A system clock implementation for Embassy.
pub struct EmbassyClock;

impl Clock for EmbassyClock {
    type Instant = embassy_time::Instant;
    type Duration = embassy_time::Duration;

    fn now(&self) -> Self::Instant {
        embassy_time::Instant::now()
    }

    fn elapsed_since(&self, instant: Self::Instant) -> Self::Duration {
        embassy_time::Instant::now() - instant
    }

    fn has_elapsed(&self, instant: Self::Instant, duration: &Self::Duration) -> bool {
        (embassy_time::Instant::now() - instant) >= *duration
    }

    fn duration_from_millis(&self, millis: u64) -> Self::Duration {
        embassy_time::Duration::from_millis(millis)
    }
}

/// Async implementation of task-watchdog for use with nRF Embassy implementations.
pub mod embassy_nrf {
    use super::{info, warn, EmbassyClock};
    use task_watchdog_core::{Clock, HardwareWatchdog, Id, ResetReason, Watchdog, WatchdogConfig};
    use embassy_nrf::peripherals::WDT;
    use embassy_nrf::wdt::{Config, Watchdog as NrfWatchdogStruct, WatchdogHandle};
    use embassy_time::{Instant, Timer};

    /// nRF specific watchdog implementation.
    pub struct NrfWatchdog {
        peripheral: Option<WDT>,
        inner: Option<WatchdogHandle>,
    }

    impl NrfWatchdog {
        /// Create a new nRF watchdog.
        #[must_use]
        pub fn new(peripheral: WDT) -> Self {
            Self {
                peripheral: Some(peripheral),
                inner: None,
            }
        }
    }

    impl HardwareWatchdog<EmbassyClock> for NrfWatchdog {
        fn start(&mut self, timeout: embassy_time::Duration) {
            // Convert the requested timeout to ticks (32,768Hz)
            let ticks = timeout.as_micros() * 1_000_000 / 32_768_000;
            if ticks < 15 {
                warn!(
                    "Watchdog timeout {} ticks too small for nRF - will be set to 15 ticks",
                    ticks
                );
                panic!("Watchdog timeout too large for nRF");
            }
            let mut config = Config::default();
            if ticks > u32::MAX as u64 {
                panic!("Watchdog timeout {} ticks too large for nRF", ticks);
            }
            config.timeout_ticks = ticks as u32;
            let peripheral = self
                .peripheral
                .take()
                .expect("nRF Watchdog not properly initialized");

            let (_wdt, [handle]) = NrfWatchdogStruct::try_new(peripheral, config)
                .unwrap_or_else(|_| panic!("Failed to create nRF watchdog"));
            self.inner = Some(handle);
        }

        fn feed(&mut self) {
            self.inner.as_mut().expect("Watchdog not started").pet();
        }

        fn trigger_reset(&mut self) -> ! {
            cortex_m::peripheral::SCB::sys_reset();
        }

        fn reset_reason(&self) -> Option<ResetReason> {
            None
        }
    }

    /// An Embassy nRF watchdog runner.
    #[cfg(feature = "alloc")]
    pub struct WatchdogRunner<I>
    where
        I: Id,
    {
        watchdog: embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            core::cell::RefCell<Watchdog<I, NrfWatchdog, EmbassyClock>>,
        >,
    }

    #[cfg(not(feature = "alloc"))]
    pub struct WatchdogRunner<I, const N: usize>
    where
        I: Id,
    {
        watchdog: embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            core::cell::RefCell<Watchdog<I, N, NrfWatchdog, EmbassyClock>>,
        >,
    }

    #[cfg(feature = "alloc")]
    impl<I> WatchdogRunner<I>
    where
        I: Id + 'static,
    {
        /// Create a new Embassy-compatible watchdog runner.
        pub fn new(hw_watchdog: WDT, config: WatchdogConfig<EmbassyClock>) -> Self {
            let hw_watchdog = NrfWatchdog::new(hw_watchdog);
            let watchdog = Watchdog::new(hw_watchdog, config, EmbassyClock);
            Self {
                watchdog: embassy_sync::mutex::Mutex::new(core::cell::RefCell::new(watchdog)),
            }
        }

        /// Register a task with the watchdog.
        pub async fn register_task(&self, id: &I, max_duration: <EmbassyClock as Clock>::Duration) {
            self.watchdog
                .lock()
                .await
                .borrow_mut()
                .register_task(id, max_duration);
        }

        /// De-register a task with the watchdog.
        pub async fn deregister_task(&self, id: &I) {
            self.watchdog.lock().await.borrow_mut().deregister_task(id);
        }

        /// Feed the watchdog for a specific task.
        pub async fn feed(&self, id: &I) {
            self.watchdog.lock().await.borrow_mut().feed(id);
        }

        /// Start the watchdog.
        pub async fn start(&self) {
            self.watchdog.lock().await.borrow_mut().start();
        }

        /// Trigger a system reset.
        pub async fn trigger_reset(&self) -> ! {
            self.watchdog.lock().await.borrow_mut().trigger_reset()
        }

        /// Get the last reset reason.
        pub async fn reset_reason(&self) -> Option<ResetReason> {
            self.watchdog.lock().await.borrow().reset_reason()
        }

        /// Get the check interval
        pub async fn get_check_interval(&self) -> <EmbassyClock as Clock>::Duration {
            self.watchdog.lock().await.borrow().config.check_interval
        }

        /// Check if any tasks have starved
        pub async fn check_tasks(&self) -> bool {
            self.watchdog.lock().await.borrow_mut().check()
        }
    }

    #[cfg(not(feature = "alloc"))]
    impl<I, const N: usize> WatchdogRunner<I, N>
    where
        I: Id,
    {
        /// Create a new Embassy-compatible watchdog runner.
        pub fn new(hw_watchdog: WDT, config: WatchdogConfig<EmbassyClock>) -> Self {
            let hw_watchdog = NrfWatchdog::new(hw_watchdog);
            let watchdog = Watchdog::new(hw_watchdog, config, EmbassyClock);
            Self {
                watchdog: embassy_sync::mutex::Mutex::new(core::cell::RefCell::new(watchdog)),
            }
        }

        /// Register a task with the watchdog.
        pub async fn register_task(&self, id: &I, max_duration: <EmbassyClock as Clock>::Duration) {
            self.watchdog
                .lock()
                .await
                .borrow_mut()
                .register_task(id, max_duration)
                .ok();
        }

        /// Deregister a task with the watchdog.
        pub async fn deregister_task(&self, id: &I) {
            self.watchdog.lock().await.borrow_mut().deregister_task(id);
        }

        /// Feed the watchdog for a specific task.
        pub async fn feed(&self, id: &I) {
            self.watchdog.lock().await.borrow_mut().feed(id);
        }

        /// Start the watchdog.
        pub async fn start(&self) {
            self.watchdog.lock().await.borrow_mut().start();
        }

        /// Trigger a system reset.
        pub async fn trigger_reset(&self) -> ! {
            self.watchdog.lock().await.borrow_mut().trigger_reset()
        }

        /// Get the last reset reason.
        pub async fn reset_reason(&self) -> Option<ResetReason> {
            self.watchdog.lock().await.borrow().reset_reason()
        }

        /// Get the check interval
        pub async fn get_check_interval(&self) -> <EmbassyClock as Clock>::Duration {
            self.watchdog.lock().await.borrow().config.check_interval
        }

        /// Check if any tasks have starved
        pub async fn check_tasks(&self) -> bool {
            self.watchdog.lock().await.borrow_mut().check()
        }
    }

    #[cfg(feature = "alloc")]
    pub struct WatchdogTask<I>
    where
        I: 'static + Id,
    {
        runner: &'static WatchdogRunner<I>,
    }

    #[cfg(feature = "alloc")]
    impl<I> WatchdogRunner<I>
    where
        I: 'static + Id,
    {
        pub fn create_task(&'static self) -> WatchdogTask<I> {
            WatchdogTask { runner: self }
        }
    }

    #[cfg(feature = "alloc")]
    pub async fn watchdog_run<I>(task: WatchdogTask<I>) -> !
    where
        I: 'static + Id,
    {
        info!("Watchdog runner started");

        task.runner.start().await;

        let interval = task.runner.get_check_interval().await;
        let mut check_time = Instant::now() + interval;

        loop {
            let _ = task.runner.check_tasks().await;
            Timer::at(check_time).await;
            check_time += interval;
        }
    }

    #[cfg(not(feature = "alloc"))]
    pub struct NoAllocWatchdogTask<I, const N: usize>
    where
        I: 'static + Id,
    {
        runner: &'static WatchdogRunner<I, N>,
    }

    #[cfg(not(feature = "alloc"))]
    impl<I, const N: usize> WatchdogRunner<I, N>
    where
        I: 'static + Id,
    {
        pub fn create_task(&'static self) -> NoAllocWatchdogTask<I, N> {
            NoAllocWatchdogTask { runner: self }
        }
    }

    #[cfg(not(feature = "alloc"))]
    pub async fn watchdog_run<I, const N: usize>(task: NoAllocWatchdogTask<I, N>) -> !
    where
        I: 'static + Id,
    {
        info!("Watchdog runner started");

        task.runner.start().await;

        let interval = task.runner.get_check_interval().await;
        let mut check_time = Instant::now() + interval;

        loop {
            let _ = task.runner.check_tasks().await;
            Timer::at(check_time).await;
            check_time += interval;
        }
    }

    // Re-export logging macros if not using defmt
    #[cfg(not(feature = "defmt"))]
    use super::log_impl::*;
}

// Re-export core types for convenience
pub use task_watchdog_core::{Clock, Error, HardwareWatchdog, Id, ResetReason, Task, Watchdog, WatchdogConfig};
