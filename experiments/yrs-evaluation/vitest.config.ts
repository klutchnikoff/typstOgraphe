import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";
const root = fileURLToPath(new URL("../../", import.meta.url));
export default defineConfig({
  root,
  test: {
    environment: "jsdom",
    include: ["experiments/yrs-evaluation/probe.test.ts"],
    setupFiles: ["app/tests/setup.ts"],
    testTimeout: 60_000,
    hookTimeout: 60_000,
    maxWorkers: 1,
  },
});
