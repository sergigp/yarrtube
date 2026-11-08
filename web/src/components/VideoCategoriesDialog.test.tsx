import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { VideoCategoryChoice } from "@/api/types";
import { VideoCategoriesDialog } from "./VideoCategoriesDialog";
import {
  aPreferences,
  aVideoCategoryChoice,
  mockApi,
  renderWithProviders,
  type Routes,
} from "@/test/helpers";

function renderDialog(routes: Routes) {
  const fetchMock = mockApi({
    "GET /api/preferences": aPreferences(),
    ...routes,
  });
  renderWithProviders(
    <VideoCategoriesDialog
      recordId="rec1"
      title="Najdorf in 10 minutes"
      sourceName="@GothamChess"
      open
      onOpenChange={() => {}}
    />,
  );
  return { fetchMock };
}

/** Each chip as its accessible name, whether it's filled and whether it's locked. */
async function chips(): Promise<[string, boolean, boolean][]> {
  return within(await screen.findByRole("toolbar", { name: "Categories" }))
    .getAllByRole("button")
    .map((chip) => [
      chip.getAttribute("aria-label") ?? chip.textContent ?? "",
      chip.getAttribute("aria-pressed") === "true",
      chip.hasAttribute("disabled"),
    ]);
}

const chess = aVideoCategoryChoice({
  id: "chess",
  name: "Chess",
  belongs: true,
  from: "source",
});
const funny = aVideoCategoryChoice({ id: "funny", name: "Funny" });
const sicilian = aVideoCategoryChoice({
  id: "sicilian",
  name: "Sicilian",
  belongs: true,
  from: "ai",
  verdict: "yes",
});

function categoriesOf(...categories: VideoCategoryChoice[]) {
  return { categorization_enabled: true, categories };
}

/** The bodies sent with `method`, in order. */
function bodies(
  fetchMock: ReturnType<typeof mockApi>,
  method: string,
): unknown[] {
  return fetchMock.mock.calls
    .filter(([, init]) => (init as RequestInit | undefined)?.method === method)
    .map(([, init]) => JSON.parse((init as RequestInit).body as string));
}

describe("VideoCategoriesDialog", () => {
  it("shows locked source chips, AI marks and filled chips", async () => {
    renderDialog({
      "GET /api/videos/rec1/categories": categoriesOf(chess, funny, sicilian),
    });

    expect(
      screen.getByRole("dialog", { name: "Najdorf in 10 minutes" }),
    ).toBeInTheDocument();
    expect(await chips()).toEqual([
      ["Chess, from @GothamChess", true, true],
      ["Funny", false, false],
      ["Sicilian, decided by AI", true, false],
    ]);
    expect(screen.getByText("From @GothamChess")).toBeInTheDocument();
    expect(screen.getByText("AI")).toBeInTheDocument();
  });

  it("sets a category by hand when a chip is clicked", async () => {
    const userNo = {
      ...sicilian,
      belongs: false,
      from: "user" as const,
      verdict: "no" as const,
    };
    const { fetchMock } = renderDialog({
      "GET /api/videos/rec1/categories": categoriesOf(chess, funny, sicilian),
      "PUT /api/videos/rec1/categories/sicilian": categoriesOf(
        chess,
        funny,
        userNo,
      ),
    });
    const user = userEvent.setup();

    await user.click(
      await screen.findByRole("button", { name: "Sicilian, decided by AI" }),
    );

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Sicilian" })).toHaveAttribute(
        "aria-pressed",
        "false",
      ),
    );
    expect(bodies(fetchMock, "PUT")).toEqual([{ belongs: false }]);
    expect(screen.queryByText("AI")).not.toBeInTheDocument();
  });

  it("shows the error and keeps the chip when setting fails", async () => {
    renderDialog({
      "GET /api/videos/rec1/categories": categoriesOf(chess, funny, sicilian),
      "PUT /api/videos/rec1/categories/funny": {
        status: 500,
        error: "database is locked",
      },
    });
    const user = userEvent.setup();

    await user.click(await screen.findByRole("button", { name: "Funny" }));

    expect(
      await screen.findByText('Failed to set "Funny": database is locked'),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Funny" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("re-runs AI only while AI categorization is available", async () => {
    const politics = aVideoCategoryChoice({
      id: "politics",
      name: "Politics",
      belongs: true,
      from: "ai",
      verdict: "yes",
    });
    const responses = [
      categoriesOf(politics),
      categoriesOf({ ...politics, belongs: false, from: null, verdict: null }),
    ];
    const { fetchMock } = renderDialog({
      "GET /api/preferences": aPreferences({
        ai_enabled: true,
        categorization_enabled: true,
      }),
      "GET /api/videos/rec1/categories": () =>
        responses.shift() ?? categoriesOf(),
      "DELETE /api/videos/rec1/ai-verdicts": {},
    });
    const user = userEvent.setup();

    await user.click(await screen.findByRole("button", { name: "Re-run AI" }));

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Politics" })).toHaveAttribute(
        "aria-pressed",
        "false",
      ),
    );
    expect(
      fetchMock.mock.calls.filter(
        ([, init]) => (init as RequestInit | undefined)?.method === "DELETE",
      ),
    ).toHaveLength(1);
  });

  it("offers Clear AI categories when the source has categorization off", async () => {
    const { fetchMock } = renderDialog({
      "GET /api/preferences": aPreferences({
        ai_enabled: true,
        categorization_enabled: true,
      }),
      "GET /api/videos/rec1/categories": {
        ...categoriesOf(sicilian),
        categorization_enabled: false,
      },
      "DELETE /api/videos/rec1/ai-verdicts": {},
    });
    const user = userEvent.setup();

    await user.click(
      await screen.findByRole("button", { name: "Clear AI categories" }),
    );

    await waitFor(() =>
      expect(
        fetchMock.mock.calls.filter(
          ([, init]) => (init as RequestInit | undefined)?.method === "DELETE",
        ),
      ).toHaveLength(1),
    );
    expect(
      screen.queryByRole("button", { name: "Re-run AI" }),
    ).not.toBeInTheDocument();
  });

  it("offers no Re-run AI while AI categorization is unavailable", async () => {
    renderDialog({ "GET /api/videos/rec1/categories": categoriesOf(chess) });

    await chips();

    expect(
      screen.queryByRole("button", { name: "Re-run AI" }),
    ).not.toBeInTheDocument();
  });
});
