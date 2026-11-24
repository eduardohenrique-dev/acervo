import { describe, it, expect } from "vitest";
import { drivesReducer } from "./driveState";
import type { DriveInfo, DriveSummary } from "../lib/api";

const infoC: DriveInfo = { letter: "C", label: "", filesystem: "NTFS", total: 1000, free: 300 };
const summaryC: DriveSummary = {
  drive: "C",
  totalBytes: 1000,
  freeBytes: 300,
  usedBytes: 700,
  byCategory: [700, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  catalogedBytes: 700,
  mode: "mft",
};

describe("drivesReducer", () => {
  it("progresso → pronto → falha", () => {
    let s = drivesReducer({}, { type: "loaded", views: [{ info: infoC, status: { state: "scanning", fraction: 0, mode: "mft" } }] });
    s = drivesReducer(s, { type: "progress", drive: "C", fraction: 0.42, mode: "mft" });
    expect(s.C.view.status).toEqual({ state: "scanning", fraction: 0.42, mode: "mft" });
    s = drivesReducer(s, { type: "ready", drive: "C", summary: summaryC, live: false });
    expect(s.C.view.status.state).toBe("ready");
    expect(s.C.summary).toBe(summaryC);
    s = drivesReducer(s, { type: "failed", drive: "C", message: "Falha no disco C:" });
    expect(s.C.view.status).toEqual({ state: "failed", message: "Falha no disco C:" });
  });

  it("ignora evento de disco desconhecido", () => {
    expect(drivesReducer({}, { type: "progress", drive: "Z", fraction: 0.1, mode: "walk" })).toEqual({});
  });

  it("loaded mantém o resumo já carregado", () => {
    let s = drivesReducer({}, { type: "loaded", views: [{ info: infoC, status: { state: "ready", mode: "mft", live: false } }] });
    s = drivesReducer(s, { type: "ready", drive: "C", summary: summaryC, live: false });
    s = drivesReducer(s, { type: "loaded", views: [{ info: infoC, status: { state: "ready", mode: "mft", live: false } }] });
    expect(s.C.summary).toBe(summaryC);
  });

  // ---- Fase 2 ----

  it("ready guarda o live", () => {
    let s = drivesReducer({}, { type: "loaded", views: [{ info: infoC, status: { state: "scanning", fraction: 0, mode: "mft" } }] });
    s = drivesReducer(s, { type: "ready", drive: "C", summary: summaryC, live: true });
    expect(s.C.view.status).toEqual({ state: "ready", mode: "mft", live: true });
  });

  it("updated troca o resumo e mantém o status", () => {
    let s = drivesReducer({}, { type: "loaded", views: [{ info: infoC, status: { state: "ready", mode: "mft", live: true } }] });
    const novo = { ...summaryC, usedBytes: 800 };
    s = drivesReducer(s, { type: "updated", drive: "C", summary: novo });
    expect(s.C.summary).toBe(novo);
    expect(s.C.view.status).toEqual({ state: "ready", mode: "mft", live: true });
  });

  it("updated de disco em análise é ignorado", () => {
    const s0 = drivesReducer({}, { type: "loaded", views: [{ info: infoC, status: { state: "scanning", fraction: 0.5, mode: "mft" } }] });
    expect(drivesReducer(s0, { type: "updated", drive: "C", summary: summaryC })).toBe(s0);
  });
});
