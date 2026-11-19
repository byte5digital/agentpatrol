// The integrity and Protection banners (spec section 8.6), as the popover
// and the Overview render them. The app hands over the banner already
// worded for the Posture and the place, with each button's command; this
// decides nothing.

import { keep } from "./keep.js";
import { nextPress } from "./press.js";

const { invoke } = window.__TAURI__.core;

// The banners whose button gave way to its working line, each with the key
// of the buttons that were pressed.
const givenWay = new WeakMap();

// Fills the banner elements under `root` (each found by its class) with
// `banner`, or hides `root` when there is none. A click names the press and
// calls the button's command; from the click until the answer the button
// waits and the banner's line says it is working ("Starting AgentPatrol…" for
// Start it), then says what it answered, or why it failed. The buttons keep
// their nodes while the banner shows the same ones, so a render cannot
// swallow a click.
//
// A button that gives way (Turn on protection) is not greyed out: from the
// click the banner shows its line ("Starting protection…") and no button,
// for as long as it is the banner that was pressed. The app then shows
// "Starting protection…" with a bar in the banner's place until the daemon
// answers. The buttons come back when the press fails, when the banner
// turns to another one (such as "Not watching right now", which then shows
// its own buttons), or when it went away and comes again.
export function renderBanner(root, banner) {
  root.hidden = banner === null;
  if (banner === null) {
    givenWay.delete(root);
    return;
  }
  const part = (name) => root.querySelector(`.${name}`);
  root.dataset.tone = banner.tone;
  part("banner-title").textContent = banner.title;
  part("banner-body").textContent = banner.body;
  // The popover's integrity line is the title alone (spec section 8.1).
  part("banner-body").hidden = banner.body === "";
  part("banner-developer").textContent = banner.developer ?? "";
  part("banner-developer").hidden = banner.developer === null;
  const line = part("banner-line");
  const say = (text) => {
    line.textContent = text;
    line.hidden = text === "";
  };
  const buttons = part("banner-buttons");
  const key = JSON.stringify(banner.buttons);
  keep(buttons, key, () =>
    banner.buttons.map((button, index) => {
      const node = document.createElement("button");
      node.type = "button";
      node.textContent = button.label;
      if (index === 0) {
        node.className = "primary";
      }
      node.addEventListener("click", async () => {
        if (button.givesWay) {
          givenWay.set(root, key);
          buttons.hidden = true;
        } else {
          node.disabled = true;
        }
        say(button.working);
        try {
          const answer = await invoke(button.command, { press: nextPress() });
          say(typeof answer === "string" ? answer : "");
        } catch (why) {
          say(String(why));
          givenWay.delete(root);
          buttons.hidden = false;
        }
        node.disabled = false;
      });
      return node;
    }),
  );
  if (givenWay.has(root) && givenWay.get(root) !== key) {
    givenWay.delete(root);
    say("");
  }
  buttons.hidden = givenWay.has(root);
}
