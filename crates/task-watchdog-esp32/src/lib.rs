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

/// Async implementation of task-watchdog for use with ESP32 Embassy implementations.
pub mod embassy_esp32 {
    use super::{info, EmbassyClock};
    use task_watchdog_core::{Clock, HardwareWatchdog, Id, ResetReason, Watchdog, WatchdogConfig};
    use embassy_time::{Instant, Timer};
    use esp_hal::peripherals::TIMG0;
    use esp_hal::timer::timg::MwdtStage;
    use esp_hal::timer::timg::TimerGroup;
    use esp_hal::timer::timg::Wdt;

    /// ESP32 specific watchdog implementation.
    pub struct Esp32Watchdog {
        inner: Wdt<TIMG0>,
    }

    impl Esp32Watchdog {
        /// Create a new ESP32 watchdog.
        #[must_use]
        pub fn new(timg0: TimerGroup<TIMG0>) -> Self {
            let wdt = timg0.wdt;
            Self { inner: wdt }
        }
    }

    impl HardwareWatchdog<EmbassyClock> for Esp32Watchdog {
        fn start(&mut self, timeout: embassy_time::Duration) {
            self.inner.set_timeout(
                MwdtStage::Stage0,
                esp_hal::time::Duration::from_millis(timeout.as_millis()),
            );
            self.inner.enable();
        }

        fn feed(&mut self) {
            self.inner.feed();
        }

        fn trigger_reset(&mut self) -> ! {
            esp_hal::system::software_reset();
        }

        fn reset_reason(&self) -> Option<ResetReason> {
            None
        }
    }

    /// An Embassy ESP32 watchdog runner.
    #[cfg(feature = "alloc")]
    pub struct WatchdogRunner<I>
    where
        I: Id,
    {
        watchdog: embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            core::cell::RefCell<Watchdog<I, Esp32Watchdog, EmbassyClock>>,
        >,
    }

    #[cfg(not(feature = "alloc"))]
    pub struct WatchdogRunner<I, const N: usize>
    where
        I: Id,
    {
        watchdog: embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            core::cell::RefCell<Watchdog<I, N, Esp32Watchdog, EmbassyClock>>,
        >,
    }

    #[cfg(feature = "alloc")]
    impl<I> WatchdogRunner<I>
    where
        I: Id + 'static,
    {
        /// Create a new Embassy-compatible watchdog runner.
        pub fn new(timg0: TimerGroup<TIMG0>, config: WatchdogConfig<EmbassyClock>) -> Self {
            let hw_watchdog = Esp32Watchdog::new(timg0);
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
        pub fn new(timg0: TimerGroup<TIMG0>, config: WatchdogConfig<EmbassyClock>) -> Self {
            let hw_watchdog = Esp32Watchdog::new(timg0);
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
