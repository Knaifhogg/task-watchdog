#![no_std]
#![no_main]

use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use task_watchdog_core::Id;
use task_watchdog_esp32::embassy_esp32::{watchdog_run, WatchdogRunner};

// Define task IDs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TaskId {
    MainTask,
    FeedTask,
}
impl Id for TaskId {}

// Number of tasks we'll monitor
const NUM_TASKS: usize = 2;

static WATCHDOG: StaticCell<WatchdogRunner<TaskId, NUM_TASKS>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    info!("Starting task-watchdog ESP32 example");

    // Initialize ESP32 hardware
    let peripherals = esp_hal::init(Default::default());
    
    // Create and configure hardware watchdog using TIMG0
    let hw_watchdog = task_watchdog_esp32::embassy_esp32::Esp32Watchdog::new(
        esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0),
    );

    let config = task_watchdog_core::WatchdogConfig {
        check_interval: Duration::from_millis(500),
        hardware_timeout: Duration::from_secs(5),
    };

    let watchdog = WatchdogRunner::new(hw_watchdog, config);
    let watchdog = WATCHDOG.init(watchdog);

    // Register tasks with their maximum feed intervals
    watchdog
        .register_task(&TaskId::MainTask, Duration::from_secs(2))
        .await;
    watchdog
        .register_task(&TaskId::FeedTask, Duration::from_secs(3))
        .await;

    info!("Watchdog configured with {} tasks", NUM_TASKS);

    // Spawn watchdog monitoring task (must run continuously)
    unwrap!(spawner.spawn(watchdog_task(watchdog)));

    // Spawn application tasks
    unwrap!(spawner.spawn(main_task(watchdog)));
    unwrap!(spawner.spawn(feed_task(watchdog)));

    // Main executor loop - keep running
    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}

/// Watchdog monitoring task - continuously checks and feeds the hardware watchdog
#[embassy_executor::task]
async fn watchdog_task(w: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
    info!("Watchdog task started");
    watchdog_run(w.create_task()).await
}

/// Main application task - must feed watchdog regularly
#[embassy_executor::task]
async fn main_task(w: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
    info!("Main task started");
    loop {
        info!("Main task: feeding watchdog");
        w.feed(&TaskId::MainTask).await;
        Timer::after(Duration::from_millis(1000)).await;
    }
}

/// Secondary application task - also feeds watchdog regularly
#[embassy_executor::task]
async fn feed_task(w: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
    info!("Feed task started");
    loop {
        info!("Feed task: feeding watchdog");
        w.feed(&TaskId::FeedTask).await;
        Timer::after(Duration::from_millis(2000)).await;
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("Panic: {}", defmt::Display2Format(info));
    loop {}
}
