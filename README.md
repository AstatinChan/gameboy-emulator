# Astatin Emulator

Hii !! This is the emulator I use to make [BunnyLand (Temporary name)](https://git.astatin.live/bunny-game.git/about/) on [stream on Twitch](https://links.astatin.live/twitch)

# Building the emulator

You need to put the bootrom you want the emulator to use in `assets/dmg_boot.bin`. There are a few bootroms you can find on the internet.

If you want to use the Astatin logo bootrom the source is provided in `assets/Astatin-bootrom.gbasm`. 

To assemble it yourself:
```bash
# Downloading directly the executable of the assembler (For linux x86_64)
# Alternatively you can follow the instructions here:
#   https://git.astatin.live/gameboy-asm.git/about/
# to compile it yourself
wget https://pellets.astatin.live/pkgs/gameboy-asm/latest/gbasm_linux-x86_64
chmod +x gbasm_linux-x86_64

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

If there is the message `No gamepad found` in the first messages, it means your gamepad hasn't been detected or initialized properly. Connect a gamepad to fix it. It should print `Found Gamepad id: GamepadId(0)` instead.

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

Serial communication can be used through tcp using the -L and -c arguments.  
One gameboy will use the -L to listen for connection on a port and the other will connect to the first one using the ip address and port of the first one.

If the two emulators are on the same machine, two linux fifo files can also be used with --fifo-input and --fifo-output.  
The files must be created before and the input fifo file of one must be the output of the other.

# Contributing

This emulator is not the fastest one, the most accurate one or the most well made. I'm not even sure in which environment it works (I never tested it on windows). I just made it because it's fun and it's a good way to learn how the gameboy works in detail.

If you find a bug or want to change something, I would be more confortable if you talked about it with me [on stream](https://links.astatin.live/twitch) or [on discord](https://links.astatin.live/discord) instead of sending a patch/opening a pull request directly.
