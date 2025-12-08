//! Adapted from [`embassy_rp::clocks::dormant_sleep()`] and
//! pico-extras src/rp2_common/pico_sleep/sleep.c
//! pico-sdk src/rp2350/hardware_regs/include/hardware/regs/rosc.h
//!
//! https://ghubcoder.github.io/posts/awaking-the-pico/
//!
//! https://developer.arm.com/documentation/dui0662/b/The-Cortex-M0--Processor/Power-management?lang=en

use core::{arch::asm, time::Duration};

use chrono::TimeDelta;
use cortex_m::peripheral::SCB;
use embassy_rp::{
    pac::{
        self,
        common::{RW, Reg},
    },
    peripherals::RTC,
    rtc::{DateTime, DateTimeFilter, Rtc},
};

// TODO: determine math to run RTC at correct speed
const XOSC_RTC_DIVISOR_MODIFIER: u32 = 5;

/// Helper for restoring a register's contents on Drop.
struct Set<T: Copy, F: Fn()> {
    /// Register to modify.
    register: Reg<T, RW>,
    /// The old value of the register, written on drop.
    old: T,
    /// Function to run after writing `old` on drop.
    after_restore: F,
}

impl<T: Copy, F: Fn()> Drop for Set<T, F> {
    fn drop(&mut self) {
        self.register.write_value(self.old);
        (self.after_restore)();
    }
}

/// Set a register, restore it on drop, then run the given closure.
///
/// `f` should set the register and return a closure to be run after restore.
fn temporary_set_with_post_restore<T: Copy, After: Fn(), F: FnOnce(&mut T) -> After>(
    register: Reg<T, RW>,
    func: F,
) -> Set<T, impl Fn()> {
    register.modify(|w| {
        // save the old value
        let old = *w;
        // bind closure
        let after_restore = func(w);
        Set {
            register,
            old,
            after_restore,
        }
    })
}

/// Set a register and restore it on drop.
///
/// `f` should set the register.
fn temporary_set<T: Copy, F: FnOnce(&mut T)>(register: Reg<T, RW>, func: F) -> Set<T, impl Fn()> {
    temporary_set_with_post_restore(register, |r| {
        func(r);
        || () // do nothing after writing old value
    })
}

// TODO
fn duration_to_filter(rtc_now: DateTime, duration: Duration) -> DateTimeFilter {
    const SECS_PER_MIN: i64 = 60;
    const SECS_PER_HOUR: i64 = SECS_PER_MIN * 60;
    const SECS_PER_DAY: i64 = SECS_PER_HOUR * 24;

    let target_utc = chrono::DateTime::from_timestamp_secs(
        rtc_now.day as i64 * SECS_PER_DAY
            + rtc_now.hour as i64 * SECS_PER_HOUR
            + rtc_now.minute as i64 * SECS_PER_MIN
            + rtc_now.second as i64,
    )
    .unwrap()
        + chrono::TimeDelta::from_std(duration).unwrap();
    let mut rtc_wake_filter = DateTimeFilter::default();

    let target = target_utc - chrono::DateTime::from_timestamp_secs(0).unwrap();

    // TODO: create rtc_wake_filter from target

    todo!()
}

