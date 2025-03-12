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

If you do not set the `-k` cli parameter, the emulator will try to find a gamepad.

If there is the message `No gamepad found` in the first messages, it means your gamepad hasn't been detected or initialized properly. Connect a gamepad and restart the emulator to fix it. It should print `Found Gamepad id: GamepadId(0)` instead.

(I don't know which gamepad exactly. The 8BitDo SF30 Pro in USB mode on Arch Linux works. That's all I know lol)

## CPU Usage

## Speed

You can adjust the speed with the `-s` argument

This command makes it run at 2x speed
```bash
emulator <gameboy_rom> -s 2
```

## Keyboard

By default will be from a gamepad. Keyboard can be used by using the `-k` argument.

The keyboard keys are:
```
Left, Right, Up, Down => Directional arrow
Letter A and B => A and B button
Enter => Start
Backspace => Select
```

This command will force the use of the keyboard:
```bash
emulator <gameboy_rom> -k
```

## Timing issues

Timing used to be inaccurate using sleep syscalls. The problem should now be fixed but if the timing is inconsistant on your system you can try to use the `--loop-lock-timing` and see if it is better. Note that using this will set your CPU usage to 100%.

## Serial communication

Serial communication is currently work in progress and doesn't work with basic ROMs but the `--fifo-input` and `--fifo-output` can already be tested by passing files created with mkfifo. If the goal is to allow communication between two gameboy, one gameboy's input should be the other's output.

# Contributing

This emulator is not the fastest one, the most accurate one or the most well made. I'm not even sure in which environment it works (I never tested it on windows). I just made it because it's fun and it's a good way to learn how the gameboy works in detail.

For this reason, I'm not entirely sure what I would do if someone were to open a PR without any previous discussion. If you find a bug or want to change something, I would be more confortable if you talked about it with me on stream or [on discord](https://discord.com/invite/XVTCuYJh) before.
