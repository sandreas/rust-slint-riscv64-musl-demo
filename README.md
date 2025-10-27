# rust-slint-riscv64-musl-demo
A demo for rust slint on riscv64 musl

# How to build for LicheeRV Nano

IMPORTANT: Currently, the build requires a custom docker image  `ghcr.io/cross-rs/riscv64gc-unknown-linux-musl:local`, 
which has to be build manually and requires a lot of steps. I'm working on replacing this with a `pre-build = []` step
in `Cross.toml`. Until that is done, you won't be able to build this project without the custom image.


- Download LicheeRV Nano release from scpcom: https://github.com/scpcom/LicheeSG-Nano-Build/releases/download/v2.2.9-22/licheervnano-e_sd.img.xz
- Extract the image to `licheervnano-e_sd.img`
- Mount the image
  - ```bash
    fdisk -lu licheervnano-e_sd.img
    # Device                             Boot Start     End Sectors   Size Kn Type
    # licheervnano-e_sd-2025-10-26.img1 *         1   32768    32768   16M  c W95 FAT3
    # licheervnano-e_sd-2025-10-26.img2       32769 3309568  3276800  1,6G 83 Linux
    
    # Now take 32769 * 512 = 16777728 to mount the device
    sudo mkdir /mnt/lichee
    sudo mount -o loop,offset=16777728 licheervnano-e_sd.img /mnt/lichee
    cd /mnt/lichee/
    tar -czvf /tmp/usr-lib.tar.gz usr/lib
    ```
    
- Work in progress - 