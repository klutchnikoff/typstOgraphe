import { createLigatureAnimation } from "./motion.mjs";

const animation = createLigatureAnimation(document.querySelector(".splash"));
window.typstographeSplash = animation;
const theme = new URLSearchParams(location.search).get("theme");
if (theme === "light" || theme === "dark")
  document.documentElement.dataset.theme = theme;
addEventListener("message", (event) => {
  if (event.origin !== location.origin || event.source !== parent) return;
  if (event.data?.theme === "light" || event.data?.theme === "dark") {
    document.documentElement.dataset.theme = event.data.theme;
  }
});
animation.play();
