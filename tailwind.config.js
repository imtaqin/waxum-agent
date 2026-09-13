/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,ts,tsx}'],
  theme: {
    extend: {
      colors: {
        emerald: {
          500: '#10b981',
          600: '#059669',
          700: '#047857',
        },
        charcoal: {
          900: '#0a0f0d',
          800: '#0e1512',
          700: '#111827',
        },
        hud: {
          400: '#5eead4',
          500: '#22d3ee',
          600: '#0891b2',
          glow: '#67e8f9',
        },
      },
      fontFamily: {
        sans: ['Geist', 'Inter', 'system-ui', 'sans-serif'],
        mono: ['Geist Mono', 'JetBrains Mono', 'ui-monospace', 'monospace'],
      },
      boxShadow: {
        hud: '0 0 0 1px rgba(34,211,238,0.25), 0 0 24px rgba(34,211,238,0.15)',
        'hud-lg': '0 0 0 1px rgba(34,211,238,0.35), 0 0 48px rgba(34,211,238,0.25)',
      },
    },
  },
  plugins: [],
}
