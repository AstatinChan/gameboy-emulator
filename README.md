# Astatin Emulator

Hii !! This is the emulator I use to make [BunnyLand (Temporary name)](https://github.com/AstatinChan/BunnyLand-Gameboy) on [stream on Twitch](https://www.twitch.tv/astatinchan)

# Building the emulator

You need to put the bootrom you want the emulator to use in `assets/dmg_boot.bin`. There are a few bootroms you can find on the internet.

If you want to use the Astatin logo bootrom the source is provided in `assets/Astatin-bootrom.gbasm`. 

To assemble it yourself:
```bash
# Downloading directly the executable of the assembler (For linux x86_64)
# Alternatively you can follow the instructions here:
#   https://github.com/AstatinChan/gameboy-asm/blob/latest/README.md
# to compile it yourself
wget https://github.com/AstatinChan/gameboy-asm/releases/download/latest/gbasm_linux-x86_64
chmod +x gbasm_linux-x86-64

# Assembling the bootrom
./gbasm_linux-x86_64 assets/Astatin-bootrom.gbasm assets/dmg_boot.bin
```

After that you can build the emulator with
```bash
cargo build --release
```

The executable will be built in `target/release/emulator`

# Usage

The basic usage is just to provide a rom as a first argument:
```bash
emulator <gameboy_rom>
```

## Gamepad

If you do not set the `-k` cli parameter, the emulator will try to find a gamepad.

If there is the message `No gamepad found` in the first messages, it means your gamepad hasn't been detected or initialized properly. Connect a gamepad and restart the emulator to fix it. It should print `Found Gamepad id: GamepadId(0)` instead.

(I don't know which gamepad exactly. The 8BitDo SF30 Pro in USB mode on Arch Linux works. That's all I know lol)

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

## Serial communication

Serial communication is currently work in progress and doesn't work with basic ROMs but the `--fifo-input` and `--fifo-output` can already be tested by passing files created with mkfifo. If the goal is to allow communication between two gameboy, one gameboy's input should be the other's output.

# Contributing

This emulator is not the fastest one, the most accurate one or the most well made. I'm not even sure in which environment it works (I never tested it on windows). I just made it because it's fun and it's a good way to learn how the gameboy works in detail.

For this reason, I'm not entirely sure what I would do if someone were to open a PR without any previous discussion. If you find a bug or want to change something, I would be more confortable if you talked about it with me on stream or [on discord](https://discord.com/invite/XVTCuYJh) before.
