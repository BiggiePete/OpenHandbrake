#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Input, Pull};
use embassy_stm32::usb::Driver;
use embassy_stm32::{Config, bind_interrupts, peripherals, usb};
use embassy_time::Timer;
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler, State};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Handler};
use usbd_hid::descriptor::{SerializedDescriptor, generator_prelude::*};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USB_LP_CAN1_RX0 => usb::InterruptHandler<peripherals::USB>;
});

// HID Joystick Report Descriptor
#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = JOYSTICK) = {
        (collection = PHYSICAL, usage = POINTER) = {
            // X and Y axes
            (usage_page = GENERIC_DESKTOP,) = {
                (usage = X,) = {
                    #[item_settings data,variable,absolute] x=input;
                };
                (usage = Y,) = {
                    #[item_settings data,variable,absolute] y=input;
                };
            };
            // 4 Buttons
            (usage_page = BUTTON, usage_min = BUTTON_1, usage_max = BUTTON_4) = {
                #[packed_bits 4] #[item_settings data,variable,absolute] buttons=input;
            };
        };
    }
)]
#[allow(dead_code)]
pub struct JoystickReport {
    pub x: i8,
    pub y: i8,
    pub buttons: u8,
}

struct MyDeviceHandler {}

impl Handler for MyDeviceHandler {
    fn enabled(&mut self, enabled: bool) {
        if enabled {
            info!("Device enabled");
        } else {
            info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        info!("Bus reset");
    }

    fn addressed(&mut self, addr: u8) {
        info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        if configured {
            info!("Device configured");
        } else {
            info!("Device deconfigured");
        }
    }
}

struct MyRequestHandler {}

impl RequestHandler for MyRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        info!("Set report for {:?}: {=[u8]}", id, data);
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        info!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        info!("Get idle rate for {:?}", id);
        None
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting USB HID Joystick");

    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: embassy_stm32::time::Hertz(8_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll = Some(Pll {
            src: PllSource::HSE,
            prediv: PllPreDiv::DIV1,
            mul: PllMul::MUL9,
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV2;
        config.rcc.apb2_pre = APBPrescaler::DIV1;
    }

    let p = embassy_stm32::init(config);

    // Configure button pins (adjust these to your actual pin connections)
    let button1 = Input::new(p.PA0, Pull::Up);
    let button2 = Input::new(p.PA1, Pull::Up);
    let button3 = Input::new(p.PA2, Pull::Up);
    let button4 = Input::new(p.PA3, Pull::Up);

    // Create USB driver
    let driver = Driver::new(p.USB, Irqs, p.PA12, p.PA11);

    // Create embassy-usb Config
    let mut usb_config = embassy_usb::Config::new(0x1209, 0x0001);
    usb_config.manufacturer = Some("Embassy");
    usb_config.product = Some("USB Joystick");
    usb_config.serial_number = Some("12345678");
    usb_config.max_power = 100;
    usb_config.max_packet_size_0 = 64;

    // Required for HID
    usb_config.device_class = 0x00;
    usb_config.device_sub_class = 0x00;
    usb_config.device_protocol = 0x00;
    usb_config.composite_with_iads = false;

    // Create embassy-usb DeviceBuilder
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        usb_config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [], // no msos descriptors
        &mut control_buf,
    );

    // let mut device_handler = MyDeviceHandler {};
    // builder.handler(&mut device_handler);
    // Create HID class
    let hid_config = embassy_usb::class::hid::Config {
        report_descriptor: JoystickReport::desc(),
        request_handler: None,
        poll_ms: 10,
        max_packet_size: 64,
    };

    let mut hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, &mut state, hid_config);

    // Build USB device
    let mut usb = builder.build();

    // Run USB device
    let usb_fut = usb.run();

    // HID report loop
    let hid_fut = async {
        let mut report = JoystickReport {
            x: 0,
            y: 0,
            buttons: 0,
        };

        loop {
            // Read button states (active low with pull-up)
            let mut buttons = 0u8;
            if !button1.is_high() {
                buttons |= 0x01;
            }
            if !button2.is_high() {
                buttons |= 0x02;
            }
            if !button3.is_high() {
                buttons |= 0x04;
            }
            if !button4.is_high() {
                buttons |= 0x08;
            }

            // Update report
            report.buttons = buttons;

            // For demonstration, you can add joystick movement here
            // report.x and report.y would be read from actual analog inputs

            // Send report
            let report_bytes = [report.x as u8, report.y as u8, report.buttons];

            match hid.write(&report_bytes).await {
                Ok(()) => {}
                Err(e) => warn!("Failed to send report: {:?}", e),
            }

            Timer::after_millis(10).await;
        }
    };

    // Run both futures concurrently
    embassy_futures::join::join(usb_fut, hid_fut).await;
}
