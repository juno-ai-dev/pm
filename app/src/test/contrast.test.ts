import { describe, expect, it } from 'vitest'
import tokens from '../styles/tokens.css?raw'

function token(name: string): string {
  const value = new RegExp(`--${name}:\\s*(#[0-9A-Fa-f]{6})`).exec(tokens)?.[1]
  if (!value) throw new Error(`missing color token ${name}`)
  return value
}

function luminance(hex: string): number {
  const channels = [1, 3, 5].map((offset) => Number.parseInt(hex.slice(offset, offset + 2), 16) / 255)
  const [red, green, blue] = channels.map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4)
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue
}

function contrast(foreground: string, background: string): number {
  const [lighter, darker] = [luminance(foreground), luminance(background)].sort((a, b) => b - a)
  return (lighter + 0.05) / (darker + 0.05)
}

describe('small muted copy contrast', () => {
  it('meets WCAG AA on every app surface', () => {
    const foreground = token('cream-faint')
    for (const surface of ['maroon', 'surface-card', 'void-pure', 'void']) {
      expect(contrast(foreground, token(surface)), surface).toBeGreaterThanOrEqual(4.5)
    }
  })
})
