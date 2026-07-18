use std::env;
extern crate memcrs;

cfg_if::cfg_if! {
    if #[cfg(feature = "rust-alloc")] {
      // uses the default Rust allocator
    } else if #[cfg(feature = "tikv-alloc")] {
        use tikv_jemallocator::Jemalloc;
        #[global_allocator]
        static GLOBAL: Jemalloc = Jemalloc;
    } else if #[cfg(feature = "system-alloc")] {
        use std::alloc::System;
        #[global_allocator]
        static GLOBAL: System = System;
    } else {
        // uses the default Rust allocator
    }
}

fn main() {
    memcrs::server::main::run(env::args().collect());
}
