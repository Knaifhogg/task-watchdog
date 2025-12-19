#![no_std]
#![no_main]

use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_time::{Duration, Timer};
use panic_probe as _;
use static_cell::StaticCell;
use task_watchdog_core::Id;
use task_watchdog_rp::embassy_rp::{watchdog_run, WatchdogRunner};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TaskId {
    MainTask,
    BlinkTask,
}
impl Id for TaskId {}

const NUM_TASKS: usize = 2;

static WATCHDOG: StaticCell<WatchdogRunner<TaskId, NUM_TASKS>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    info!("Starting task-watchdog RP2040 example");

    let p = embassy_rp::init(Default::default());

    // Create GPIO output on LED pin
    let mut led = Output::new(p.PIN_25, Level::Low);

    // Create watchdog
    let hw_watchdog = task_watchdog_rp::embassy_rp::RpWatchdog::new(p.WATCHDOG);

    let config = task_watchdog_core::WatchdogConfig {
        check_interval: Duration::from_millis(500),
        hardware_timeout: Duration::from_secs(5),
    };

    let watchdog = WatchdogRunner::new(hw_watchdog, config);
    let watchdog = WATCHDOG.init(watchdog);

    // Register tasks
    watchdog
        .register_task(&TaskId::MainTask, Duration::from_secs(2))
        .await;
    watchdog
        .register_task(&TaskId::BlinkTask, Duration::from_secs(3))
        .await;

    info!("Watchdog configured with {} tasks", NUM_TASKS);

    // Spawn tasks
    unwrap!(spawner.spawn(watchdog_task(watchdog)));
    unwrap!(spawner.spawn(main_task(watchdog)));
    unwrap!(spawner.spawn(blink_task(watchdog, led)));

    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}

#[embassy_executor::task]
async fn watchdog_task(w: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
    info!("Watchdog task started");
    watchdog_run(w.create_task()).await
}

#[embassy_executor::task]
async fn main_task(w: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
    info!("Main task started");
    loop {
        info!("Main task: feeding watchdog");
        w.feed(&TaskId::MainTask).await;
        Timer::after(Duration::from_millis(1000)).await;
    }
}

#[embassy_executor::task]
async fn blink_task(
    w: &'static WatchdogRunner<TaskId, NUM_TASKS>,
    mut led: embassy_rp::gpio::Output<'static>,
) -> ! {
    info!("Blink task started");
    loop {
        info!("Blink task: feeding watchdog");
        w.feed(&TaskId::BlinkTask).await;
        led.toggle();
        Timer::after(Duration::from_millis(2000)).await;
    }
}
