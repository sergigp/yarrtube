import { describe, expect, it } from "vitest";
import type { VideoStatus } from "@/api/types";
import { aStep, someSteps } from "@/test/helpers";
import { canRetry, failingStepLines, statusLabel } from "./videoSteps";

const NOW = new Date("2026-11-10T10:00:00Z");

describe("statusLabel", () => {
  it("reads each download status in words", () => {
    const statuses: VideoStatus[] = [
      "PENDING",
      "IN_PROGRESS",
      "DOWNLOADED",
      "RETRYING",
      "FAILED",
      "UNAVAILABLE",
    ];

    expect(statuses.map(statusLabel)).toEqual([
      "Pending",
      "Downloading",
      "Downloaded",
      "Retrying",
      "Failed",
      "Unavailable",
    ]);
  });
});

describe("failingStepLines", () => {
  it("reads a retrying step with when its next attempt is due and why it failed", () => {
    const steps = someSteps({
      download: aStep({
        status: "RETRYING",
        failed_attempts: 2,
        next_attempt_at: "2026-11-10T12:00:00Z",
        failure: "HTTP Error 503",
      }),
      subtitles: aStep({
        status: "RETRYING",
        failed_attempts: 1,
        next_attempt_at: "2026-11-10T10:10:00Z",
      }),
      chapters: aStep({
        status: "RETRYING",
        failed_attempts: 2,
        next_attempt_at: "2026-11-10T11:00:00Z",
      }),
      quality: aStep({
        status: "RETRYING",
        failed_attempts: 5,
        next_attempt_at: "2026-11-12T10:00:00Z",
      }),
    });

    expect(failingStepLines(steps, NOW)).toEqual([
      "Download: retrying, next attempt in 2 hours · HTTP Error 503",
      "Subtitles: retrying, next attempt in 10 minutes",
      "Chapters: retrying, next attempt in 1 hour",
      "Quality check: retrying, next attempt in 2 days",
    ]);
  });

  it("reads a failed step with how many attempts it gave up after and why it failed", () => {
    const steps = someSteps({
      download: aStep({
        status: "FAILED",
        failed_attempts: 11,
        failure: "HTTP Error 503",
      }),
      summary: aStep({ status: "FAILED", failed_attempts: 11 }),
      subtitles: aStep({ status: "UNAVAILABLE" }),
      thumbnail: aStep({ status: "DONE" }),
    });

    expect(failingStepLines(steps, NOW)).toEqual([
      "Download: failed, gave up after 11 attempts · HTTP Error 503",
      "Summary: failed, gave up after 11 attempts",
    ]);
  });

  it("reads a retrying step whose next attempt is overdue as due now", () => {
    const steps = someSteps({
      download: aStep({
        status: "RETRYING",
        failed_attempts: 1,
        next_attempt_at: "2026-11-10T09:50:00Z",
      }),
    });

    expect(failingStepLines(steps, NOW)).toEqual([
      "Download: retrying, next attempt due now",
    ]);
  });
});

describe("canRetry", () => {
  it("is true only when a step is retrying or failed", () => {
    expect(
      [
        someSteps({
          download: aStep({ status: "DONE" }),
          subtitles: aStep({ status: "UNAVAILABLE" }),
        }),
        someSteps({ download: aStep({ status: "IN_PROGRESS" }) }),
        someSteps({ chapters: aStep({ status: "RETRYING" }) }),
        someSteps({ summary: aStep({ status: "FAILED" }) }),
      ].map(canRetry),
    ).toEqual([false, false, true, true]);
  });
});
