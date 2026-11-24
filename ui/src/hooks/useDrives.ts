import { useCallback, useEffect, useReducer, useState } from "react";
import { source } from "../lib/source";
import { drivesReducer, type DriveEntry } from "./driveState";

export function useDrives(): { drives: DriveEntry[]; rescan: (drive: string) => void; elevated: boolean } {
  const [state, dispatch] = useReducer(drivesReducer, {});
  const [elevated, setElevated] = useState(true);

  useEffect(() => {
    let alive = true;
    const ready = async (drive: string, live: boolean) => {
      const summary = await source.driveSummary(drive);
      if (alive) dispatch({ type: "ready", drive, summary, live });
    };
    const updated = async (drive: string) => {
      const summary = await source.driveSummary(drive);
      if (alive) dispatch({ type: "updated", drive, summary });
    };
    const offs = [
      source.onIndexProgress((e) => alive && dispatch({ type: "progress", ...e })),
      source.onIndexReady((e) => void ready(e.drive, e.live)),
      source.onIndexUpdated((e) => void updated(e.drive)),
      source.onIndexFailed((e) => alive && dispatch({ type: "failed", ...e })),
    ];
    void source.appIsElevated().then((v) => alive && setElevated(v));
    void source.listDrives().then((views) => {
      if (!alive) return;
      dispatch({ type: "loaded", views });
      for (const v of views) if (v.status.state === "ready") void ready(v.info.letter, v.status.live);
    });
    return () => {
      alive = false;
      offs.forEach((p) => void p.then((off) => off()));
    };
  }, []);

  const rescan = useCallback((drive: string) => {
    dispatch({ type: "progress", drive, fraction: 0, mode: "mft" });
    void source.rescanDrive(drive);
  }, []);

  const drives = Object.values(state).sort((a, b) => a.view.info.letter.localeCompare(b.view.info.letter));
  return { drives, rescan, elevated };
}
