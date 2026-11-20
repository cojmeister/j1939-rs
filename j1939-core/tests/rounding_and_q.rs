//! Bugs 2, 3, 6, 7: raw conversion must saturate to the field range, rounding must be explicit,
//! and Q formats must be generic and saturating.
use j1939_core::{Q, Q9, RoundingMode, UQ, f32_to_raw_signed, f32_to_raw_unsigned};

// ---- bug 2: scaled raw conversion ----
#[test]
fn rpm_6000_at_0_125_is_raw_48000_in_16_bits() {
    let raw = f32_to_raw_unsigned(6000.0 / 0.125, 16, RoundingMode::HalfAwayFromZero);
    assert_eq!(raw, 48000);
}

#[test]
fn unsigned_saturates_to_field_max_not_i16_max() {
    assert_eq!(
        f32_to_raw_unsigned(1.0e9, 16, RoundingMode::HalfAwayFromZero),
        65535
    );
    assert_eq!(
        f32_to_raw_unsigned(1.0e12, 32, RoundingMode::HalfAwayFromZero),
        u32::MAX as u64
    );
    assert_eq!(
        f32_to_raw_unsigned(-5.0, 16, RoundingMode::HalfAwayFromZero),
        0
    );
    assert_eq!(
        f32_to_raw_unsigned(f32::NAN, 16, RoundingMode::HalfAwayFromZero),
        0
    );
}

#[test]
fn signed_saturates_to_field_range() {
    let m = RoundingMode::HalfAwayFromZero;
    assert_eq!(f32_to_raw_signed(1.0e9, 16, m), 32767);
    assert_eq!(f32_to_raw_signed(-1.0e9, 16, m), -32768);
    assert_eq!(f32_to_raw_signed(-40000.0, 24, m), -40000); // below i16::MIN must NOT saturate
    assert_eq!(f32_to_raw_signed(200.0, 8, m), 127);
}

// ---- rounding policy ----
#[test]
fn rounding_modes_differ_on_fractions() {
    // 1.997 -> truncate 1, round 2 ; -1.997 -> -1 / -2 ; 2.5 -> 2 / 3 ; -2.5 -> -2 / -3
    assert_eq!(f32_to_raw_signed(1.997, 16, RoundingMode::Truncate), 1);
    assert_eq!(
        f32_to_raw_signed(1.997, 16, RoundingMode::HalfAwayFromZero),
        2
    );
    assert_eq!(f32_to_raw_signed(-1.997, 16, RoundingMode::Truncate), -1);
    assert_eq!(
        f32_to_raw_signed(-1.997, 16, RoundingMode::HalfAwayFromZero),
        -2
    );
    assert_eq!(
        f32_to_raw_signed(2.5, 16, RoundingMode::HalfAwayFromZero),
        3
    );
    assert_eq!(
        f32_to_raw_signed(-2.5, 16, RoundingMode::HalfAwayFromZero),
        -3
    );
}

#[test]
fn default_rounding_is_half_away_from_zero() {
    assert_eq!(RoundingMode::default(), RoundingMode::HalfAwayFromZero);
}

// ---- bug 6: Q9 saturation ----
#[test]
fn q9_plus_one_saturates_instead_of_wrapping() {
    assert_eq!(Q9::from_float(1.0).to_raw(), 511);
    assert_eq!(Q9::from_float(2.0).to_raw(), 511);
    assert_eq!(Q9::from_float(-1.0).to_raw(), -512);
    assert_eq!(Q9::from_float(-7.0).to_raw(), -512);
    assert!(Q9::from_float(1.0).to_float() > 0.99);
}

#[test]
fn q9_truncate_vs_round() {
    assert_eq!(
        Q9::from_float_with(0.0039, RoundingMode::Truncate).to_raw(),
        1
    );
    assert_eq!(
        Q9::from_float_with(0.0039, RoundingMode::HalfAwayFromZero).to_raw(),
        2
    );
    assert_eq!(Q9::from_float(0.0039).to_raw(), 2); // default rounds
}

// ---- bug 7: generic Q / UQ ----
#[test]
fn q7_boundaries() {
    assert_eq!(Q::<7>::BITS, 8);
    assert_eq!(Q::<7>::from_float(1.0).to_raw(), 127);
    assert_eq!(Q::<7>::from_float(-1.0).to_raw(), -128);
    assert_eq!(Q::<7>::from_float(0.5).to_raw(), 64);
    assert_eq!(Q::<7>::from_raw(-128).to_float(), -1.0);
}

#[test]
fn q15_boundaries() {
    assert_eq!(Q::<15>::BITS, 16);
    assert_eq!(Q::<15>::from_float(1.0).to_raw(), 32767);
    assert_eq!(Q::<15>::from_float(-1.0).to_raw(), -32768);
    assert_eq!(Q::<15>::from_float(0.5).to_raw(), 16384);
}

#[test]
fn q9_is_q_of_9() {
    let a: Q<9> = Q9::from_float(0.25);
    assert_eq!(a.to_raw(), 128);
    assert_eq!(Q9::BITS, 10);
}

#[test]
fn uq10_boundaries_use_2n_minus_1_denominator() {
    assert_eq!(UQ::<10>::BITS, 10);
    assert_eq!(UQ::<10>::from_float(1.0).to_raw(), 1023);
    assert_eq!(UQ::<10>::from_float(0.0).to_raw(), 0);
    assert_eq!(UQ::<10>::from_float(2.0).to_raw(), 1023);
    assert_eq!(UQ::<10>::from_float(-0.5).to_raw(), 0);
    assert_eq!(UQ::<10>::from_raw(1023).to_float(), 1.0);
    assert_eq!(UQ::<10>::from_float(0.5).to_raw(), 512); // 511.5 rounds away
    assert_eq!(
        UQ::<10>::from_float_with(0.5, RoundingMode::Truncate).to_raw(),
        511
    );
}

#[test]
fn uq8_full_scale() {
    assert_eq!(UQ::<8>::from_float(1.0).to_raw(), 255);
    assert_eq!(UQ::<8>::from_raw(255).to_float(), 1.0);
}

#[test]
fn ranged_q9_over_90_degrees() {
    // physical = q * range; encode: raw = q(physical / range)
    let range = 90.0f32;
    let raw = Q9::from_float(45.0 / range).to_raw();
    assert_eq!(raw, 256);
    assert_eq!(Q9::from_raw(raw).to_float() * range, 45.0);
    // saturates at +range
    assert_eq!(Q9::from_float(120.0 / range).to_raw(), 511);
}
