use j1939_macros::j1939_mux;

#[j1939_mux(selector = 0..8)]
enum Foo {
    #[j1939(selector = 1)]
    X { a: u8 },
}

fn main() {}
