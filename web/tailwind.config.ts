import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        sand: "#f2e9d8",
        graphite: "#20221f",
        olive: "#6f7b50",
        signal: "#cd7c2f",
        steel: "#7f8b7d",
      },
      boxShadow: {
        panel: "0 14px 40px rgba(24, 26, 23, 0.12)",
      },
      backgroundImage: {
        grain:
          "radial-gradient(circle at 20% 20%, rgba(205,124,47,0.1), transparent 26%), radial-gradient(circle at 80% 0%, rgba(111,123,80,0.16), transparent 32%), linear-gradient(140deg, #f5efe1 0%, #ebe3d1 44%, #dad4c8 100%)",
      },
    },
  },
  plugins: [],
} satisfies Config;
