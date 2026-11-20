use j1939_core::swap_field_bytes;

#[test]
fn swaps_whole_bytes() {
    assert_eq!(swap_field_bytes(0x1234, 16), 0x3412);
    assert_eq!(swap_field_bytes(0x00AB_CDEF, 24), 0x00EF_CDAB);
    assert_eq!(
        swap_field_bytes(0x0102_0304_0506_0708, 64),
        0x0807_0605_0403_0201
    );
    assert_eq!(swap_field_bytes(0xAB, 8), 0xAB);
}

#[test]
fn swap_is_an_involution() {
    for bits in [8usize, 16, 24, 32, 40, 48, 56, 64] {
        let v = 0x0123_4567_89AB_CDEFu64 & (u64::MAX >> (64 - bits));
        assert_eq!(
            swap_field_bytes(swap_field_bytes(v, bits), bits),
            v,
            "{bits}"
        );
    }
}

#[test]
fn only_low_bytes_are_swapped() {
    assert_eq!(swap_field_bytes(0xFF00_1234, 16), 0xFF00_3412);
}
