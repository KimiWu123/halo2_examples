# halo2_examples
Some exapmles to show how to write a circuit in Halo2

## Commands

Compile 
```
cargo build
```


Run Tests
```
cargo test

// run a specific test
cargo test -- --nocapture <test case>
```

Generate a circuit layout
```
cargo test --all-features -- --nocapture <function name>

// for example
cargo test --all-features -- --nocapture plot_fibonacci1
```
