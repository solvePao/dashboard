/**
 * Mobile Stock API client: fetches real-time quotes via TradingView Scanner
 * across global exchanges, including India (NSE, BSE) and New York Stock Exchange (NYSE),
 * with automatic caching (5-minute TTL) and rate limit safety.
 */

export type StockPeriod = "1D" | "5D" | "1W" | "1M" | "3M" | "6M" | "YTD" | "1Y";

export type MarketFilter = "all" | "india" | "nyse" | "us" | "crypto";

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
  exchange?: string;
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

export interface StockSearchResult {
  symbol: string;
  ticker: string;
  exchange?: string;
  name: string;
  price: number;
  change: number;
  changePercent: number;
  volume?: number;
  currency: string;
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

const CACHE_PREFIX = "mo_stock_cache_v3_";
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

function deriveCurrency(rowCurrency: unknown, exchange?: string): string {
  if (typeof rowCurrency === "string" && rowCurrency.trim() !== "") {
    return rowCurrency.trim().toUpperCase();
  }
  if (exchange === "NSE" || exchange === "BSE") return "INR";
  if (exchange === "NYSE" || exchange === "NASDAQ" || exchange === "AMEX") return "USD";
  return "USD";
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

  const [exchPrefix] = row.s.includes(":") ? row.s.split(":") : [""];
  const exchange = exchPrefix ? exchPrefix.toUpperCase() : undefined;
  const inferredCurrency = deriveCurrency(currency, exchange);

  const baseSymbol = (typeof name === "string" && name) ||
    (querySymbol.includes(":") ? querySymbol.split(":")[1] : querySymbol).toUpperCase();

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
    symbol: baseSymbol,
    ticker: row.s || querySymbol.toUpperCase(),
    exchange,
    name: (typeof description === "string" && description) || baseSymbol,
    price,
    change: changeVal,
    changePercent: asNum(change),
    dayHigh: asNum(high, price),
    dayLow: asNum(low, price),
    open: asNum(open, price),
    prevClose: Number.isFinite(prevClose) ? prevClose : price,
    volume: asNum(volume),
    marketCap: typeof marketCap === "number" && Number.isFinite(marketCap) ? marketCap : undefined,
    currency: inferredCurrency,
    lastUpdated,
    perf,
  };
}

