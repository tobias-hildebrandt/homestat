use blake2::digest::{Update, VariableOutput};
use embassy_rp::clocks::RoscRng;
use embassy_time::Instant;
use rand::SeedableRng;

/// Create a new [`SeedableRng`] from [`getrandom_v3`] entropy, see [`crate::getrandom_impl`].
pub(crate) fn new_rng<Rng: SeedableRng>() -> Rng {
    let mut seed: <Rng>::Seed = <Rng>::Seed::default();

    getrandom::fill(seed.as_mut()).unwrap();

    Rng::from_seed(seed)
}

// implement custom `getrandom` v3
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom::Error> {
    // initialize buffer, since it may be uninitialized
    let buf = unsafe {
        // fill buffer with zeros
        core::ptr::write_bytes(dest, 0, len);
        // create mutable byte slice
        core::slice::from_raw_parts_mut(dest, len)
    };

    fill_buffer(buf);

    Ok(())
}

/// Fill buffer with bytes generated with different sources of entropy.
fn fill_buffer(buffer: &mut [u8]) {
    // cryptographic hasher to combine sources of entropy
    let mut blake = blake2::Blake2bVar::new(buffer.len()).expect("unable to create blake2 hasher");

    // inject entropy from system clock
    blake.update(&Instant::now().as_ticks().to_le_bytes());

    // inject entropy from ROSC
    const NUM_BITS_FROM_ROSC: usize = 2usize.pow(12);
    for _ in 0..(NUM_BITS_FROM_ROSC / (u8::BITS as usize)) {
        blake.update(&[RoscRng::next_u8()]);
    }

    // TODO: inject entropy from ADCs and onboard temperature sensor

    blake
        .finalize_variable(buffer)
        .expect("unable to fill buffer from blake hash");
}
