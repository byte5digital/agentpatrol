// The popover (spec section 8.1). It renders the view the app hands over and
// calls the app's commands; it decides nothing. It stays minimal: the orb,
// one sentence or the banner that replaces it, the last check and four
// buttons. While a Scan runs the sentence says "Checking now."; while a Full
// Scan runs it says "Checking…" over the bar the Overview shows.

import { renderBanner } from "./banner.js";
import { renderBar } from "./checking.js";
import { nextPress } from "./press.js";
import { inLocalTime } from "./when.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const root = document.querySelector(".popover");
const said = document.getElementById("said");
const sentence = document.getElementById("sentence");
const detail = document.getElementById("detail");
const checking = document.getElementById("checking");
const checkingLabel = document.getElementById("checking-label");
const checkingBar = document.getElementById("checking-bar");
const banner = document.getElementById("banner");
const lastCheck = document.getElementById("last-check");
const asking = document.getElementById("asking");
const openWindow = document.getElementById("open-window");
const scanNow = document.getElementById("scan-now");
const scanFolder = document.getElementById("scan-folder");
const scanLine = document.getElementById("scan-line");
const updateReady = document.getElementById("update-ready");
const quit = document.getElementById("quit");

function when(text) {
  const moment = new Date(text);
  if (Number.isNaN(moment.getTime())) {
    return text;
  }
  return moment.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

function render(view) {
  root.dataset.orb = view.orb;
  // A banner replaces the sentence and the line under it (spec section 8.6).
  said.hidden = view.sentence === null;
  sentence.textContent = inLocalTime(view.sentence ?? "");
  detail.textContent = view.detail;
  detail.hidden = view.detail === "";
  const scan = view.checking ?? null;
  checking.hidden = scan === null;
  if (scan !== null) {
    checkingLabel.textContent = scan.label;
    checkingLabel.hidden = scan.label === "";
    renderBar(checkingBar, scan.done, scan.total);
  }
  renderBanner(banner, view.banner);
  if (view.lastCheck) {
    lastCheck.textContent = `Last check: ${when(view.lastCheck)}`;
    lastCheck.hidden = false;
  } else {
    lastCheck.hidden = true;
  }
  // When no daemon answers the app shows its banner and offers nothing else
  // (spec section 6.2).
  asking.hidden = view.protectionOff;
  // "Update ready, restart AgentPatrol" once an update is downloaded and
  // verified; quitting installs it (spec section 9.4).
  updateReady.textContent = view.updateReady ?? "";
  updateReady.hidden = !view.updateReady;
}

function say(line) {
  scanLine.textContent = line;
  scanLine.hidden = line === "";
}

quit.addEventListener("click", () => invoke("quit"));
openWindow.addEventListener("click", () => invoke("open_window"));
scanNow.addEventListener("click", async () => {
  try {
    say(await invoke("scan_now", { press: nextPress() }));
  } catch (why) {
    say(String(why));
  }
});
scanFolder.addEventListener("click", () => invoke("scan_folder").catch((why) => say(String(why))));

await listen("agentpatrol://view", (event) => render(event.payload));
render(await invoke("popover"));
