#![no_main]
#![no_std]

use panic_halt as _;

use core::borrow::{BorrowMut, Borrow};
use core::cell::RefCell;
use core::sync::atomic::{AtomicU8, Ordering};

use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use nb::block;

use msp432p401r as pac;
use msp432p401r_hal as hal;

use pac::interrupt;
use hal::{clock::*, flash::*, gpio::*, pcm::*, timer::*, watchdog::*};
use embedded_hal::digital::PinState;

// Truth Tables
const AND_TABLE: [bool; 4] = [false, false, false, true];
const OR_TABLE: [bool; 4] = [false, true, true, true];

const DEV_MAX: u8 = 2; // AND = 0, OR = 1, self test = 2
const MODE_MAX: u8 = 4; // 0 = test all, 1-4 = test specific gate

static CFG_BUTTON_1: Mutex<RefCell<Option<hal::gpio::porta::P1_1<Input<PullUp>>>>> = Mutex::new(RefCell::new(None)); // The right push button, to switch the device under test
static DEV_IND: AtomicU8 = AtomicU8::new(0); // Device Under test

static CFG_BUTTON_2: Mutex<RefCell<Option<hal::gpio::porta::P1_4<Input<PullUp>>>>> = Mutex::new(RefCell::new(None)); // The left push button, to switch test mode
static MODE_IND: AtomicU8 = AtomicU8::new(0); // Testig mode

// Gate 1 test peripherals
static GATE1_IN1: Mutex<RefCell<Option<hal::gpio::portc::P5_0<Output>>>> = Mutex::new(RefCell::new(None)); // Gate 1 Input 1
static GATE1_IN2: Mutex<RefCell<Option<hal::gpio::portc::P5_1<Output>>>> = Mutex::new(RefCell::new(None)); // Gate 1 Input 2
static GATE1_OUT1: Mutex<RefCell<Option<hal::gpio::portc::P5_2<Input<PullUp>>>>> = Mutex::new(RefCell::new(None)); // Gate 1 Output 1

// Gate 2 test peripheals
static GATE2_IN1: Mutex<RefCell<Option<hal::gpio::portc::P5_4<Output>>>> = Mutex::new(RefCell::new(None)); // Diddo
static GATE2_IN2: Mutex<RefCell<Option<hal::gpio::portc::P5_5<Output>>>> = Mutex::new(RefCell::new(None));
static GATE2_OUT1: Mutex<RefCell<Option<hal::gpio::portc::P5_6<Input<PullUp>>>>> = Mutex::new(RefCell::new(None));

// Gate 3 test peripherals
static GATE3_IN1: Mutex<RefCell<Option<hal::gpio::portc::P6_0<Output>>>> = Mutex::new(RefCell::new(None));
static GATE3_IN2: Mutex<RefCell<Option<hal::gpio::portc::P6_1<Output>>>> = Mutex::new(RefCell::new(None));
static GATE3_OUT1: Mutex<RefCell<Option<hal::gpio::portc::P6_4<Input<PullUp>>>>> = Mutex::new(RefCell::new(None));

// Gate 4 test peripherals
static GATE4_IN1: Mutex<RefCell<Option<hal::gpio::portc::P6_5<Output>>>> = Mutex::new(RefCell::new(None));
static GATE4_IN2: Mutex<RefCell<Option<hal::gpio::portc::P6_6<Output>>>> = Mutex::new(RefCell::new(None));
static GATE4_OUT1: Mutex<RefCell<Option<hal::gpio::portc::P6_7<Input<PullUp>>>>> = Mutex::new(RefCell::new(None));


