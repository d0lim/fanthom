import { useAppState } from "../state";

const STEPS = [
  { key: "extract", label: "Extract Audio" },
  { key: "separate", label: "Separate Bass" },
  { key: "transcribe", label: "Detect Pitch" },
  { key: "convert", label: "Generate Tab" },
];

export function PipelineProgress() {
  const state = useAppState();
  if (state.pipeline.status !== "processing") return null;

  const currentStep = state.pipeline.step;
  const currentIdx = STEPS.findIndex((s) => s.key === currentStep);

  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center gap-6 bg-zinc-950/80 backdrop-blur-sm z-10">
      <div className="flex gap-2 items-center">
        {STEPS.map((step, i) => {
          const isDone = i < currentIdx;
          const isCurrent = i === currentIdx;
          return (
            <div key={step.key} className="flex items-center gap-2">
              <div
                className={`w-3 h-3 rounded-full ${
                  isDone
                    ? "bg-green-400"
                    : isCurrent
                      ? "bg-amber-400 animate-pulse"
                      : "bg-zinc-700"
                }`}
              />
              <span
                className={`text-sm ${
                  isDone
                    ? "text-green-400"
                    : isCurrent
                      ? "text-amber-400"
                      : "text-zinc-600"
                }`}
              >
                {step.label}
              </span>
              {i < STEPS.length - 1 && (
                <div className={`w-8 h-px ${isDone ? "bg-green-400" : "bg-zinc-700"}`} />
              )}
            </div>
          );
        })}
      </div>
      <p className="text-zinc-400 text-sm">{state.pipeline.message}</p>
    </div>
  );
}
