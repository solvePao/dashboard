/**
 * Mobile Stock API client: fetches real-time quotes via TradingView Scanner
 * with automatic caching (5-minute TTL) and rate limit safety.
 */

export type StockPeriod = "1D" | "5D" | "1W" | "1M" | "3M" | "6M" | "YTD" | "1Y";

export interface StockPerfData {
  "1D"?: number;
  "5D"?: number;
  "1W"?: number;
  "1M"?: number;
  "3M"?: number;
  "6M"?: number;
  "YTD"?: number;
  "1Y"?: number;
}

export interface StockQuoteData {
  symbol: string;
  ticker: string;
  name: string;
  price: number;
  change: number;
  changePercent: number;
  dayHigh: number;
  dayLow: number;
  open: number;
  prevClose: number;
  volume: number;
  marketCap?: number;
  currency: string;
  lastUpdated: string;
  perf?: StockPerfData;
}

interface TvScanRow {
  s: string;
  d: (string | number | null)[];
}

interface TvScanResponse {
  totalCount: number;
  data: TvScanRow[];
}

const COLUMNS = [
  "name",
  "description",
  "close",
  "change",
  "change_abs",
  "volume",
  "high",
  "low",
  "open",
  "market_cap_basic",
  "currency",
  "Perf.5D",
  "Perf.W",
  "Perf.1M",
  "Perf.3M",
  "Perf.6M",
  "Perf.YTD",
  "Perf.Y",
];

const CACHE_PREFIX = "mo_stock_cache_v2_";
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes

let rateLimitedUntil = 0;

function asNum(val: unknown, fallback = 0): number {
  if (typeof val === "number" && Number.isFinite(val)) return val;
  return fallback;
}

function asNullableNum(val: unknown): number | undefined {
  if (typeof val === "number" && Number.isFinite(val)) return val;
  return undefined;
}

function parseTvRow(row: TvScanRow, querySymbol: string): StockQuoteData {
  const [
    name,
    description,
    close,
    change,
    changeAbs,
    volume,
    high,
    low,
    open,
    marketCap,
    currency,
    perf5d,
    perfW,
    perf1m,
    perf3m,
    perf6m,
    perfYtd,
    perfY,
  ] = row.d;

  const price = asNum(close);
  const changeVal = asNum(changeAbs);
  const prevClose = price - changeVal;

  const now = new Date();
  const lastUpdated = now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

  const perf: StockPerfData = {
    "1D": asNum(change),
    "5D": asNullableNum(perf5d),
    "1W": asNullableNum(perfW),
    "1M": asNullableNum(perf1m),
    "3M": asNullableNum(perf3m),
    "6M": asNullableNum(perf6m),
    "YTD": asNullableNum(perfYtd),
    "1Y": asNullableNum(perfY),
  };

  return {
    symbol: (typeof name === "string" && name) || querySymbol.toUpperCase(),
    ticker: row.s,
    name: (typeof description === "string" && description) || querySymbol.toUpperCase(),
    price,
    change: changeVal,
    changePercent: asNum(change),
    dayHigh: asNum(high, price),
    dayLow: asNum(low, price),
    open: asNum(open, price),
    prevClose: Number.isFinite(prevClose) ? prevClose : price,
    volume: asNum(volume),
    marketCap: typeof marketCap === "number" && Number.isFinite(marketCap) ? marketCap : undefined,
    currency: (typeof currency === "string" && currency) || "USD",
    lastUpdated,
    perf,
  };
}

export function getCachedQuote(symbol: string): StockQuoteData | null {
  try {
    const raw = localStorage.getItem(`${CACHE_PREFIX}${symbol.toUpperCase()}`);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { quote: StockQuoteData; timestamp: number };
    if (parsed?.quote && Date.now() - parsed.timestamp < CACHE_TTL_MS) {
      return parsed.quote;
    }
  } catch {
    // Ignore storage parse issues
  }
  return null;
}

