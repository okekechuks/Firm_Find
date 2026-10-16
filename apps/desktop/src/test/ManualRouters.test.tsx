import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { ManualRouters } from "../features/ManualRouters";
import * as backend from "../lib/backend";
import type { EndpointResult, SavedRouter } from "../types/contracts";
vi.mock("../lib/backend", async (original) => ({...await original<typeof backend>(),
  getManualRouters: vi.fn(), addManualRouter: vi.fn(), removeManualRouter: vi.fn(),
  probeManagementEndpoint: vi.fn(), cancelEndpointProbe: vi.fn(),
}));
const router: SavedRouter = {id: "a".repeat(64), address: "http://192.168.100.1/", label: "Test router", interfaceId: null, deviceId: null, createdAt: 1, state: "discovered", lastResult: null};
const response: EndpointResult = {code: "authentication_required", state: "authentication_required", message: "Authentication required; no credentials sent.", status: 401, server: "fixture", checkedAt: 1, elapsedMs: 12, resolvedAddress: "192.168.100.1", tls: "not_used"};
beforeEach(() => { vi.mocked(backend.getManualRouters).mockResolvedValue([]); vi.mocked(backend.cancelEndpointProbe).mockResolvedValue(); });

it("validates manual input, displays backend rejection, saves normalized entries without probing", async () => {
  const close = vi.fn();
  const user = userEvent.setup();
  render(<ManualRouters devices={[]} draft={{address: "", interfaceId: null}} onClose={close}/>);
  await screen.findByText(/No saved routers/);
  const dialog = screen.getByRole("dialog", {name: "Add Router"});
  const input = within(dialog).getByLabelText("Management address");
  await user.type(input, "user@192.168.100.1");
  await user.click(within(dialog).getByRole("button", {name: "Save router"}));
  expect(await screen.findByRole("alert")).toHaveTextContent(/without credentials/);
  expect(backend.addManualRouter).not.toHaveBeenCalled();
  await user.clear(input); await user.type(input, "8.8.8.8");
  vi.mocked(backend.addManualRouter).mockRejectedValueOnce({code: "target_out_of_scope", message: "Only local targets allowed."});
  await user.click(within(dialog).getByRole("button", {name: "Save router"}));
  expect(await screen.findByRole("alert")).toHaveTextContent("Only local targets allowed.");
  await user.clear(input); await user.type(input, "192.168.100.1");
  vi.mocked(backend.addManualRouter).mockResolvedValue([router]);
  await user.click(within(dialog).getByRole("button", {name: "Save router"}));
  expect(await screen.findByRole("article", {name: "Test router"})).toBeInTheDocument();
  expect(close).toHaveBeenCalledOnce(); expect(backend.probeManagementEndpoint).not.toHaveBeenCalled();
});

it("requires consent, renders progress and authentication result, retries and removes", async () => {
  vi.mocked(backend.getManualRouters).mockResolvedValue([router]);
  vi.mocked(backend.probeManagementEndpoint).mockResolvedValue(response);
  vi.mocked(backend.removeManualRouter).mockResolvedValue([]);
  const user = userEvent.setup(); render(<ManualRouters devices={[]} draft={null} onClose={vi.fn()}/>);
  const check = await screen.findByRole("button", {name: "Check / Retry"}); expect(check).toBeDisabled();
  await user.click(screen.getByRole("checkbox", {name: /I own or administer/}));
  await user.click(check);
  expect(await screen.findByText(/Authentication required; no credentials sent/)).toBeInTheDocument();
  expect(backend.probeManagementEndpoint).toHaveBeenCalledWith(router.id, true);
  await user.click(check); expect(backend.probeManagementEndpoint).toHaveBeenCalledTimes(2);
  await user.click(screen.getByRole("button", {name: "Remove saved entry"}));
  await waitFor(() => expect(screen.queryByRole("article")).not.toBeInTheDocument());
});

it("cancelled checks cannot replace UI with a late success", async () => {
  vi.mocked(backend.getManualRouters).mockResolvedValue([router]);
  let finish!: (r: EndpointResult) => void;
  vi.mocked(backend.probeManagementEndpoint).mockReturnValue(new Promise((resolve) => { finish = resolve; }));
  const user = userEvent.setup(); render(<ManualRouters devices={[]} draft={null} onClose={vi.fn()}/>);
  await screen.findByRole("article"); await user.click(screen.getByRole("checkbox"));
  await user.click(screen.getByRole("button", {name: "Check / Retry"}));
  expect(await screen.findByText(/Checking one management endpoint/)).toBeInTheDocument();
  await user.click(screen.getByRole("button", {name: "Cancel check"}));
  expect(await screen.findByText(/Check cancelled/)).toBeInTheDocument();
  await act(async () => { finish(response); });
  expect(screen.queryByText(response.message)).not.toBeInTheDocument();
  expect(screen.getByText(/Check cancelled/)).toBeInTheDocument();
});

it("shows persisted observations as historical and guides deliberate certificate trust", async () => {
  vi.mocked(backend.getManualRouters).mockResolvedValue([{...router, lastResult: {...response, code: "tls_trust_failure", status: null, tls: "untrusted"}}]);
  render(<ManualRouters devices={[]} draft={null} onClose={vi.fn()}/>);
  expect(await screen.findByText(/Historical observation/)).toBeInTheDocument();
  expect(screen.getByText(/Certificate warning and deliberate trust steps/)).toBeInTheDocument();
  expect(screen.getByText(/OBTAINER always validates certificates/)).toBeInTheDocument();
  expect(backend.probeManagementEndpoint).not.toHaveBeenCalled();
});

it("late initial list cannot hide a newly persisted entry", async () => {
  let finish!: (entries: SavedRouter[]) => void;
  vi.mocked(backend.getManualRouters).mockReturnValue(new Promise((resolve) => {finish = resolve;}));
  vi.mocked(backend.addManualRouter).mockResolvedValue([router]);
  const user = userEvent.setup();
  render(<ManualRouters devices={[]} draft={{address: "192.168.100.1", interfaceId: null}} onClose={vi.fn()}/>);
  await user.click(screen.getByRole("button", {name: "Save router"}));
  expect(await screen.findByRole("article", {name: "Test router"})).toBeInTheDocument();
  await act(async () => {finish([]);});
  expect(screen.getByRole("article", {name: "Test router"})).toBeInTheDocument();
});

it("serializes removals so committed list updates cannot arrive out of order", async () => {
  const other = {...router, id: "b".repeat(64), label: "Other router"};
  vi.mocked(backend.getManualRouters).mockResolvedValue([router, other]);
  let finish!: (entries: SavedRouter[]) => void;
  vi.mocked(backend.removeManualRouter).mockReturnValueOnce(new Promise((resolve) => {finish = resolve;})).mockResolvedValueOnce([]);
  const user = userEvent.setup(); render(<ManualRouters devices={[]} draft={null} onClose={vi.fn()}/>);
  const first = await screen.findByRole("article", {name: "Test router"});
  await user.click(within(first).getByRole("button", {name: "Remove saved entry"}));
  await user.click(within(screen.getByRole("article", {name: "Other router"})).getByRole("button", {name: "Remove saved entry"}));
  expect(backend.removeManualRouter).toHaveBeenCalledTimes(1);
  await act(async () => {finish([other]);});
  await waitFor(() => expect(backend.removeManualRouter).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(screen.queryByRole("article")).not.toBeInTheDocument());
});
