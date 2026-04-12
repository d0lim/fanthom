import { useReducer } from "react";
import { AppStateContext, AppDispatchContext, appReducer, initialState } from "./state";
import { UrlInput } from "./components/UrlInput";
import { PipelineProgress } from "./components/PipelineProgress";
import { TabCanvas } from "./components/TabCanvas";
import { AudioPlayer } from "./components/AudioPlayer";
import { BpmControl } from "./components/BpmControl";
import { TransposeControl } from "./components/TransposeControl";
import { OptimizeToggle } from "./components/OptimizeToggle";
import { ExportMenu } from "./components/ExportMenu";

export default function App() {
  const [state, dispatch] = useReducer(appReducer, initialState);

  return (
    <AppStateContext.Provider value={state}>
      <AppDispatchContext.Provider value={dispatch}>
        <div className="flex flex-col h-screen bg-zinc-950 text-zinc-100">
          <header className="border-b border-zinc-800 p-4">
            <UrlInput />
          </header>

          <main className="flex-1 overflow-hidden relative">
            {state.pipeline.status === "processing" && <PipelineProgress />}
            {state.tabSheet && <TabCanvas />}
            {state.pipeline.status === "idle" && !state.tabSheet && (
              <div className="flex items-center justify-center h-full text-zinc-500">
                Paste a YouTube URL to get started
              </div>
            )}
            {state.pipeline.status === "error" && (
              <div className="flex items-center justify-center h-full text-red-400">
                {state.pipeline.message}
              </div>
            )}
          </main>

          {state.tabSheet && (
            <footer className="border-t border-zinc-800 p-3 flex flex-wrap items-center gap-4">
              <AudioPlayer />
              <div className="w-px h-6 bg-zinc-800" />
              <BpmControl />
              <div className="w-px h-6 bg-zinc-800" />
              <TransposeControl />
              <OptimizeToggle />
              <div className="ml-auto">
                <ExportMenu />
              </div>
            </footer>
          )}
        </div>
      </AppDispatchContext.Provider>
    </AppStateContext.Provider>
  );
}
