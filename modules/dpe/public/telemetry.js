import { onLCP, onINP, onCLS, onTTFB, onFCP } from './vendor/web-vitals-attribution.js';

// --- Configuration ---
const COLLECTOR_URL = '/telemetry/collect';
const MAX_BUFFER_SIZE = 50;
const MAX_ERRORS_PER_KIND = 10;

// --- Page identity ---
const pageLoadId = crypto.randomUUID?.() ?? Math.random().toString(36).slice(2);
const traceparent = document.querySelector('meta[name="traceparent"]')?.content ?? null;
const pageUrl = location.pathname;

// --- Signal buffer (bounded) ---
let buffer = [];
const errorCounts = {};

function addSignal(signal) {
  if (buffer.length >= MAX_BUFFER_SIZE) return; // prevent unbounded growth
  buffer.push({
    ...signal,
    traceparent,
    pageLoadId,
    pageUrl,
    timestamp: Date.now(),
  });
}

// --- Core Web Vitals (REQ-3.1) ---
function onVital(metric) {
  const entry = {
    type: 'web_vital',
    name: metric.name,
    value: metric.value,
    rating: metric.rating,
    navigationType: metric.navigationType,
  };

  // Attribution data — explains *why* a score is poor
  const attr = metric.attribution;
  if (attr) {
    if (metric.name === 'LCP') {
      entry.lcpElement = attr.target;
      entry.lcpUrl = attr.url;
      entry.timeToFirstByte = attr.timeToFirstByte;
      entry.resourceLoadDelay = attr.resourceLoadDelay;
      entry.resourceLoadDuration = attr.resourceLoadDuration;
      entry.elementRenderDelay = attr.elementRenderDelay;
    } else if (metric.name === 'INP') {
      entry.inpTarget = attr.interactionTarget;
      entry.inpType = attr.interactionType;
      entry.inputDelay = attr.inputDelay;
      entry.processingDuration = attr.processingDuration;
      entry.presentationDelay = attr.presentationDelay;
    } else if (metric.name === 'CLS') {
      entry.clsTarget = attr.largestShiftTarget;
    } else if (metric.name === 'TTFB') {
      entry.dnsDuration = attr.dnsDuration;
      entry.connectionDuration = attr.connectionDuration;
      entry.requestDuration = attr.requestDuration;
    }
  }

  addSignal(entry);
}

onLCP(onVital);
onINP(onVital);
onCLS(onVital);
onTTFB(onVital);
onFCP(onVital);

// --- JavaScript errors (REQ-3.2) ---
window.addEventListener('error', (e) => {
  const kind = e.target !== window ? 'resource_error' : 'js_error';
  // Cap errors per kind to prevent flood from repeated failures
  errorCounts[kind] = (errorCounts[kind] ?? 0) + 1;
  if (errorCounts[kind] > MAX_ERRORS_PER_KIND) return;

  if (e.target !== window) {
    // Resource load error (image, script, etc.)
    // Strip query parameters from resource URLs to avoid leaking tokens
    const url = e.target?.src || e.target?.href || 'unknown';
    addSignal({
      type: 'error',
      kind: 'resource_error',
      message: `Failed to load: ${url.split('?')[0]}`,
      tagName: e.target?.tagName,
    });
  } else {
    addSignal({
      type: 'error',
      kind: 'js_error',
      message: (e.message ?? '').slice(0, 256),
      filename: e.filename?.split('?')[0],
      lineno: e.lineno,
      colno: e.colno,
    });
  }
}, true); // capture phase to catch resource errors

window.addEventListener('unhandledrejection', (e) => {
  errorCounts['promise_rejection'] = (errorCounts['promise_rejection'] ?? 0) + 1;
  if (errorCounts['promise_rejection'] > MAX_ERRORS_PER_KIND) return;

  addSignal({
    type: 'error',
    kind: 'promise_rejection',
    message: String(e.reason).slice(0, 256),
  });
});

// --- Datastar SSE errors (REQ-3.3) ---
document.addEventListener('datastar-sse-error', (e) => {
  errorCounts['datastar_sse'] = (errorCounts['datastar_sse'] ?? 0) + 1;
  if (errorCounts['datastar_sse'] > MAX_ERRORS_PER_KIND) return;

  addSignal({
    type: 'error',
    kind: 'datastar_sse',
    message: (e.detail?.message ?? 'SSE error').slice(0, 256),
    url: e.detail?.url?.split('?')[0],
  });
});

// --- Long Animation Frames (LoAF) ---
if ('PerformanceLongAnimationFrameTiming' in window) {
  const observer = new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) {
      if (entry.duration >= 200) { // 200ms threshold for read-only HDA
        const scripts = entry.scripts ?? [];
        addSignal({
          type: 'loaf',
          duration: entry.duration,
          blockingDuration: entry.blockingDuration,
          firstScript: scripts[0]?.sourceURL ?? null,
          scriptCount: scripts.length,
        });
      }
    }
  });
  observer.observe({ type: 'long-animation-frame', buffered: true });
  // Clean up on page teardown
  window.addEventListener('pagehide', () => observer.disconnect(), { once: true });
}

// --- Navigation timing ---
window.addEventListener('load', () => {
  // Delay to ensure PerformanceNavigationTiming is populated
  setTimeout(() => {
    const nav = performance.getEntriesByType('navigation')[0];
    if (!nav) return;
    addSignal({
      type: 'navigation',
      dns: nav.domainLookupEnd - nav.domainLookupStart,
      tcp: nav.connectEnd - nav.connectStart,
      tls: nav.secureConnectionStart > 0 ? nav.connectEnd - nav.secureConnectionStart : 0,
      ttfb: nav.responseStart - nav.requestStart,
      download: nav.responseEnd - nav.responseStart,
      domParse: nav.domInteractive - nav.responseEnd,
      domReady: nav.domContentLoadedEventEnd - nav.domContentLoadedEventStart,
      fullLoad: nav.loadEventEnd - nav.loadEventStart,
      transferSize: nav.transferSize,
    });
  }, 0);
});

// --- Connection quality (contextual) ---
function getConnectionInfo() {
  const conn = navigator.connection;
  if (!conn) return null;
  return {
    effectiveType: conn.effectiveType,
    downlink: conn.downlink,
    rtt: conn.rtt,
    saveData: conn.saveData,
  };
}

// --- Beacon flush (REQ-3.4) ---
function flush() {
  if (buffer.length === 0) return;

  const payload = JSON.stringify({
    signals: buffer,
    connection: getConnectionInfo(),
  });
  buffer = [];

  // Use Blob with text/plain to avoid CORS preflight on sendBeacon
  const blob = new Blob([payload], { type: 'text/plain' });
  navigator.sendBeacon(COLLECTOR_URL, blob);
}

// Primary: visibilitychange (works on all browsers)
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'hidden') flush();
});

// Fallback: pagehide (Safari)
window.addEventListener('pagehide', flush);
