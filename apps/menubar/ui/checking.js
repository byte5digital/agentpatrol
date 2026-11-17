// The bar of a Full Scan that runs (spec sections 8.1 and 8.3), in the
// popover and on the Overview: the Items done of the total, or a bar that
// does not know how far yet while the Scan still looks for what to check.

export function renderBar(bar, done, total) {
  if (total > 0) {
    bar.max = total;
    bar.value = Math.min(done, total);
  } else {
    bar.removeAttribute("value");
  }
}