function setCachedQuote(symbol: string, quote: StockQuoteData): void {
  try {
    localStorage.setItem(
      `${CACHE_PREFIX}${symbol.toUpperCase()}`,
      JSON.stringify({ quote, timestamp: Date.now() })
    );
  } catch {
    // Storage quota or disabled
  }
}

async function postTvScanner(url: string, body: Record<string, unknown>, signal?: AbortSignal): Promise<TvScanResponse | null> {
  const now = Date.now();
  if (now < rateLimitedUntil) {
    const waitSec = Math.ceil((rateLimitedUntil - now) / 1000);
    throw new Error(`Rate limit cooldown active. Please wait ${waitSec}s.`);
  }

  // If running on localhost, prefer Vite dev proxy to eliminate CORS completely
  const targetUrl =
    typeof window !== "undefined" && window.location.hostname === "localhost" && url.startsWith("https://scanner.tradingview.com")
      ? url.replace("https://scanner.tradingview.com", "/api/tv-scanner")
      : url;

  try {
    const res = await fetch(targetUrl, {
      method: "POST",
      headers: { "Content-Type": "text/plain;charset=UTF-8" },
      body: JSON.stringify(body),
      signal,
    });

    if (res.status === 429) {
      rateLimitedUntil = Date.now() + 60 * 1000;
      throw new Error("Rate limit reached. Cooling down for 60 seconds.");
    }

    if (res.ok) {
      return (await res.json().catch(() => null)) as TvScanResponse | null;
    }
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") throw err;
    if (err instanceof Error && err.message.includes("Rate limit")) throw err;
  }

  // Fallback to direct URL with text/plain (CORS safelisted header, no preflight) if proxy failed
  if (targetUrl !== url) {
    try {
      const directRes = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "text/plain;charset=UTF-8" },
        body: JSON.stringify(body),
        signal,
      });

      if (directRes.status === 429) {
        rateLimitedUntil = Date.now() + 60 * 1000;
        throw new Error("Rate limit reached. Cooling down for 60 seconds.");
      }

      if (directRes.ok) {
        return (await directRes.json().catch(() => null)) as TvScanResponse | null;
      }
    } catch (err) {
      if (err instanceof DOMException && err.name === "AbortError") throw err;
      if (err instanceof Error && err.message.includes("Rate limit")) throw err;
    }
  }

  return null;
}

/**
 * Searches and fetches real-time quote for a symbol.
 */
