import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { useState } from "react";
import { Sidebar, type Screen } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { useDrives } from "./hooks/useDrives";
import type { CategoryKey } from "./lib/categories";
import { Category } from "./screens/Category";
import { OperationToast } from "./components/OperationToast";
import { useOperations } from "./hooks/useOperations";
import { Files } from "./screens/Files";
import { Operations } from "./screens/Operations";
import { Overview } from "./screens/Overview";
import { System } from "./screens/System";

/** O que está na área principal. A tela Categoria fica "dentro" da Visão geral. */
type View = { kind: Screen } | { kind: "category"; drive: string; category: CategoryKey };

export default function App() {
  const [view, setView] = useState<View>({ kind: "overview" });
  const [systemDrive, setSystemDrive] = useState<string | null>(null);
  const { drives, rescan, elevated } = useDrives();
  const ops = useOperations();
  const [opFocus, setOpFocus] = useState<number | null>(null);

  const sidebarCurrent: Screen = view.kind === "category" ? "overview" : view.kind;
  const viewKey =
    view.kind === "category" ? `category-${view.drive}-${view.category}` : view.kind === "operations" ? `operations-${opFocus}` : view.kind;
  const categoryEntry = view.kind === "category" ? drives.find((d) => d.view.info.letter === view.drive) : undefined;

  return (
    <MotionConfig reducedMotion="user">
      <div className="window">
        <TitleBar />
        <div className="flex min-h-0">
          <Sidebar
            current={sidebarCurrent}
            busy={ops.running !== null}
            onChange={(s) => {
              setOpFocus(null);
              setView({ kind: s });
            }}
          />
          <main className="min-w-0 flex-1 overflow-y-auto px-6 pb-6 pt-1">
            <AnimatePresence mode="wait" initial={false}>
              <motion.div
                key={viewKey}
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -4 }}
                transition={{ duration: 0.22, ease: [0.16, 1, 0.3, 1] }}
                className="h-full"
              >
                {view.kind === "overview" && (
                  <Overview
                    drives={drives}
                    elevated={elevated}
                    onRescan={rescan}
                    onOpenSystem={(d) => {
                      setSystemDrive(d);
                      setView({ kind: "system" });
                    }}
                    onOpenCategory={(drive, category) => setView({ kind: "category", drive, category })}
                  />
                )}
                {view.kind === "files" && <Files drives={drives} />}
                {view.kind === "operations" && <Operations running={ops.running} history={ops.history} focus={opFocus} />}
                {view.kind === "system" &&<System drives={drives} drive={systemDrive} onDrive={setSystemDrive} />}
                {view.kind === "category" && categoryEntry && (
                  <Category entry={categoryEntry} category={view.category} onBack={() => setView({ kind: "overview" })} />
                )}
              </motion.div>
            </AnimatePresence>
          </main>
        </div>
      </div>
      <OperationToast
        running={ops.running}
        last={ops.history[0] ?? null}
        onDetails={(id) => {
          setOpFocus(id);
          setView({ kind: "operations" });
        }}
      />
    </MotionConfig>
  );
}
