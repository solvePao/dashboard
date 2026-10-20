/// Per-device widget contracts, mirrored by the backend
/// `DeviceWidget` model and the mobile app's copy. The backend stores
/// `config` opaquely, so new widget types only extend these types and
/// the widget registry — never the API shape.
export interface DeviceWidget {
  widget_key: string;
  enabled: boolean;
  config: Record<string, unknown>;
}

export interface WidgetsResponse {
  widgets: DeviceWidget[];
}

export interface WeatherCityItem {
  city: string;
  country?: string;
  admin1?: string;
  latitude: number;
  longitude: number;
}

/// Weather widget config: a list of selected cities plus the coordinates the
/// Open-Meteo forecast API needs. The mobile app never geocodes; it
/// reads lat/long straight from here.
export interface WeatherWidgetConfig {
  cities: WeatherCityItem[];
  city?: string;
  country?: string;
  admin1?: string;
  latitude?: number;
  longitude?: number;
}

/// Validates an opaque widget config into a weather config.
/// Returns `null` when the widget is missing, disabled-shape-invalid,
/// or carries no valid cities — callers render nothing instead
/// of fetching with bad parameters.
export function asWeatherConfig(widget: DeviceWidget | undefined): WeatherWidgetConfig | null {
  if (!widget || typeof widget.config !== "object" || widget.config === null) return null;
  const raw = widget.config as Record<string, unknown>;
  const validCities: WeatherCityItem[] = [];

  if (Array.isArray(raw.cities)) {
    for (const item of raw.cities) {
      if (typeof item === "object" && item !== null) {
        const c = item as Record<string, unknown>;
        if (
          typeof c.city === "string" &&
          c.city.trim() !== "" &&
          typeof c.latitude === "number" &&
          Number.isFinite(c.latitude) &&
          typeof c.longitude === "number" &&
          Number.isFinite(c.longitude)
        ) {
          validCities.push({
            city: c.city.trim(),
            country: typeof c.country === "string" ? c.country : undefined,
            admin1: typeof c.admin1 === "string" ? admin1ToString(c.admin1) : undefined,
            latitude: c.latitude,
            longitude: c.longitude,
          });
        }
      }
    }
  }

  // Fallback to legacy top-level single city properties if cities array is empty
  if (validCities.length === 0) {
    const { city, country, admin1, latitude, longitude } = raw;
    if (
      typeof city === "string" &&
      city.trim() !== "" &&
      typeof latitude === "number" &&
      Number.isFinite(latitude) &&
      typeof longitude === "number" &&
      Number.isFinite(longitude)
    ) {
      validCities.push({
        city: city.trim(),
        country: typeof country === "string" ? country : undefined,
        admin1: typeof admin1 === "string" ? admin1 : undefined,
        latitude,
        longitude,
      });
    }
  }

  if (validCities.length === 0) return null;

  const primary = validCities[0];
  return {
    cities: validCities,
    city: primary.city,
    country: primary.country,
    admin1: primary.admin1,
    latitude: primary.latitude,
    longitude: primary.longitude,
  };
}

function admin1ToString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

export interface StockSymbolItem {
  symbol: string;
  ticker?: string;
  name?: string;
  exchange?: string;
  currency?: string;
}

export interface StockWidgetConfig {
  symbols: StockSymbolItem[];
  symbol?: string;
  name?: string;
}

export function asStockConfig(widget: DeviceWidget | undefined): StockWidgetConfig | null {
  if (!widget || typeof widget.config !== "object" || widget.config === null) return null;
  const raw = widget.config as Record<string, unknown>;
  const validSymbols: StockSymbolItem[] = [];

  if (Array.isArray(raw.symbols)) {
    for (const item of raw.symbols) {
      if (typeof item === "object" && item !== null) {
        const s = item as Record<string, unknown>;
        if (typeof s.symbol === "string" && s.symbol.trim() !== "") {
          validSymbols.push({
            symbol: s.symbol.trim().toUpperCase(),
            ticker: typeof s.ticker === "string" ? s.ticker.trim().toUpperCase() : undefined,
            name: typeof s.name === "string" ? s.name : undefined,
            exchange: typeof s.exchange === "string" ? s.exchange.trim().toUpperCase() : undefined,
            currency: typeof s.currency === "string" ? s.currency.trim().toUpperCase() : undefined,
          });
        }
      }
    }
  }

  // Fallback to single symbol if symbols array is missing
  if (validSymbols.length === 0) {
    if (typeof raw.symbol === "string" && raw.symbol.trim() !== "") {
      validSymbols.push({
        symbol: raw.symbol.trim().toUpperCase(),
        name: typeof raw.name === "string" ? raw.name : undefined,
      });
    }
  }

  if (validSymbols.length === 0) return null;

  return {
    symbols: validSymbols,
    symbol: validSymbols[0].symbol,
    name: validSymbols[0].name,
  };
}

