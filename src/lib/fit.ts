// Display fonts differ a lot in width: "Comilito" in Cinzel Decorative (all capitals) is half as
// wide again as in UnifrakturMaguntia. A title sized for one theme can overflow its column in
// another. `use:fitText` shrinks a one-line title just enough to fit its width, and never
// makes it bigger than its CSS size.

/** Smallest size, as a share of the CSS size, so a very long title still stays readable. */
const MIN_SCALE = 0.5;

/**
 * A Svelte action (`<h1 use:fitText>`): Svelte calls it when the element is created, and
 * calls the returned `destroy` when it's removed. It re-fits when the element resizes, its
 * text changes, a font finishes loading, or the theme changes (which sets new CSS variables on
 * <html>).
 */
export function fitText(node: HTMLElement) {
  node.style.whiteSpace = "nowrap";

  function fit() {
    // Measure at the size the stylesheet asks for, then scale down if it overflows.
    node.style.fontSize = "";
    const wanted = parseFloat(getComputedStyle(node).fontSize);
    const available = node.clientWidth;
    const needed = node.scrollWidth;
    if (needed > available && available > 0) {
      const scale = Math.max(MIN_SCALE, available / needed);
      // A pixel less than the exact fit, so rounding never leaves it a hair too wide.
      node.style.fontSize = `${Math.floor(wanted * scale) - 1}px`;
    }
  }

  // Avoid fitting several times in one frame when several things change at once.
  let pending = 0;
  const schedule = () => {
    cancelAnimationFrame(pending);
    pending = requestAnimationFrame(fit);
  };

  const resized = new ResizeObserver(schedule);
  resized.observe(node);
  const textChanged = new MutationObserver(schedule);
  textChanged.observe(node, { childList: true, characterData: true, subtree: true });
  const themeChanged = new MutationObserver(schedule);
  themeChanged.observe(document.documentElement, { attributes: true, attributeFilter: ["style", "class"] });
  document.fonts.addEventListener("loadingdone", schedule);
  schedule();

  return {
    destroy() {
      cancelAnimationFrame(pending);
      resized.disconnect();
      textChanged.disconnect();
      themeChanged.disconnect();
      document.fonts.removeEventListener("loadingdone", schedule);
    },
  };
}