/// Makes most chip components go into deep sleep for `duration`.
pub(crate) fn deep_sleep_xosc(scb: &mut SCB, duration: Duration, rtc: &mut Rtc<'_, RTC>) {
    let filter = duration_to_filter(rtc.now().unwrap(), duration);

    // TODO: set IRQs manually
    rtc.schedule_alarm(filter);

    {
        // SETUP XOSC
        let _set_xosc = temporary_set_with_post_restore(pac::XOSC.ctrl(), |w| {
            w.set_enable(pac::xosc::vals::Enable::ENABLE);
            // w.set_freq_range(pac::xosc::vals::CtrlFreqRange::_1_15MHZ);
            // TODO: set freq range???
            while !pac::XOSC.status().read().stable() {}

            || while !pac::XOSC.status().read().stable() {}
        });

        // use xosc as clck ref
        let _switch_clk_ref = temporary_set(pac::CLOCKS.clk_ref_ctrl(), |w| {
            w.set_src(pac::clocks::vals::ClkRefCtrlSrc::XOSC_CLKSRC);
        });

        // // set clk_ref divisor for use with xosc???
        // let _set_clk_ref_freq = temporary_set(pac::CLOCKS.clk_ref_div(), |w| {
        //     // TODO: ?????
        //     // w.set_int(5);
        // });

        // set rtc divisor??
        let _set_rtc_div = temporary_set(pac::CLOCKS.clk_rtc_div(), |w| {
            // TODO
            w.set_int(w.int() / XOSC_RTC_DIVISOR_MODIFIER);
        });

        let _switch_clk_sys = temporary_set(pac::CLOCKS.clk_sys_ctrl(), |w| {
            w.set_src(pac::clocks::vals::ClkSysCtrlSrc::CLK_REF);
        });

        // disable unused clocks
        let _stop_adc = temporary_set(pac::CLOCKS.clk_adc_ctrl(), |w| w.set_enable(false));
        let _stop_usb = temporary_set(pac::CLOCKS.clk_usb_ctrl(), |w| w.set_enable(false));

        // use xosc to drive RTC
        let _switch_rtc = temporary_set(pac::CLOCKS.clk_rtc_ctrl(), |w| {
            w.set_auxsrc(pac::clocks::vals::ClkRtcCtrlAuxsrc::XOSC_CLKSRC);
        });

        // use system clock to drive peri
        let _switch_peri = temporary_set(pac::CLOCKS.clk_peri_ctrl(), |w| {
            w.set_auxsrc(pac::clocks::vals::ClkPeriCtrlAuxsrc::CLK_SYS);
        });

        // stop PLLs
        let _stop_pll_sys = temporary_set_with_post_restore(pac::PLL_SYS.pwr(), |w| {
            let wake = !w.pd() && !w.vcopd();
            w.set_pd(true);
            w.set_vcopd(true);
            move || while wake && !pac::PLL_SYS.cs().read().lock() {}
        });
        let _stop_pll_usb = temporary_set_with_post_restore(pac::PLL_USB.pwr(), |w| {
            let wake = !w.pd() && !w.vcopd();
            w.set_pd(true);
            w.set_vcopd(true);
            move || while wake && !pac::PLL_USB.cs().read().lock() {}
        });

        // disable rosc
        let _stop_rosc = temporary_set_with_post_restore(pac::ROSC.ctrl(), |w| {
            let wake = w.enable() == pac::rosc::vals::Enable::ENABLE;
            if wake {
                w.set_enable(pac::rosc::vals::Enable::DISABLE);
            }
            move || while wake && !pac::ROSC.status().read().stable() {}
        });

        // make rosc dormant too
        let _dormant_rosc = temporary_set(pac::ROSC.dormant(), |w| {
            w.set_dormant(pac::rosc::vals::Dormant::DORMANT);
        });

        // enable rtc in sleep mode
        let _sleep_en0 = temporary_set(pac::CLOCKS.sleep_en0(), |w| {
            w.set_clk_rtc_rtc(true);
        });
        let _sleep_en1 = temporary_set(pac::CLOCKS.sleep_en1(), |w| {
            w.0 = 0;
        });

        // power down xip cache
        let _power_down_xip_cache = temporary_set(pac::XIP_CTRL.ctrl(), |w| w.set_power_down(true));

        // make sure we enter deep sleep on `wfi`
        // TODO: adapt to use set(), need to make it generic over trait that allows for `.modify()`
        scb.set_sleepdeep();

        // power down memory
        let _power_down_mem = temporary_set(pac::SYSCFG.mempowerdown(), |w| {
            w.0 = 0x0;
        });

        // enter sleep
        unsafe { asm!("wfi") };
    }
    // helpers are dropped, states are re-set to initial values

    scb.clear_sleepdeep();

    rtc.disable_alarm();
}