fn self_test() -> [bool; 4] {
    let mut returntable: [bool; 4] = [false, false, false, false];
    cortex_m::interrupt::free(|cs| {
        let mut pin1 = GATE1_IN1.borrow(cs).borrow_mut();
        let mut pin2 = GATE1_IN2.borrow(cs).borrow_mut();
        let pout1 = GATE1_OUT1.borrow(cs).borrow();
        // 00 test
        pin1.as_mut().unwrap().set_low();
        pin2.as_mut().unwrap().set_low();
        returntable[0] = pout1.as_ref().unwrap().is_high().unwrap();
        // 01 test
        pin1.as_mut().unwrap().set_high();
        pin2.as_mut().unwrap().set_low();
        returntable[1] = pout1.as_ref().unwrap().is_high().unwrap();
        // 10 test
        pin1.as_mut().unwrap().set_low();
        pin2.as_mut().unwrap().set_high();
        returntable[2] = pout1.as_ref().unwrap().is_high().unwrap();
        // 11 test
        pin1.as_mut().unwrap().set_high();
        pin2.as_mut().unwrap().set_high();
        returntable[3] = pout1.as_ref().unwrap().is_high().unwrap();
    });
    return returntable;
}
fn test_1 (truthtable: &[bool; 4]) -> bool {
    let mut outtable: [bool; 4] = [false, false, false, false];
    cortex_m::interrupt::free(|cs| { // A Crtical Section
        // get IO from mutexes
        let mut pin1 = GATE1_IN1.borrow(cs).borrow_mut();
        let mut pin2 = GATE1_IN2.borrow(cs).borrow_mut();
        let pout1 = GATE1_OUT1.borrow(cs).borrow();
        // 00 test
        pin1.as_mut().unwrap().set_low();
        pin2.as_mut().unwrap().set_low();
        outtable[0] = pout1.as_ref().unwrap().is_high().unwrap();
        // 01 test
        pin1.as_mut().unwrap().set_high();
        pin2.as_mut().unwrap().set_low();
        outtable[1] = pout1.as_ref().unwrap().is_high().unwrap();
        // 10 test
        pin1.as_mut().unwrap().set_low();
        pin2.as_mut().unwrap().set_high();
        outtable[2] = pout1.as_ref().unwrap().is_high().unwrap();
        // 11 test
        pin1.as_mut().unwrap().set_high();
        pin2.as_mut().unwrap().set_high();
        outtable[3] = pout1.as_ref().unwrap().is_high().unwrap();
    });
    return outtable == *truthtable;
}

fn test_2 (truthtable: &[bool; 4]) -> bool {
    let mut outtable: [bool;4] = [false, false, false, false];
    cortex_m::interrupt::free(|cs| {
        // get IO from mutexes
        let mut pin1 = GATE2_IN1.borrow(cs).borrow_mut();
        let mut pin2 = GATE2_IN2.borrow(cs).borrow_mut();
        let pout1 = GATE2_OUT1.borrow(cs).borrow_mut();

        pin1.as_mut().unwrap().set_low();
        pin2.as_mut().unwrap().set_low();
        outtable[0] = pout1.as_ref().unwrap().is_high().unwrap();

        pin1.as_mut().unwrap().set_high();
        pin2.as_mut().unwrap().set_low();
        outtable[1] = pout1.as_ref().unwrap().is_high().unwrap();

        pin1.as_mut().unwrap().set_low();
        pin2.as_mut().unwrap().set_high();
        outtable[2] = pout1.as_ref().unwrap().is_high().unwrap();

        pin1.as_mut().unwrap().set_high();
        pin2.as_mut().unwrap().set_high();
        outtable[3] = pout1.as_ref().unwrap().is_high().unwrap();
    });
    return outtable == *truthtable;
}

fn test_3 (truthtable: &[bool; 4]) -> bool {
    let mut outtable: [bool; 4] = [false, false, false, false];
    cortex_m::interrupt::free(|cs| {
        let mut pin1 = GATE3_IN1.borrow(cs).borrow_mut();
        let mut pin2 = GATE3_IN2.borrow(cs).borrow_mut();
        let pout1 = GATE3_OUT1.borrow(cs).borrow_mut();

        pin1.as_mut().unwrap().set_low();
        pin2.as_mut().unwrap().set_low();
        outtable[0] = pout1.as_ref().unwrap().is_high().unwrap();

        pin1.as_mut().unwrap().set_high();
        pin2.as_mut().unwrap().set_low();
        outtable[1] = pout1.as_ref().unwrap().is_high().unwrap();

        pin1.as_mut().unwrap().set_low();
        pin2.as_mut().unwrap().set_high();
        outtable[2] = pout1.as_ref().unwrap().is_high().unwrap();

        pin1.as_mut().unwrap().set_high();
        pin2.as_mut().unwrap().set_high();
        outtable[3] = pout1.as_ref().unwrap().is_high().unwrap();
    });
    return outtable == *truthtable;
}

fn test_4 (truthtable: &[bool; 4]) -> bool {
    let mut outtable: [bool; 4] = [false, false, false, false];
    cortex_m::interrupt::free(|cs| {
        let mut pin1 = GATE4_IN1.borrow(cs).borrow_mut();
        let mut pin2 = GATE4_IN2.borrow(cs).borrow_mut();
        let pout1 = GATE4_OUT1.borrow(cs).borrow_mut();

        pin1.as_mut().unwrap().set_low();
        pin2.as_mut().unwrap().set_low();
        outtable[0] = pout1.as_ref().unwrap().is_high().unwrap();

        pin1.as_mut().unwrap().set_high();
        pin2.as_mut().unwrap().set_low();
        outtable[1] = pout1.as_ref().unwrap().is_high().unwrap();

        pin1.as_mut().unwrap().set_low();
        pin2.as_mut().unwrap().set_high();
        outtable[2] = pout1.as_ref().unwrap().is_high().unwrap();

        pin1.as_mut().unwrap().set_high();
        pin2.as_mut().unwrap().set_high();
        outtable[3] = pout1.as_ref().unwrap().is_high().unwrap();
    });
    return outtable == *truthtable;
}

