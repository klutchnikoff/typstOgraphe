// Shared by the approved preview and the application welcome screen.
export function createLigatureAnimation(root) {
  const camera = root.querySelector(".camera");
  const wordmark = root.querySelector(".wordmark");
  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
  const tracks = [];
  const duration = 2100;
  let started = false;

  function track(element, frames, timing) {
    const animation = element.animate(frames, {
      fill: "both",
      iterations: 1,
      ...timing,
    });
    tracks.push(animation);
  }

  function finish() {
    for (const animation of tracks) animation.finish();
    root.dataset.state = "settled";
  }

  function play() {
    if (started) return;
    started = true;
    if (reducedMotion.matches || !camera.animate) {
      finish();
      return;
    }
    root.dataset.state = "writing";
    const draw = (selector, delay, time) =>
      track(
        root.querySelector(selector),
        [{ strokeDashoffset: "100" }, { strokeDashoffset: "0" }],
        { duration: time, delay, easing: "cubic-bezier(.42,0,.25,1)" },
      );
    draw(".reveal-t", 80, 650);
    draw(".reveal-crossbar", 530, 320);
    draw(".reveal-o", 720, 1050);
    track(
      camera,
      [
        {
          transform: "translate(-7px, 9px) scale(1.14)",
          filter: "blur(2.5px)",
          opacity: 0.4,
        },
        {
          transform: "translate(-2px, 2px) scale(1.035)",
          filter: "blur(.5px)",
          opacity: 1,
          offset: 0.65,
        },
        {
          transform: "translate(0, 0) scale(1)",
          filter: "blur(0)",
          opacity: 1,
        },
      ],
      { duration, easing: "cubic-bezier(.2,.55,.3,1)" },
    );
    track(wordmark, [{ opacity: 0 }, { opacity: 1 }], {
      duration: 450,
      delay: 1550,
    });
    Promise.all(tracks.map((animation) => animation.finished))
      .then(finish)
      .catch(finish);
  }

  const onMotionChange = (event) => {
    if (event.matches) finish();
  };
  const onVisibilityChange = () => {
    if (document.hidden) finish();
  };
  reducedMotion.addEventListener("change", onMotionChange);
  document.addEventListener("visibilitychange", onVisibilityChange);
  return {
    duration,
    play,
    finish,
    seek(time) {
      for (const animation of tracks) {
        animation.pause();
        animation.currentTime = Math.max(0, Math.min(duration, time));
      }
    },
    dispose() {
      finish();
      for (const animation of tracks) animation.cancel();
      reducedMotion.removeEventListener("change", onMotionChange);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    },
  };
}
