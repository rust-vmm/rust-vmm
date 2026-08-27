# Generate Bindings

## Install Dependencies

```shell
$ cargo install bindgen
$ sudo apt install llvm-dev libclang-dev clang llvm
```

## Generate bindings

Generate the bindings with the following command:

```shell
./scripts/generate_binding.py -k {KERNEL_SOURCE_ROOT}
```

bindings.rs would be updated in mshv-bindings/src/
