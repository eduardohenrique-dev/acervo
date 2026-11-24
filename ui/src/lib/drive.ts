import type { DriveInfo } from "./api";

/** "Disco local (C:)" ou "Jogos (D:)". */
export const driveName = (info: DriveInfo) => `${info.label.trim() || "Disco local"} (${info.letter}:)`;
