/// Per-device widget contracts, mirrored from the web dashboard.
/// The backend stores `config` opaquely; each widget component owns
/// the interpretation of its own config shape.
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

export interface WeatherWidgetConfig {
  cities: WeatherCityItem[];
  city?: string;
  country?: string;
  admin1?: string;
  latitude?: number;
  longitude?: number;
}

/// Validates an opaque widget config into a weather config.
/// Returns `null` unless the widget is enabled and carries at least one
/// valid city with finite coordinates.
export function asWeatherConfig(widget: DeviceWidget | undefined): WeatherWidgetConfig | null {
  if (!widget || !widget.enabled) return null;
  if (typeof widget.config !== "object" || widget.config === null) return null;
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
            admin1: typeof c.admin1 === "string" ? c.admin1 : undefined,
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

export interface StockSymbolItem {
  symbol: string;
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
  if (!widget || !widget.enabled) return null;
  if (typeof widget.config !== "object" || widget.config === null) return null;
  const raw = widget.config as Record<string, unknown>;
  const validSymbols: StockSymbolItem[] = [];

  if (Array.isArray(raw.symbols)) {
    for (const item of raw.symbols) {
      if (typeof item === "object" && item !== null) {
        const s = item as Record<string, unknown>;
        if (typeof s.symbol === "string" && s.symbol.trim() !== "") {
          validSymbols.push({
            symbol: s.symbol.trim().toUpperCase(),
            name: typeof s.name === "string" ? s.name : undefined,
            exchange: typeof s.exchange === "string" ? s.exchange : undefined,
            currency: typeof s.currency === "string" ? s.currency : undefined,
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

