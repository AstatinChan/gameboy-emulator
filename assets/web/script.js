import init, { Emulator } from "./emulator.js";

var emu = undefined;

export async function step_emulator(canvas) {
  await emu.run(canvas);
}

export async function start_emulator() {
  const gb_canvas = document.getElementById("gb");

  document.body.addEventListener("keydown", (event) => {
    const key_event = new KeyboardEvent("keydown", {
      key: event.key,
      keyCode: event.keyCode,
      code: event.code,
      which: event.which,
      shiftKey: event.shiftKey,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
    });
    gb_canvas.dispatchEvent(key_event);
  });

  document.body.addEventListener("keyup", (event) => {
    const key_event = new KeyboardEvent("keyup", {
      key: event.key,
      keyCode: event.keyCode,
      code: event.code,
      which: event.which,
      shiftKey: event.shiftKey,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
    });
    gb_canvas.dispatchEvent(key_event);
  });

  if (!emu) {
    await init();
    emu = new Emulator();
    emu.load_state();
  }

  return await step_emulator(gb_canvas);
}

module.start_emulator = start_emulator;
