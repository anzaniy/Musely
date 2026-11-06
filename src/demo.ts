import { state } from "./state";
import { setHeader, setPlayIcon } from "./ui";

export function startDemo() {
  state.demoMode = true;
  setHeader("Creep", "Radiohead \u2014 Pablo Honey");
  state.demoLines = [
    { text: "When you were here before", startMs: 0, endMs: 4000 },
    { text: "Couldn't look you in the eye", startMs: 4000, endMs: 8000 },
    { text: "You're just like an angel", startMs: 8000, endMs: 12000 },
    { text: "Your skin makes me cry", startMs: 12000, endMs: 16000 },
    { text: "You float like a feather", startMs: 16000, endMs: 20000 },
    { text: "In a beautiful world", startMs: 20000, endMs: 24000 },
  ];
  state.demoStart = performance.now();
  state.lines = state.demoLines;
  state.lastIndex = -2;
  state.playing = true;
  setPlayIcon(true);
}