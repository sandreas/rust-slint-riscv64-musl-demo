# rust-slint-riscv64-musl-demo
A demo for rust slint on riscv64 musl

# How to build for LicheeRV Nano

IMPORTANT: Currently, the build requires a custom docker image  `ghcr.io/cross-rs/riscv64gc-unknown-linux-musl:local`, 
which has to be build manually - there is no working official docker image to pull directly:

```
git clone https://github.com/sandreas/cross.git
cd cross
./build-docker.sh
```


 This takes a while. After building successfully, you should have the required image on your machine:

```
> docker images                                                                                                                                                                         🕰 32m36s766ms  | 15:00:37
REPOSITORY                                      TAG                                            IMAGE ID       CREATED          SIZE
ghcr.io/cross-rs/riscv64gc-unknown-linux-musl   local                                          8229b26cdce6   58 minutes ago   3.06GB
```


Does not work:
./riscv64-unknown-linux-musl/riscv64-unknown-linux-musl/sysroot/usr/lib/libudev.so.1

Does work:
./riscv64-unknown-linux-musl/riscv64-unknown-linux-musl/sysroot/usr/lib/python3.12/site-packages/pyudev/_ctypeslib/libudev.py
./riscv64-unknown-linux-musl/riscv64-unknown-linux-musl/sysroot/usr/lib/python3.12/site-packages/pyudev/_ctypeslib/__pycache__/libudev.cpython-312.pyc
./riscv64-unknown-linux-musl/riscv64-unknown-linux-musl/sysroot/usr/lib/libudev.so
./riscv64-unknown-linux-musl/riscv64-unknown-linux-musl/sysroot/usr/lib/libudev.so.1.6.3
./riscv64-unknown-linux-musl/riscv64-unknown-linux-musl/sysroot/usr/lib/libudev.so.1