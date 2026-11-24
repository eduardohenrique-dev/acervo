/** Rodando dentro do app Tauri (e não num navegador de desenvolvimento). */
export const isTauri = (): boolean => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
