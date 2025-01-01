# rs-V 🦀
RV32IM, machine privilege mode

## Build riscv-tests
Prerequisites:
`riscv-gnu-toolchain, autoconf`

RISCV environment variable is set to the RISC-V tools install path

```
$ git clone https://github.com/riscv/riscv-tests
$ cd riscv-tests
$ git submodule update --init --recursive
```
Edit riscv-tests/env/p/link.ld:

`. = 0x00000000;`
```
$ autoconf
$ ./configure --prefix=$RISCV/target
$ make
$ make install
```
