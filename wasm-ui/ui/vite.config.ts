import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
  plugins: [tailwindcss(), reactRouter(), tsconfigPaths()],
  server: {
    fs: {
      // Allow serving files from one level up to the project root
      // which is where the rust_simulator/pkg directory resides.
      allow: [
        '..',
      ],
    },
  },
});
