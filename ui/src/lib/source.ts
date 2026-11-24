import * as api from "./api";
import { demo } from "./demo";
import { isTauri } from "./env";

/** No app usa o backend Rust; no navegador de desenvolvimento, a demonstração. */
export const source: typeof demo = isTauri() ? api : demo;
