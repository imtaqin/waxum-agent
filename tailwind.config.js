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
          400: '#fde047',
          500: '#facc15',
          600: '#ca8a04',
          glow: '#fef08a',
        },
        cyber: {
          pink: '#ff2fb4',
          cyan: '#22e5e5',
        },
      },
      fontFamily: {
        sans: ['Geist', 'Inter', 'system-ui', 'sans-serif'],
        mono: ['Geist Mono', 'JetBrains Mono', 'ui-monospace', 'monospace'],
      },
      boxShadow: {
        hud: '0 0 0 1px rgba(250,204,21,0.3), 0 0 24px rgba(250,204,21,0.2)',
        'hud-lg': '0 0 0 1px rgba(250,204,21,0.4), 0 0 48px rgba(250,204,21,0.3)',
        cyber: '0 0 0 1px rgba(255,47,180,0.3), 0 0 20px rgba(255,47,180,0.25)',
      },
    },
  },
  plugins: [],
}
