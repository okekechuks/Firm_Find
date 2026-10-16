import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { ViewBoundary } from "../components/ViewBoundary";

it("contains a failed view, retries it, and recovers when navigation changes", async () => {
  vi.spyOn(console, "error").mockImplementation(() => {});
  let fail = true;
  function View() {
    if (fail) throw new Error("Injected render failure");
    return <h2>Working view</h2>;
  }
  const { rerender } = render(<ViewBoundary key="devices"><View /></ViewBoundary>);
  expect(screen.getByRole("alert")).toHaveTextContent("This view could not be displayed");
  fail = false;
  await userEvent.click(screen.getByRole("button", { name: "Retry view" }));
  expect(screen.getByRole("heading", { name: "Working view" })).toBeInTheDocument();
  fail = true;
  rerender(<ViewBoundary key="failed-again"><View /></ViewBoundary>);
  expect(screen.getByRole("alert")).toBeInTheDocument();
  rerender(<ViewBoundary key="settings"><h2>Another view</h2></ViewBoundary>);
  expect(screen.getByRole("heading", { name: "Another view" })).toBeInTheDocument();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});
