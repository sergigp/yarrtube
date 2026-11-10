import { afterEach, describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { VideoDetail } from "./VideoDetail";
import { usePlaylistVideos } from "@/api/queries";
import type { Video } from "@/api/types";
import {
  aChannel,
  aStep,
  aVideo,
  mockApi,
  renderWithProviders,
  someSteps,
} from "@/test/helpers";

describe("VideoDetail", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("lists each retrying or failed step with its next attempt or how it gave up", () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.setSystemTime(new Date("2026-11-10T10:00:00Z"));
    const video = aVideo({
      status: "DOWNLOADED",
      steps: someSteps({
        download: aStep({ status: "DONE" }),
        subtitles: aStep({
          status: "RETRYING",
          failed_attempts: 2,
          next_attempt_at: "2026-11-10T11:00:00Z",
        }),
        summary: aStep({ status: "FAILED", failed_attempts: 11 }),
      }),
    });

    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    expect(
      screen.getByRole("list", { name: "Failing steps" }),
    ).toHaveTextContent(
      "Subtitles: retrying, next attempt in 1 hourSummary: failed, gave up after 11 attempts",
    );
  });

  it("shows the storage location with no More control", () => {
    // The matchMedia stub reports a non-desktop viewport.
    const video = aVideo({ title: "My Video", filename: "my-video.mp4" });

    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    expect(screen.getByText("My Video")).toBeInTheDocument();
    expect(screen.getByText("playlists/mix")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /More/ }),
    ).not.toBeInTheDocument();
  });

  it("shows the AI summary", () => {
    const video = aVideo({ summary: "What the video says." });

    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    expect(
      screen.getByRole("region", { name: "AI summary" }),
    ).toHaveTextContent("AI summaryWhat the video says.");
  });

  it("shows no AI summary label when the video has none", () => {
    const video = aVideo({ summary: null });

    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    expect(
      screen.queryByRole("region", { name: "AI summary" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("AI summary")).not.toBeInTheDocument();
  });

  it("links the video on YouTube from the title row, in a new tab", () => {
    const video = aVideo({ id: "rec-abc", youtube_id: "abc" });
    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    const link = screen.getByRole("link", { name: "Open video on YouTube" });
    expect(link).toHaveAttribute("href", "https://www.youtube.com/watch?v=abc");
    expect(link).toHaveAttribute("target", "_blank");
  });

  it("shows the season folder as the storage location", () => {
    const video = aVideo({ filename: "Season 2026/S2026E031501 - Title.mp4" });
    renderWithProviders(
      <VideoDetail
        basePath="channels/some-channel"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    expect(
      screen.getByText("channels/some-channel/Season 2026"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/S2026E031501/)).not.toBeInTheDocument();
  });

  it("shows the status of a video that is not downloaded", () => {
    const video = aVideo({ status: "RETRYING", filename: null });
    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    expect(screen.getByText("Retrying")).toBeInTheDocument();
  });

  it("composes the meta line from the available parts", () => {
    const video = aVideo({
      channel_name: "The Channel",
      published_at: "2026-01-02T00:00:00Z",
      synced_at: null,
    });
    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    // In a playlist view (no channel prop) the meta line names the channel.
    expect(screen.getByText("The Channel")).toBeInTheDocument();
    expect(screen.getByText(/Published/)).toBeInTheDocument();
    expect(screen.queryByText(/Synced/)).not.toBeInTheDocument();
  });

  it("links the channel avatar instead of naming the channel in a channel view", () => {
    const channel = aChannel({ id: "chan", name: "The Channel" });
    const video = aVideo({ channel_name: "The Channel" });
    renderWithProviders(
      <VideoDetail
        basePath="channels/chan"
        video={video}
        channel={channel}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    expect(screen.getByRole("link", { name: "The Channel" })).toHaveAttribute(
      "href",
      "/channels/chan",
    );
    expect(
      screen.queryByText("The Channel", { selector: "span" }),
    ).not.toBeInTheDocument();
  });
  it("retries a video from the detail pane and shows it refetched", async () => {
    const failed = aVideo({
      id: "abc",
      status: "FAILED",
      filename: null,
      steps: someSteps({
        download: aStep({ status: "FAILED", failed_attempts: 11 }),
      }),
    });
    let stored: Video = failed;
    const retry = vi.fn(() => {
      stored = { ...failed, status: "PENDING", steps: someSteps() };
      return null;
    });
    mockApi({
      "GET /api/playlists/PL1/videos": () => [stored],
      "POST /api/videos/abc/retry": retry,
    });
    renderWithProviders(<PlaylistVideoDetail />);

    await userEvent
      .setup()
      .click(await screen.findByRole("button", { name: "Retry now" }));

    expect(retry).toHaveBeenCalledOnce();
    expect(await screen.findByText("Pending")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Retry now" }),
    ).not.toBeInTheDocument();
  });

  it("offers no retry for a video with no retrying or failed step", () => {
    const video = aVideo({
      steps: someSteps({
        download: aStep({ status: "DONE" }),
        subtitles: aStep({ status: "UNAVAILABLE" }),
      }),
    });

    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    expect(
      screen.queryByRole("button", { name: "Retry now" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("list", { name: "Failing steps" }),
    ).not.toBeInTheDocument();
  });

  it("tells the user when retrying fails and leaves the video as it was", async () => {
    const alert = vi.spyOn(window, "alert").mockImplementation(() => {});
    const video = aVideo({
      id: "abc",
      title: "My Video",
      status: "FAILED",
      filename: null,
      steps: someSteps({
        download: aStep({ status: "FAILED", failed_attempts: 11 }),
      }),
    });
    mockApi({
      "GET /api/playlists/PL1/videos": [video],
      "POST /api/videos/abc/retry": {
        status: 500,
        error: "database is locked",
      },
    });
    renderWithProviders(<PlaylistVideoDetail />);

    await userEvent
      .setup()
      .click(await screen.findByRole("button", { name: "Retry now" }));

    await vi.waitFor(() =>
      expect(alert).toHaveBeenCalledWith(
        'Failed to retry "My Video": database is locked',
      ),
    );
    expect(screen.getByText("Failed")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Retry now" })).toBeEnabled();
  });

  it("marks the selected video watched from the detail pane", async () => {
    const markWatched = vi.fn(() => null);
    mockApi({ "POST /api/videos/abc/watched": markWatched });
    const video = aVideo({ id: "abc", title: "My Video" });
    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "Mark watched" }));

    expect(markWatched).toHaveBeenCalledOnce();
  });

  it("says a watched video is watched first on the meta line, with no mark button", () => {
    const video = aVideo({
      watched: true,
      published_at: "2026-01-02T00:00:00Z",
      synced_at: null,
    });
    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    expect(screen.getByText(/Published/).parentElement).toHaveTextContent(
      /^Watched · Published/,
    );
    expect(
      screen.queryByRole("button", { name: "Mark watched" }),
    ).not.toBeInTheDocument();
  });

  it("disables the speed control until the video is downloaded", () => {
    const video = aVideo({ status: "IN_PROGRESS", filename: null });

    renderWithProviders(
      <VideoDetail
        basePath="playlists/mix"
        video={video}
        playbackRate={1}
        onPlaybackRateChange={() => {}}
      />,
    );

    expect(
      screen.getByRole("button", { name: "Playback speed" }),
    ).toBeDisabled();
  });
});

/** The detail pane of the first video of playlist PL1, as the API lists it. */
function PlaylistVideoDetail() {
  const { data } = usePlaylistVideos("PL1");
  const video = data?.[0];
  return video ? (
    <VideoDetail
      basePath="playlists/mix"
      video={video}
      playbackRate={1}
      onPlaybackRateChange={() => {}}
    />
  ) : null;
}