export async function fetchStockQuote(symbolQuery: string, signal?: AbortSignal, force = false): Promise<StockQuoteData> {
  const sym = symbolQuery.trim().toUpperCase();
  if (!sym) {
    throw new Error("Stock symbol is required.");
  }

  const cleanSym = sym.includes(":") ? sym : sym.replace(/[\/\-_]/g, "").trim();

  if (!force) {
    const cached = getCachedQuote(sym) || (cleanSym !== sym ? getCachedQuote(cleanSym) : null);
    if (cached) return cached;
  }

  // Case 1: Symbol has explicit exchange prefix, e.g. "NASDAQ:AAPL" or "BINANCE:BTCUSDT" or "FX:EURUSD"
  if (sym.includes(":")) {
    const resp = await postTvScanner(
      "https://scanner.tradingview.com/global/scan",
      {
        symbols: { tickers: [sym] },
        columns: COLUMNS,
      },
      signal
    );
    if (resp?.data && resp.data.length > 0) {
      const quote = parseTvRow(resp.data[0], sym);
      setCachedQuote(sym, quote);
      return quote;
    }
    throw new Error(`Symbol "${sym}" not found.`);
  }

  // Case 2: Try US America scanner
  const usResp = await postTvScanner(
    "https://scanner.tradingview.com/america/scan",
    {
      filter: [{ left: "name", operation: "equal", right: cleanSym }],
      columns: COLUMNS,
    },
    signal
  );
  if (usResp?.data && usResp.data.length > 0) {
    const quote = parseTvRow(usResp.data[0], cleanSym);
    setCachedQuote(sym, quote);
    if (cleanSym !== sym) setCachedQuote(cleanSym, quote);
    return quote;
  }

  // Case 3: Try Crypto scanner
  const cryptoResp = await postTvScanner(
    "https://scanner.tradingview.com/crypto/scan",
    {
      filter: [{ left: "name", operation: "equal", right: cleanSym }],
      columns: COLUMNS,
    },
    signal
  );
  if (cryptoResp?.data && cryptoResp.data.length > 0) {
    const quote = parseTvRow(cryptoResp.data[0], cleanSym);
    setCachedQuote(sym, quote);
    if (cleanSym !== sym) setCachedQuote(cleanSym, quote);
    return quote;
  }

  // Case 4: Try India scanner
  const inResp = await postTvScanner(
    "https://scanner.tradingview.com/india/scan",
    {
      filter: [{ left: "name", operation: "equal", right: cleanSym }],
      columns: COLUMNS,
    },
    signal
  );
  if (inResp?.data && inResp.data.length > 0) {
    const quote = parseTvRow(inResp.data[0], cleanSym);
    setCachedQuote(sym, quote);
    if (cleanSym !== sym) setCachedQuote(cleanSym, quote);
    return quote;
  }

  // Case 5: Try Forex / Currency scanner (e.g. EURUSD, USDINR, GBPUSD, USDJPY)
  const forexResp = await postTvScanner(
    "https://scanner.tradingview.com/forex/scan",
    {
      filter: [{ left: "name", operation: "equal", right: cleanSym }],
      columns: COLUMNS,
    },
    signal
  );
  if (forexResp?.data && forexResp.data.length > 0) {
    const quote = parseTvRow(forexResp.data[0], cleanSym);
    setCachedQuote(sym, quote);
    if (cleanSym !== sym) setCachedQuote(cleanSym, quote);
    return quote;
  }

  throw new Error(`Symbol "${sym}" not found.`);
}

/** Formats quote price with adaptive precision (4 decimals for < 10, 2 decimals for >= 10) */
export function formatQuotePrice(price?: number): string {
  if (price === undefined || price === null || !Number.isFinite(price)) return "--";
  if (Math.abs(price) < 10) {
    return price.toFixed(4);
  }
  return price.toFixed(2);
}

/** Formats quote change with adaptive precision and sign */
export function formatQuoteChange(change?: number): string {
  if (change === undefined || change === null || !Number.isFinite(change)) return "--";
  const sign = change >= 0 ? "+" : "";
  if (Math.abs(change) < 10) {
    return `${sign}${change.toFixed(4)}`;
  }
  return `${sign}${change.toFixed(2)}`;
}

/** Formats a compact volume or market cap (e.g. 21.05M, 3.42T) */
export function formatCompactNumber(num?: number): string {
  if (num === undefined || num === null || !Number.isFinite(num) || num === 0) return "--";
  const abs = Math.abs(num);
  if (abs >= 1e12) return `${(num / 1e12).toFixed(2)}T`;
  if (abs >= 1e9) return `${(num / 1e9).toFixed(2)}B`;
  if (abs >= 1e6) return `${(num / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `${(num / 1e3).toFixed(1)}K`;
  return num.toLocaleString();
}

/**
 * Batches fetching quotes for multiple symbols concurrently.
 * Returns a map of symbol -> StockQuoteData.
 */
export async function fetchMultipleStockQuotes(
  symbols: string[],
  signal?: AbortSignal,
  force = false
): Promise<Record<string, StockQuoteData>> {
  const result: Record<string, StockQuoteData> = {};
  if (!symbols || symbols.length === 0) return result;

  const tasks = symbols.map(async (sym) => {
    try {
      const q = await fetchStockQuote(sym, signal, force);
      result[sym.trim().toUpperCase()] = q;
    } catch {
      // Individual quote error doesn't break other tiles
    }
  });

  await Promise.allSettled(tasks);
  return result;
}

