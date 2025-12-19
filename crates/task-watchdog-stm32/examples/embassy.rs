// STM32 (F103) async example using Embassy
// Two tasks: MainTask and SensorTask
// Demonstrates watchdog timeout behavior when SensorTask stops feeding

use core::mem::discriminant;
use task_watchdog_stm32::prelude::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

// Task identifiers
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TaskId {
    MainTask,
    SensorTask,
}

// Configure number of tasks
const NUM_TASKS: usize = 2;

// Create static watchdog instance
static WATCHDOG: StaticCell<WatchdogRunner<TaskId, NUM_TASKS>> = StaticCell::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_stm32::init(Default::default());
    
    // Create and initialize watchdog
    let watchdog = WatchdogRunner::new(
        p.IWDG,
        WatchdogConfig {
            hardware_timeout: Duration::from_secs(10),
            check_interval: Duration::from_secs(2),
        },
    );
    
    let watchdog = WATCHDOG.init(watchdog);
    
    let executor = embassy_executor::Executor::new();
    
    executor.run(|spawner| {
        let _ = spawner.spawn(watchdog_task(watchdog));
        let _ = spawner.spawn(main_task(watchdog));
        let _ = spawner.spawn(sensor_task(watchdog));
    })
}

// Watchdog monitoring task
#[embassy_executor::task]
async fn watchdog_task(w: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
    watchdog_run(w.create_task()).await
}

// Main application task
#[embassy_executor::task]
async fn main_task(w: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
    // Register with 5 second timeout
    w.register_task(&TaskId::MainTask, Duration::from_secs(5))
        .await;

    loop {
        // Feed the watchdog every second
        w.feed(&TaskId::MainTask).await;
        defmt::info!("MainTask: fed");
        Timer::after(Duration::from_secs(1)).await;
    }
}

// Sensor task - stops feeding after 15 seconds to trigger watchdog timeout
#[embassy_executor::task]
async fn sensor_task(w: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
    // Register with 5 second timeout
    w.register_task(&TaskId::SensorTask, Duration::from_secs(5))
        .await;

    // Feed for 15 seconds
    let mut fed_count = 0;
    loop {
        w.feed(&TaskId::SensorTask).await;
        defmt::info!("SensorTask: fed (count: {})", fed_count);
        fed_count += 1;

        if fed_count >= 15 {
            defmt::warn!("SensorTask: stopping watchdog feed (will timeout in 5s)");
            break;
        }

        Timer::after(Duration::from_secs(1)).await;
    }

    // Task stops responding - watchdog will detect and trigger reset
    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}
