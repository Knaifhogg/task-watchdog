#![no_std]

use task_watchdog_core::{Clock, HardwareWatchdog, Id, ResetReason, Watchdog, WatchdogConfig};

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

/// A system clock implementation for Embassy.
#[cfg(feature = "embassy")]
pub struct EmbassyClock;

#[cfg(feature = "embassy")]
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

/// Synchronous implementation of task-watchdog for use with RP2040 and RP2350 HALs.
#[cfg(any(feature = "rp2040-hal", feature = "rp2350-hal"))]
pub mod rp_hal {
    use task_watchdog_core::{Clock, HardwareWatchdog, ResetReason};
    use hal::fugit::{Duration as RpHalDuration, MicrosDurationU32};
    #[cfg(feature = "rp2350-hal")]
    use hal::timer::CopyableTimer0;
    use hal::timer::{Instant as RpHalInstant, Timer as RpHalTimer};
    use hal::watchdog::Watchdog as RpHalWatchdog;
    #[cfg(feature = "rp2040-hal")]
    use rp2040_hal as hal;
    #[cfg(feature = "rp2350-hal")]
    use rp235x_hal as hal;

    /// A simple clock implementation based on hal::timer::Timer
    #[cfg(feature = "rp2040-hal")]
    pub struct RpHalClock {
        inner: RpHalTimer,
    }
    #[cfg(feature = "rp2040-hal")]
    impl RpHalClock {
        pub fn new(timer: RpHalTimer) -> Self {
            Self { inner: timer }
        }
    }
    #[cfg(feature = "rp2350-hal")]
    pub struct RpHalClock {
        inner: RpHalTimer<CopyableTimer0>,
    }
    #[cfg(feature = "rp2350-hal")]
    impl RpHalClock {
        pub fn new(timer: RpHalTimer<CopyableTimer0>) -> Self {
            Self { inner: timer }
        }
    }

    /// Implement the Clock trait for [`RpHalClock`]
    impl Clock for RpHalClock {
        type Instant = RpHalInstant;
        type Duration = RpHalDuration<u64, 1, 1_000_000>;

        fn now(&self) -> Self::Instant {
            self.inner.get_counter()
        }

        fn elapsed_since(&self, instant: Self::Instant) -> Self::Duration {
            self.now().checked_duration_since(instant).unwrap()
        }

        fn has_elapsed(&self, instant: Self::Instant, duration: &Self::Duration) -> bool {
            (self.now() - instant) >= *duration
        }

        fn duration_from_millis(&self, millis: u64) -> Self::Duration {
            RpHalDuration::<u64, 1, 1_000_000>::millis(millis as u64)
        }
    }

    /// A hardware watchdog implementation using the RP2040/RP2350 HAL.
    pub struct RpHalTaskWatchdog {
        inner: RpHalWatchdog,
    }
    impl RpHalTaskWatchdog {
        pub fn new(watchdog: RpHalWatchdog) -> Self {
            Self { inner: watchdog }
        }
    }

    /// Implement the HardwareWatchdog trait for the HAL watchdog.
    impl HardwareWatchdog<RpHalClock> for RpHalTaskWatchdog {
        fn start(&mut self, timeout: <RpHalClock as Clock>::Duration) {
            let timeout_micros = timeout.to_micros();
            assert!(timeout_micros <= u32::MAX as u64);
            let micros_dur_u32: MicrosDurationU32 =
                MicrosDurationU32::micros(timeout_micros as u32);
            self.inner.start(micros_dur_u32);
        }

        fn feed(&mut self) {
            self.inner.feed();
        }

        fn trigger_reset(&mut self) -> ! {
            hal::reset()
        }

        fn reset_reason(&self) -> Option<ResetReason> {
            None
        }
    }
}

/// Async implementation of task-watchdog for use with RP2040 and RP2350 Embassy implementations.
#[cfg(any(feature = "rp2040-embassy", feature = "rp2350-embassy"))]
pub mod embassy_rp {
    use super::{info, EmbassyClock};
    use task_watchdog_core::{Clock, HardwareWatchdog, Id, ResetReason, Watchdog, WatchdogConfig};
    use embassy_rp::peripherals::WATCHDOG;
    use embassy_rp::watchdog as rp_watchdog;
    use embassy_rp::Peri;
    use embassy_time::{Instant, Timer};

    /// RP2040/RP2350-specific watchdog implementation.
    pub struct RpWatchdog {
        inner: rp_watchdog::Watchdog,
    }

    impl RpWatchdog {
        /// Create a new RP2040/RP2350 watchdog.
        #[must_use]
        pub fn new(peripheral: Peri<'static, WATCHDOG>) -> Self {
            Self {
                inner: rp_watchdog::Watchdog::new(peripheral),
            }
        }
    }

    /// Implement the HardwareWatchdog trait for the RP2040/RP2350 watchdog.
    impl HardwareWatchdog<EmbassyClock> for RpWatchdog {
        fn start(&mut self, timeout: <EmbassyClock as Clock>::Duration) {
            self.inner.start(timeout);
        }

        fn feed(&mut self) {
            self.inner.feed();
        }

        fn trigger_reset(&mut self) -> ! {
            self.inner.trigger_reset();
            panic!("Triggering reset via watchdog failed");
        }

        fn reset_reason(&self) -> Option<ResetReason> {
            self.inner.reset_reason().map(|reason| match reason {
                embassy_rp::watchdog::ResetReason::Forced => ResetReason::Forced,
                embassy_rp::watchdog::ResetReason::TimedOut => ResetReason::TimedOut,
            })
        }
    }

    /// An Embassy RP2040/RP2350 watchdog runner.
    #[cfg(feature = "alloc")]
    pub struct WatchdogRunner<I>
    where
        I: Id,
    {
        watchdog: embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            core::cell::RefCell<Watchdog<I, RpWatchdog, EmbassyClock>>,
        >,
    }

    /// An Embassy RP2040/RP2350 watchdog runner.
    #[cfg(not(feature = "alloc"))]
    pub struct WatchdogRunner<I, const N: usize>
    where
        I: Id,
    {
        watchdog: embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            core::cell::RefCell<Watchdog<I, N, RpWatchdog, EmbassyClock>>,
        >,
    }

    #[cfg(feature = "alloc")]
    impl<I> WatchdogRunner<I>
    where
        I: Id + 'static,
    {
        /// Create a new Embassy-compatible watchdog runner.
        pub fn new(
            hw_watchdog: Peri<'static, WATCHDOG>,
            config: WatchdogConfig<EmbassyClock>,
        ) -> Self {
            let hw_watchdog = RpWatchdog::new(hw_watchdog);
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
        pub fn new(
            hw_watchdog: Peri<'static, WATCHDOG>,
            config: WatchdogConfig<EmbassyClock>,
        ) -> Self {
            let hw_watchdog = RpWatchdog::new(hw_watchdog);
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

    // For alloc feature
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

    /// A version of the Watchdog Task when not using the `alloc` feature.
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