function parseTvSearchResult(row: TvScanRow): StockSearchResult {
  const [name, description, close, change, changeAbs, volume, , , , , currency] = row.d;
  const [exchPrefix] = row.s.includes(":") ? row.s.split(":") : [""];
  const exchange = exchPrefix ? exchPrefix.toUpperCase() : undefined;
  const inferredCurrency = deriveCurrency(currency, exchange);

  const baseSymbol = (typeof name === "string" && name) ||
    (row.s.includes(":") ? row.s.split(":")[1] : row.s);

  return {
    symbol: baseSymbol,
    ticker: row.s,
    exchange,
    name: (typeof description === "string" && description) || baseSymbol,
    price: asNum(close),
    change: asNum(changeAbs),
    changePercent: asNum(change),
    volume: asNullableNum(volume),
    currency: inferredCurrency,
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

  // Case 1: Symbol has explicit exchange prefix, e.g. "NYSE:KO", "NSE:RELIANCE", "BSE:SENSEX"
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

  // Case 2: Try US America scanner (covers NYSE, NASDAQ, AMEX)
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

  // Case 4: Try India scanner (covers NSE, BSE)
  const inResp = await postTvScanner(
    "https://scanner.tradingview.com/india/scan",
    {
      filter: [{ left: "name", operation: "equal", right: cleanSym }],
      columns: COLUMNS,
    },
    signal
  );
  if (inResp?.data && inResp.data.length > 0) {
    const nseRow = inResp.data.find((r) => r.s.startsWith("NSE:")) || inResp.data[0];
    const quote = parseTvRow(nseRow, cleanSym);
    setCachedQuote(sym, quote);
    if (cleanSym !== sym) setCachedQuote(cleanSym, quote);
    return quote;
  }

  // Case 5: Try Forex / Currency scanner
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

/**
 * Searches symbols scoped by market filter:
 * - "india": NSE and BSE equities and indices
 * - "nyse": New York Stock Exchange equities and indices
 * - "us": All US markets (NYSE, NASDAQ, AMEX)
 * - "crypto": Major cryptocurrencies
 * - "all": Unified cross-market search
 */
export async function searchStockSymbols(
  query: string,
  marketFilter: MarketFilter = "all",
  signal?: AbortSignal
): Promise<StockSearchResult[]> {
  const trimmed = query.trim().toUpperCase();
  if (!trimmed) return [];

  // If query has explicit exchange (e.g. "NSE:RELIANCE", "NYSE:KO"), use global/scan
  if (trimmed.includes(":")) {
    const resp = await postTvScanner(
      "https://scanner.tradingview.com/global/scan",
      {
        symbols: { tickers: [trimmed] },
        columns: COLUMNS,
      },
      signal
    );
    if (resp?.data && resp.data.length > 0) {
      return resp.data.map(parseTvSearchResult);
    }
    return [];
  }

  const cleanSym = trimmed.replace(/[\/\-_]/g, "").trim();

  if (marketFilter === "india") {
    const resp = await postTvScanner(
      "https://scanner.tradingview.com/india/scan",
      {
        filter: [{ left: "name,description", operation: "match", right: cleanSym }],
        sort: { sortBy: "volume", sortOrder: "desc" },
        columns: COLUMNS,
        range: [0, 10],
      },
      signal
    );
    return (resp?.data || []).map(parseTvSearchResult);
  }

  if (marketFilter === "nyse") {
    const resp = await postTvScanner(
      "https://scanner.tradingview.com/america/scan",
      {
        filter: [
          { left: "exchange", operation: "equal", right: "NYSE" },
          { left: "name,description", operation: "match", right: cleanSym },
        ],
        sort: { sortBy: "volume", sortOrder: "desc" },
        columns: COLUMNS,
        range: [0, 10],
      },
      signal
    );
    return (resp?.data || []).map(parseTvSearchResult);
  }

  if (marketFilter === "us") {
    const resp = await postTvScanner(
      "https://scanner.tradingview.com/america/scan",
      {
        filter: [{ left: "name,description", operation: "match", right: cleanSym }],
        sort: { sortBy: "volume", sortOrder: "desc" },
        columns: COLUMNS,
        range: [0, 10],
      },
      signal
    );
    return (resp?.data || []).map(parseTvSearchResult);
  }

  if (marketFilter === "crypto") {
    const resp = await postTvScanner(
      "https://scanner.tradingview.com/crypto/scan",
      {
        filter: [{ left: "name,description", operation: "match", right: cleanSym }],
        sort: { sortBy: "volume", sortOrder: "desc" },
        columns: COLUMNS,
        range: [0, 10],
      },
      signal
    );
    return (resp?.data || []).map(parseTvSearchResult);
  }

  // Market filter "all": Parallel query to America, India, and Crypto
  const [usResp, inResp, crResp] = await Promise.allSettled([
    postTvScanner(
      "https://scanner.tradingview.com/america/scan",
      {
        filter: [{ left: "name,description", operation: "match", right: cleanSym }],
        sort: { sortBy: "volume", sortOrder: "desc" },
        columns: COLUMNS,
        range: [0, 6],
      },
      signal
    ),
    postTvScanner(
      "https://scanner.tradingview.com/india/scan",
      {
        filter: [{ left: "name,description", operation: "match", right: cleanSym }],
        sort: { sortBy: "volume", sortOrder: "desc" },
        columns: COLUMNS,
        range: [0, 6],
      },
      signal
    ),
    postTvScanner(
      "https://scanner.tradingview.com/crypto/scan",
      {
        filter: [{ left: "name,description", operation: "match", right: cleanSym }],
        sort: { sortBy: "volume", sortOrder: "desc" },
        columns: COLUMNS,
        range: [0, 4],
      },
      signal
    ),
  ]);

  const rawRows: TvScanRow[] = [];
  if (usResp.status === "fulfilled" && usResp.value?.data) rawRows.push(...usResp.value.data);
  if (inResp.status === "fulfilled" && inResp.value?.data) rawRows.push(...inResp.value.data);
  if (crResp.status === "fulfilled" && crResp.value?.data) rawRows.push(...crResp.value.data);

  const seenTickers = new Set<string>();
  const parsed = rawRows
    .filter((r) => {
      if (seenTickers.has(r.s)) return false;
      seenTickers.add(r.s);
      return true;
    })
    .map(parseTvSearchResult);

  parsed.sort((a, b) => {
    const aExact = a.symbol.toUpperCase() === cleanSym || a.ticker.toUpperCase() === cleanSym ? 1 : 0;
    const bExact = b.symbol.toUpperCase() === cleanSym || b.ticker.toUpperCase() === cleanSym ? 1 : 0;
    return bExact - aExact;
  });

  return parsed;
}

/** Formats quote price with locale comma thousands separator and adaptive precision */
export function formatQuotePrice(price?: number): string {
  if (price === undefined || price === null || !Number.isFinite(price)) return "--";
  if (Math.abs(price) < 10) {
    return price.toFixed(4);
  }
  return price.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

/** Formats quote change with sign and locale thousands separator */
export function formatQuoteChange(change?: number): string {
  if (change === undefined || change === null || !Number.isFinite(change)) return "--";
  const sign = change > 0 ? "+" : change < 0 ? "-" : "";
  const abs = Math.abs(change);
  if (abs < 10) {
    return `${sign}${abs.toFixed(4)}`;
  }
  const formatted = abs.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  return `${sign}${formatted}`;
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
 * Batches fetching quotes for multiple symbols with fast single-request
 * global/scan query for prefixed tickers (e.g. NYSE:KO, NSE:RELIANCE).
 */
export async function fetchMultipleStockQuotes(
  symbols: string[],
  signal?: AbortSignal,
  force = false
): Promise<Record<string, StockQuoteData>> {
  const result: Record<string, StockQuoteData> = {};
  if (!symbols || symbols.length === 0) return result;

  const prefixedTickers: string[] = [];
  const unprefixedSymbols: string[] = [];

  for (const s of symbols) {
    const sym = s.trim().toUpperCase();
    if (!sym) continue;

    if (!force) {
      const cached = getCachedQuote(sym);
      if (cached) {
        result[sym] = cached;
        continue;
      }
    }

    if (sym.includes(":")) {
      prefixedTickers.push(sym);
    } else {
      unprefixedSymbols.push(sym);
    }
  }

  // Fast batch query for all prefixed tickers in a single global/scan request
  if (prefixedTickers.length > 0) {
    try {
      const batchResp = await postTvScanner(
        "https://scanner.tradingview.com/global/scan",
        {
          symbols: { tickers: prefixedTickers },
          columns: COLUMNS,
        },
        signal
      );
      if (batchResp?.data) {
        for (const row of batchResp.data) {
          const q = parseTvRow(row, row.s);
          result[row.s.toUpperCase()] = q;
          setCachedQuote(row.s, q);
          if (q.symbol) {
            setCachedQuote(q.symbol, q);
          }
        }
      }
    } catch {
      // Fallback to individual fetches below
    }
  }

  // Fetch any missing prefixed tickers or unprefixed symbols
  const remaining = [
    ...unprefixedSymbols,
    ...prefixedTickers.filter((t) => !result[t]),
  ];

  if (remaining.length > 0) {
    const tasks = remaining.map(async (sym) => {
      try {
        const q = await fetchStockQuote(sym, signal, force);
        result[sym] = q;
      } catch {
        // Individual quote error doesn't break other tiles
      }
    });
    await Promise.allSettled(tasks);
  }

  return result;
}
