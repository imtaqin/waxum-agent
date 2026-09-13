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
          900: '#050b14',
          800: '#0a1420',
          700: '#0f1e30',
        },
        hud: {
          300: '#a5f3fc',
          400: '#22d3ee',
          500: '#0ea5b8',
          600: '#0891b2',
          glow: '#a5f3fc',
        },
        cyber: {
          pink: '#ff2fb4',
          amber: '#ffb020',
        },
        void: {
          900: '#050b14',
          800: '#0a1420',
          700: '#0f1e30',
        },
      },
      fontFamily: {
        sans: ['Geist', 'Inter', 'system-ui', 'sans-serif'],
        mono: ['Geist Mono', 'JetBrains Mono', 'ui-monospace', 'monospace'],
      },
      boxShadow: {
        hud: '0 0 0 1px rgba(34,211,238,0.35), 0 0 24px rgba(34,211,238,0.25)',
        'hud-lg': '0 0 0 1px rgba(34,211,238,0.45), 0 0 48px rgba(34,211,238,0.35)',
        cyber: '0 0 0 1px rgba(255,47,180,0.35), 0 0 20px rgba(255,47,180,0.3)',
      },
    },
  },
  plugins: [],
}
