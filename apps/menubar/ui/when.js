// Moments in local time. The daemon writes a moment in UTC, as
// 2026-11-10T08:12:44.000Z; the popover and the window show it in the
// user's own time and form.

// A moment written by the daemon, as a date and a time.
const MOMENT = /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})/g;

// One moment in local time, or the text as it came when it is no moment.
export function when(text) {
  const moment = new Date(text);
  if (Number.isNaN(moment.getTime())) {
    return text;
  }
  return moment.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

// A sentence with every moment in it in local time, such as the all-clear
// sentence "All clear, last checked <moment>.".
export function inLocalTime(sentence) {
  return sentence.replace(MOMENT, when);
}
