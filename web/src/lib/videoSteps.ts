import type { Step, VideoStatus, VideoStepName, VideoSteps } from "@/api/types";

const STATUS_LABELS: Record<VideoStatus, string> = {
  PENDING: "Pending",
  IN_PROGRESS: "Downloading",
  DOWNLOADED: "Downloaded",
  RETRYING: "Retrying",
  FAILED: "Failed",
  UNAVAILABLE: "Unavailable",
};

/** The download status as the status indicator reads it. */
export function statusLabel(status: VideoStatus): string {
  return STATUS_LABELS[status];
}

/**
 * One line per Retrying or Failed step, in step order, as of `now`: e.g.
 * "Download: retrying, next attempt in 2 hours · HTTP Error 503" or
 * "Summary: failed, gave up after 11 attempts".
 */
export function failingStepLines(steps: VideoSteps, now: Date): string[] {
  return STEP_NAMES.flatMap(([name, label]) => {
    const step = steps[name];
    const state = stepState(step, now);
    return state ? [withFailure(`${label}: ${state}`, step)] : [];
  });
}

/** Every step in the order the pane lists them, with its name in words. */
const STEP_NAMES: [VideoStepName, string][] = [
  ["download", "Download"],
  ["thumbnail", "Thumbnail"],
  ["subtitles", "Subtitles"],
  ["chapters", "Chapters"],
  ["quality", "Quality check"],
  ["summary", "Summary"],
  ["categorization", "Categorization"],
];

const MINUTE_MS = 60 * 1000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;

/** What a Retrying or Failed step is doing; `null` for any other. */
function stepState(step: Step, now: Date): string | null {
  switch (step.status) {
    case "RETRYING":
      return `retrying, ${nextAttempt(step, now)}`;
    case "FAILED":
      return `failed, gave up after ${step.failed_attempts} attempts`;
    default:
      return null;
  }
}

function nextAttempt(step: Step, now: Date): string {
  const dueInMs =
    new Date(step.next_attempt_at ?? now).getTime() - now.getTime();
  return dueInMs > 0
    ? `next attempt in ${inWords(dueInMs)}`
    : "next attempt due now";
}

/** A duration in its largest whole unit: minutes, hours or days. */
function inWords(ms: number): string {
  const [unit, unitMs] =
    ms >= DAY_MS
      ? ["day", DAY_MS]
      : ms >= HOUR_MS
        ? ["hour", HOUR_MS]
        : ["minute", MINUTE_MS];
  const count = Math.max(1, Math.round(ms / unitMs));
  return `${count} ${unit}${count === 1 ? "" : "s"}`;
}

function withFailure(line: string, step: Step): string {
  return step.failure ? `${line} · ${step.failure}` : line;
}

/** Whether any step is Retrying or Failed, so the video can be retried. */
export function canRetry(steps: VideoSteps): boolean {
  return Object.values(steps).some(
    (step) => step.status === "RETRYING" || step.status === "FAILED",
  );
}
