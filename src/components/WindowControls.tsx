import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";

const appWindow = getCurrentWindow();

export function WindowControls() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    const refresh = () => {
      void appWindow
        .isMaximized()
        .then((value) => {
          if (!disposed) setMaximized(value);
        })
        .catch(() => undefined);
    };

    refresh();
    void appWindow
      .onResized(refresh)
      .then((stopListening) => {
        if (disposed) stopListening();
        else unlisten = stopListening;
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return (
    <div className="flex items-center gap-1">
      <button
        type="button"
        onClick={() => void appWindow.minimize()}
        className="grid h-8 w-8 place-items-center rounded-xl text-slate-500 transition hover:bg-white/8 hover:text-slate-100"
        aria-label="最小化"
        title="最小化"
      >
        <Minus size={15} />
      </button>
      <button
        type="button"
        onClick={() =>
          void appWindow
            .toggleMaximize()
            .then(() => appWindow.isMaximized())
            .then(setMaximized)
            .catch(() => undefined)
        }
        className="grid h-8 w-8 place-items-center rounded-xl text-slate-500 transition hover:bg-white/8 hover:text-slate-100"
        aria-label={maximized ? "还原窗口" : "最大化"}
        title={maximized ? "还原窗口" : "最大化"}
      >
        {maximized ? <Copy size={13} /> : <Square size={13} />}
      </button>
      <button
        type="button"
        onClick={() => void appWindow.close()}
        className="grid h-8 w-8 place-items-center rounded-xl text-slate-500 transition hover:bg-rose-400/15 hover:text-rose-200"
        aria-label="关闭 EMCL"
        title="关闭 EMCL"
      >
        <X size={16} />
      </button>
    </div>
  );
}
