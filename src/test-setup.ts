import '@testing-library/jest-dom';

// Polyfill window.matchMedia for antd responsive components
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

// Polyfill ResizeObserver for antd resize-observer
class ResizeObserverPolyfill {
  observe() {}
  unobserve() {}
  disconnect() {}
}
window.ResizeObserver = ResizeObserverPolyfill as unknown as typeof ResizeObserver;

// Polyfill getComputedStyle for antd
const originalGetComputedStyle = window.getComputedStyle;
window.getComputedStyle = (elt: Element, pseudoElt?: string | null) => {
  const style = originalGetComputedStyle(elt, pseudoElt);
  return style;
};
