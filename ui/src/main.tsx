import "@fontsource-variable/inter";
import "./styles/glass.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { isTauri } from "./lib/env";

if (isTauri()) document.documentElement.classList.add("is-tauri");

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

// A janela nasce escondida para não piscar branco; aparece depois do primeiro quadro.
if (isTauri()) requestAnimationFrame(() => void getCurrentWindow().show());
