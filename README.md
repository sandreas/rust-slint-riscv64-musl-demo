# rust-slint-riscv64-musl-demo
A demo for rust slint on riscv64 musl


# Prerequisites

## Hardware

- [Sipeed LicheeRV Nano](https://wiki.sipeed.com/hardware/en/lichee/RV_Nano/1_intro.html)
- [Sitronix(?) LHCM228TS003A 2.28" Touch Display](https://b2b.baidu.com/land?id=39559f991fdef58e6c72b9f770bae1d810)
- [Apple USB-C to 3.5 mm Headphone Jack Adapter](https://www.apple.com/shop/product/mw2q3am/a/usb-c-to-35-mm-headphone-jack-adapter)
- Optional: [TP4057 Battery charger board](https://makerselectronics.com/product/lithium-battery-charger-discharge-module-tp4057-lx-lbc3-type-c-usb-1a/)

To connect the USB-Audio Adapter AND power the device, you need to supply power to the VSYS and GND Pin, because you can
not use USB to power the device. 
CAUTION: You need to supply 5V, the LicheeRV Nano is pretty picky about to high voltages, so be careful.

## Firmware-Image

- [LicheeRV Nano Fork by scpcom](https://github.com/scpcom/LicheeSG-Nano-Build/) - download latest `licheervnano-dap_sd.img.xz` and flash via `xzcat licheervnano-e_sd.img.xz | sudo dd of=<your-device> bs=100M status=progress conv=fsync`
- Mount `boot` partition and create / edit the following files
  - `touch fb` - enables framebuffer
  - `echo "panel=st7701_lhcm228ts003a" > uEnv.txt` - enables the display
  - `rm usb.dev && touch usb.host` - enable USB host mode to support USB-Audio-Adapters
  - Create `wpa_supplicant.conf`
    ```
    ctrl_interface=/var/run/wpa_supplicant
    ap_scan=1
    network={
        ssid="<YOUR SSID>"
        psk="<YOUR PSK>"
    }
    ```


# How to build for LicheeRV Nano

First you need  `docker` to be installed on your system - the `cross` crate will take care of the rest.

After checking out the repository and using `cargo` to update all dependencies, it should be enough to execute

```bash
./build-cross.sh
```

The script `copy-lichee.sh` will `scp` the binary to `lichee:/root`, which can be easily modified to match your DNS configuration. After 
copying the binary over, it can simply be run via SSH.

**Important:** You need to ensure to have a USB-Audio Adapter connected (see [Prerequisites](#Prerequisites))

By typing `./rust-slint-riscv64-musl-demo` the demo should be visible on the screen.

# Development

The project development happens on Linux, because the LicheeRV also operates on Linux and has some hard dependencies 
(like `alsa` for audio and `framebuffer` for graphics). It might be possible to run it on other systems, but it is clearly
NOT RECOMMENDED to do so.

Using Linux you should easily be able to just run the project via

```bash
cargo run
```

That's it for now, more to come.