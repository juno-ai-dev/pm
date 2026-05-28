/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        // Ink / paper / accent — three colors only per PLAN.md §3.
        ink: '#0a0a0a',
        paper: '#fafaf7',
        accent: '#ff5b35',
        muted: '#6b6b66',
        rule: '#1a1a1a',
      },
      fontFamily: {
        sans: ['"Inter"', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'ui-monospace', 'monospace'],
      },
      letterSpacing: {
        wider: '0.04em',
      },
    },
  },
  plugins: [],
}
