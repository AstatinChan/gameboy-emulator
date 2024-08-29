# Astatin Emulator

Hii !! This is the emulator I use to make [BunnyLand (Temporary name)](https://github.com/AstatinChan/BunnyLand-Gameboy) on [stream on Twitch](https://www.twitch.tv/astatinchan)

# Building the emulator

For reasons related to my Japanese visa, I will not distribute the boot roms in this repo, but I'm sure you know how to use the internet.

You need to put the dmg boot rom in the file `assets/dmg_boot.bin` and the cgb boot rom in the file `assets/cgb_boot.bin`.

When you have both the boot roms ready, build the emulator with `cargo build --release`. It will give you a binary in `target/release/emulator`.

# Usage

The basic usage is just to provide a rom as a first argument:
```bash
emulator <gameboy_rom>
```

## Gamepad

This emulator needs a gamepad to be connected to play. There is no keyboard option for now.

If there is the message `No gamepad found` in the first messages, it means your gamepad hasn't been detected or initialized properly. Connect a gamepad and restart the emulator to fix it. It should print `Found Gamepad id: GamepadId(0)` instead.

(I don't know which gamepad exactly. The 8BitDo SF30 Pro in USB mode on Arch Linux works. That's all I know lol)

## CPU Usage

By default the emulator will spin lock instead of using thread::sleep (Bc I'm bad at programming and for some reason I can't manage to get an accurate time using thread::sleep)

If you're on battery or the 100% CPU usage bothers you, you can use the --thread-sleep option, though it might cause some lags and inaccurate timing.

```bash
emulator <gameboy_rom> --thread-sleep
```

## Speed

You can adjust the speed with the `-s` argument

This command makes it run at 2x speed
```bash
emulator <gameboy_rom> -s 2
```

# Contributing

This emulator is not the fastest one, the most accurate one or the most well made. I'm not even sure in which environment it works (I never tested it on windows). I just made it because it's fun and it's a good way to learn how the gameboy works in detail.

For this reason, I'm not entirely sure what I would do if someone were to open a PR. If you find a bug or want to change something, I would be more confortable if you talked about it with me on stream or [on discord](https://discord.com/invite/XVTCuYJh) before.
