import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

afterEach(() => {
  cleanup();
  window.location.hash = "";
});
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  })),
});

// JSDOM does not implement native modal dialogs; browser focus and Escape are tested natively.
HTMLDialogElement.prototype.showModal=function(){this.setAttribute("open","");};
HTMLDialogElement.prototype.close=function(){this.removeAttribute("open");};

window.scrollTo=vi.fn();
