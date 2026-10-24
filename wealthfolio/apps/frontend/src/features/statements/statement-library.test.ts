import { afterEach, describe, expect, it, vi } from "vitest";
import { notifyUnauthorized } from "@/lib/auth-token";
import {
  archiveServerReport,
  isServerReport,
  readLibrary,
  uploadStatement,
} from "./statement-library";
vi.mock("@/lib/auth-token", () => ({ notifyUnauthorized: vi.fn() }));
afterEach(() => {
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
const queued = {
  id: "a".repeat(64),
  filename: "bank.pdf",
  status: "queued",
  report: null,
  error: null,
};
describe("saved statement upload library", () => {
  it("uploads original files as multipart without converting in the browser", async () => {
    const request = vi.fn().mockResolvedValue(new Response(JSON.stringify(queued)));
    vi.stubGlobal("fetch", request);
    const file = new File(["fictional bank PDF"], "bank.pdf", { type: "application/pdf" });
    expect((await uploadStatement(file)).status).toBe("queued");
    const [url, options] = request.mock.calls[0];
    expect(url).toBe("/api/v1/statements/library");
    expect(options.method).toBe("POST");
    expect(options.body.get("file").name).toBe("bank.pdf");
    expect(options.headers).toBeUndefined();
  });
  it("rejects empty and oversize files before uploading", async () => {
    const request = vi.fn();
    vi.stubGlobal("fetch", request);
    await expect(uploadStatement(new File([], "empty.pdf"))).rejects.toThrow("nonempty");
    const large = new File(["x"], "large.pdf");
    Object.defineProperty(large, "size", { value: 50_000_001 });
    await expect(uploadStatement(large)).rejects.toThrow("50 MB");
    expect(request).not.toHaveBeenCalled();
  });
  it("restores saved library entries and recognizes their server identity", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify([queued]))));
    expect((await readLibrary())[0].filename).toBe("bank.pdf");
    expect(isServerReport(queued.id)).toBe(true);
  });
  it("rejects invalid saved reports rather than presenting corrupt charts", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response(JSON.stringify([{ ...queued, report: {} }]))),
    );
    expect((await readLibrary())[0].status).toBe("failed");
  });
  it("reports expired authentication", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          new Response(JSON.stringify({ message: "Sign in again" }), { status: 401 }),
        ),
    );
    await expect(readLibrary()).rejects.toThrow("Sign in again");
    expect(notifyUnauthorized).toHaveBeenCalledOnce();
  });
  it("keeps server failures visible when archiving", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          new Response(JSON.stringify({ message: "Storage unavailable" }), { status: 503 }),
        ),
    );
    await expect(archiveServerReport(queued.id)).rejects.toThrow("Storage unavailable");
  });
});