fn test_all (truthtable: &[bool; 4]) -> [bool; 5] {
    return [[true, true, true, true] == [test_1(truthtable), test_2(truthtable), test_3(truthtable), test_4(truthtable)], test_1(truthtable), test_2(truthtable), test_3(truthtable), test_4(truthtable)];
}

#[entry]
fn main() -> ! {

    // Take the peripherals for configuration
    let periph = pac::Peripherals::take().unwrap();

    // Watchdog config
    let mut _watchdog = periph.WDT_A.constrain();
    _watchdog.set_timer_interval(TimerInterval::At27);
    _watchdog.feed().unwrap();

    // PCM config
    let _pcm = periph.PCM.constrain()
        .set_vcore(VCoreSel::DcdcVcore1)
        .freeze();

    // Flash Config
    let _flash_control = periph.FLCTL.constrain()
        .set_waitstates(FlashWaitStates::_2)
        .freeze();

    // Set the clock
    let _clock = periph.CS.constrain()
        .mclk_dcosource_selection(DCOFrequency::_48MHz, MPrescaler::DIVM_1)
        .smclk_prescaler(SMPrescaler::DIVS_2)
        .freeze();

    hprintln!("AND/OR Gate Tester Mk.1 ");

    // Split GPIO
    let pgpio = periph.DIO.split();

    // Configure LEDs
    let mut srled = pgpio.p1_0.into_output();
    let mut rled = pgpio.p2_0.into_output();
    let mut gled = pgpio.p2_1.into_output();
    let mut bled = pgpio.p2_2.into_output();

    // Configure the Device Configuration Button
    let mut pushr = pgpio.p1_1.into_pull_up_input();
    pushr.trigger_on_edge(Edge::Falling);
    pushr.enable_interrupt();

    // Configure the Mode Configuration Button
    let mut pushl = pgpio.p1_4.into_pull_up_input();
    pushl.trigger_on_edge(Edge::Falling);
    pushl.enable_interrupt();

    // Initialise Gate Test Array 1
    let gate1_in1 = pgpio.p5_0.into_output();
    let gate1_in2 = pgpio.p5_1.into_output();
    let mut gate1_out1 = pgpio.p5_2.into_pull_up_input();
    gate1_out1.trigger_on_edge(Edge::Falling);

    // Initialise Gate Test Array
    // 2
    let gate2_in1 = pgpio.p5_4.into_output();
    let gate2_in2 = pgpio.p5_5.into_output();
    let mut gate2_out1 = pgpio.p5_6.into_pull_up_input();
    gate2_out1.trigger_on_edge(Edge::Falling);

    // Initialise Gate Test Array 3
    let gate3_in1 = pgpio.p6_0.into_output();
    let gate3_in2 = pgpio.p6_1.into_output();
    let mut gate3_out1 = pgpio.p6_4.into_pull_up_input();
    gate3_out1.trigger_on_edge(Edge::Falling);

    // Initialise Gate Test Array 4
    let gate4_in1 = pgpio.p6_5.into_output();
    let gate4_in2 = pgpio.p6_6.into_output();
    let mut gate4_out1 = pgpio.p6_7.into_pull_up_input();
    gate4_out1.trigger_on_edge(Edge::Falling);

    // External LED peripherals for test. we don't need to take these as mutexes
    let mut testall_led = pgpio.p4_1.into_output();
    let mut test1_led = pgpio.p4_2.into_output();
    let mut test2_led = pgpio.p4_3.into_output();
    let mut test3_led = pgpio.p4_4.into_output();
    let mut test4_led = pgpio.p4_5.into_output();


    unsafe {
        cortex_m::peripheral::NVIC::unmask(pac::interrupt::PORT1_IRQ);  // enable the port 1 interrupt
        cortex_m::interrupt::enable();
    }
    cortex_m::interrupt::free(|cs| { // Critical Section
        // In order to safely use the peripherals globally, we must place them in a static mutex
        CFG_BUTTON_1.borrow(cs).replace(Some(pushr));
        CFG_BUTTON_2.borrow(cs).replace(Some(pushl));
        // take the Gate Test Array 1
        GATE1_IN1.borrow(cs).replace(Some(gate1_in1));
        GATE1_IN2.borrow(cs).replace(Some(gate1_in2));
        GATE1_OUT1.borrow(cs).replace(Some(gate1_out1));
        // take the Gate Test Array 2
        GATE2_IN1.borrow(cs).replace(Some(gate2_in1));
        GATE2_IN2.borrow(cs).replace(Some(gate2_in2));
        GATE2_OUT1.borrow(cs).replace(Some(gate2_out1));
        // take the Gate Test Array 3
        GATE3_IN1.borrow(cs).replace(Some(gate3_in1));
        GATE3_IN2.borrow(cs).replace(Some(gate3_in2));
        GATE3_OUT1.borrow(cs).replace(Some(gate3_out1));
        // take the Gate Test Array 4
        GATE4_IN1.borrow(cs).replace(Some(gate4_in1));
        GATE4_IN2.borrow(cs).replace(Some(gate4_in2));
        GATE4_OUT1.borrow(cs).replace(Some(gate4_out1));
    });

    srled.set_low();
    rled.set_low();
    gled.set_low();
    bled.set_low();

    loop {
        let mut dev_table: [bool; 4] = [false, false, false, false];
        if DEV_IND.load(Ordering::Relaxed) == 0 {
            bled.set_low();
            rled.set_high();
            dev_table = OR_TABLE;
        } else if DEV_IND.load(Ordering::Relaxed) == 1 {
            rled.set_low();
            bled.set_high();
            dev_table = AND_TABLE;
        } else if DEV_IND.load(Ordering::Relaxed) == DEV_MAX {
            srled.set_low();
            gled.set_low();
            dev_table = self_test();
            rled.set_state(PinState::from(dev_table == OR_TABLE));
            bled.set_state(PinState::from(dev_table == AND_TABLE));
            hprintln!("OR Gate: {} | AND Gate: {}", dev_table == OR_TABLE, dev_table == AND_TABLE);
        }
        if DEV_IND.load(Ordering::Relaxed) != DEV_MAX {
            let mut testtable: [bool; 5] = [false, false, false, false, false];
            if MODE_IND.load(Ordering::Relaxed) == 0 {
                srled.set_high();
                testtable = test_all(&dev_table);
                gled.set_state(PinState::from(testtable[0]));
                hprintln!("Test result {:?}", testtable);
            } else if MODE_IND.load(Ordering::Relaxed) == 1 {
                srled.toggle();
                gled.set_state(PinState::from(test_1(&dev_table)));
                testtable[1] = test_1(&dev_table);
                hprintln!("Test result {}", test_1(&dev_table));
            } else if MODE_IND.load(Ordering::Relaxed) == 2 {
                srled.toggle();
                gled.set_state(PinState::from(test_2(&dev_table)));
                testtable[2] = test_2(&dev_table);
                hprintln!("Test result {}", test_1(&dev_table));
            } else if MODE_IND.load(Ordering::Relaxed) == 3 {
                srled.toggle();
                testtable[3] = test_3(&dev_table);
                hprintln!("Test result {}", test_3(&dev_table));
                gled.set_state(PinState::from(test_1(&dev_table)));
            } else if MODE_IND.load(Ordering::Relaxed) == 4 {
                srled.toggle();
                gled.set_state(PinState::from(test_4(&dev_table)));
                testtable[4] = test_4(&dev_table);
                hprintln!("Test result {}", test_4(&dev_table));
            }
            testall_led.set_state(PinState::from(testtable[0]));
            test1_led.set_state(PinState::from(testtable[1]));
            test2_led.set_state(PinState::from(testtable[2]));
            test3_led.set_state(PinState::from(testtable[3]));
            test4_led.set_state(PinState::from(testtable[4]));
        }
        _watchdog.feed().unwrap();
        continue;
    }
}

#[pac::interrupt]
fn PORT1_IRQ() {
    cortex_m::interrupt::free(|cs| {
        let mut button1 = CFG_BUTTON_1.borrow(cs).borrow_mut();
        let mut button2 = CFG_BUTTON_2.borrow(cs).borrow_mut();
        if button1.as_mut().unwrap().check_interrupt() {
            if DEV_IND.load(Ordering::Relaxed) == DEV_MAX {
                DEV_IND.store(0, Ordering::Relaxed);
            } else {
                DEV_IND.fetch_add(1, Ordering::Relaxed);
            }
            button1.as_mut().unwrap().clear_interrupt_pending_bit();
        }
        if button2.as_mut().unwrap().check_interrupt() {
            if MODE_IND.load(Ordering::Relaxed) == MODE_MAX {
                MODE_IND.store(0, Ordering::Relaxed);
            } else {
                MODE_IND.fetch_add(1, Ordering::Relaxed);
            }
            button2.as_mut().unwrap().clear_interrupt_pending_bit();
        }
    });
    hprintln!("Interrupt Triggered, DEV_IND = {}, MODE_IND = {}", DEV_IND.load(Ordering::Relaxed), MODE_IND.load(Ordering::Relaxed))
}
