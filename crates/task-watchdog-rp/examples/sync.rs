#![no_std]
#![no_main]

use defmt::{info, unwrap};
use embassy_time::{Duration, Instant};
use panic_probe as _;
use task_watchdog_core::Id;
use task_watchdog_rp::rp_hal::{RpHalClock, RpHalWatchdog, Watchdog};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TaskId {
    MainTask,
    BlinkTask,
}
impl Id for TaskId {}

const NUM_TASKS: usize = 2;

#[cortex_m_rt::entry]
fn main() -> ! {
    info!("Starting task-watchdog RP2040 synchronous example");

    // Initialize hardware
    let mut pac = rp2040_hal::pac::Peripherals::take().unwrap();
    let core = rp2040_hal::pac::CorePeripherals::take().unwrap();
    let mut watchdog = rp2040_hal::watchdog::Watchdog::new(pac.WATCHDOG);
    let clocks = rp2040_hal::clocks::init_clocks_and_plls(
        rp2040_hal::xosc::setup_xosc_with_crystal(
            pac.XOSC,
            rp2040_hal::xosc::CrystalFreq::Freq12Mhz,
        )
        .map_err(|_| ())
        .unwrap(),
        pac.CLK,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // Initialize GPIO LED
    let sio = rp2040_hal::sio::Sio::new(pac.SIO);
    let pins = rp2040_hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let mut led_pin = pins.gpio25.into_push_pull_output();

    // Create task-watchdog
    let hw_watchdog = RpHalWatchdog::new(watchdog);
    let config = task_watchdog_core::WatchdogConfig {
        check_interval: Duration::from_millis(500),
        hardware_timeout: Duration::from_secs(5),
    };

    let mut watchdog = Watchdog::new(hw_watchdog, config, RpHalClock);

    // Register tasks
    watchdog.register_task(&TaskId::MainTask, Duration::from_secs(2));
    watchdog.register_task(&TaskId::BlinkTask, Duration::from_secs(3));

    info!("Watchdog configured with {} tasks", NUM_TASKS);

    // Start watchdog
    watchdog.start();

    // Main loop
    let mut last_main_feed = Instant::now();
    let mut last_blink = Instant::now();
    let mut blink_state = false;

    loop {
        // Check watchdog
        if watchdog.check() {
            info!("Task starved! Triggering reset.");
            watchdog.trigger_reset();
        }

        // Feed main task
        let now = Instant::now();
        if (now - last_main_feed).as_millis() >= 1000 {
            info!("Main task: feeding watchdog");
            watchdog.feed(&TaskId::MainTask);
            last_main_feed = now;
        }

        // Feed blink task and toggle LED
        if (now - last_blink).as_millis() >= 2000 {
            info!("Blink task: feeding watchdog");
            watchdog.feed(&TaskId::BlinkTask);
            blink_state = !blink_state;
            if blink_state {
                led_pin.set_high().unwrap();
            } else {
                led_pin.set_low().unwrap();
            }
            last_blink = now;
        }

        delay.delay_ms(100);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("Panic: {}", defmt::Display2Format(info));
    loop {}
}
