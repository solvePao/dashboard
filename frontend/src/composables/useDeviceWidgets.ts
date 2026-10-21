import { onScopeDispose, ref } from "vue";
import { logger } from "../lib/logger";
import { ApiError } from "../api/client";
import { fetchWidgetsApi, saveWidgetsApi } from "../api/widgetApi";
import { subscribeHubEvents } from "./useDevices";
import { useAuth } from "./useAuth";
import type { DeviceWidget, StockPeriod, StockSymbolItem, WeatherCityItem, WeatherWidgetConfig } from "../types/widget";
import { asStockConfig, asWeatherConfig } from "../types/widget";

/**
 * Reactive controller for one device's widget set inside the Manage
 * sheet. Every mutation saves the complete set immediately (PUT
 * replace semantics), so there is no draft state to lose. Subscribes
 * to hub `widgets_updated` events so a second tab's save refreshes
 * this sheet live.
 */
export function useDeviceWidgets(deviceToken: string) {
  const { authToken } = useAuth();
  const widgets = ref<DeviceWidget[]>([]);
  const widgetsLoading = ref(false);
  const widgetsError = ref<string | null>(null);
  const widgetsNotice = ref<string | null>(null);
  const saving = ref(false);

  async function load(): Promise<void> {
    const token = authToken.value;
    if (!token) {
      widgetsError.value = "Please sign in to manage widgets.";
      return;
    }
    // Keep mounted widget settings and their confirmation state during background refreshes.
    widgetsLoading.value = widgets.value.length === 0;
    widgetsError.value = null;
    try {
      widgets.value = await fetchWidgetsApi(token, deviceToken);
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) throw err;
      widgetsError.value = err instanceof Error ? err.message : "Failed to load widgets.";
      logger.error("widgets", "failed to fetch widgets", err);
    } finally {
      widgetsLoading.value = false;
    }
  }

  async function persist(next: DeviceWidget[]): Promise<boolean> {
    if (saving.value) return false;
    const token = authToken.value;
    if (!token) {
      widgetsError.value = "Please sign in to manage widgets.";
      return false;
    }
    saving.value = true;
    widgetsError.value = null;
    try {
      const result = await saveWidgetsApi(token, deviceToken, next);
      widgets.value = result.widgets;
      widgetsNotice.value = result.cleanup_pending ? "Widget saved. Previous audio connections are still being closed; the server is retrying." : null;
      return true;
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) throw err;
      widgetsError.value = err instanceof Error ? err.message : "Failed to save widgets.";
      logger.error("widgets", "failed to save widgets", err);
      return false;
    } finally {
      saving.value = false;
    }
  }

  /** Adds or replaces the weather widget with the picked city (or cities). */
  async function setWeatherCity(config: WeatherWidgetConfig): Promise<boolean> {
    const next = widgets.value.filter((w) => w.widget_key !== "weather");
    const cities = config.cities && config.cities.length > 0
      ? config.cities
      : config.city && config.latitude !== undefined && config.longitude !== undefined
      ? [{
          city: config.city,
          country: config.country,
          admin1: config.admin1,
          latitude: config.latitude,
          longitude: config.longitude,
        }]
      : [];

    next.push({
      widget_key: "weather",
      enabled: true,
      config: { cities },
    });
    return persist(next);
  }

  /** Appends a city to the device's weather widget cities list. */
  async function addWeatherCity(city: WeatherCityItem): Promise<boolean> {
    const existing = widgets.value.find((w) => w.widget_key === "weather");
    const parsed = existing ? asWeatherConfig(existing) : null;
    const currentCities = parsed ? [...parsed.cities] : [];

    // Check if city already exists (by lat/lon or city name)
    const duplicate = currentCities.some(
      (c) =>
        (Math.abs(c.latitude - city.latitude) < 0.001 && Math.abs(c.longitude - city.longitude) < 0.001) ||
        (c.city.toLowerCase() === city.city.toLowerCase() && c.country === city.country)
    );
    if (!duplicate) {
      currentCities.push(city);
    }

    const next = widgets.value.filter((w) => w.widget_key !== "weather");
    next.push({
      widget_key: "weather",
      enabled: existing ? existing.enabled : true,
      config: { cities: currentCities },
    });
    return persist(next);
  }

  /** Removes a city at the given index from the weather widget. */
  async function removeWeatherCity(index: number): Promise<boolean> {
    const existing = widgets.value.find((w) => w.widget_key === "weather");
    if (!existing) return true;
    const parsed = asWeatherConfig(existing);
    const currentCities = parsed ? [...parsed.cities] : [];
    if (index >= 0 && index < currentCities.length) {
      currentCities.splice(index, 1);
    }

    const next = widgets.value.filter((w) => w.widget_key !== "weather");
    next.push({
      widget_key: "weather",
      enabled: existing.enabled,
      config: { cities: currentCities },
    });
    return persist(next);
  }

  /** Appends a stock symbol to the device's stock widget symbols list. */
  async function addStockSymbol(symbolItem: StockSymbolItem): Promise<boolean> {
    const existing = widgets.value.find((w) => w.widget_key === "stock");
    const parsed = existing ? asStockConfig(existing) : null;
    const currentSymbols = parsed ? [...parsed.symbols] : [];

    const symUpper = symbolItem.symbol.trim().toUpperCase();
    const tickerUpper = symbolItem.ticker?.trim().toUpperCase();
    const exchangeUpper = symbolItem.exchange?.trim().toUpperCase();

    const uniqueId = tickerUpper || (exchangeUpper ? `${exchangeUpper}:${symUpper}` : symUpper);

    const duplicate = currentSymbols.some((s) => {
      const sId = s.ticker || (s.exchange ? `${s.exchange}:${s.symbol}` : s.symbol);
      return sId.toUpperCase() === uniqueId.toUpperCase();
    });
    if (!duplicate) {
      if (currentSymbols.length >= 4) {
        widgetsError.value = "Maximum 4 stock symbols allowed per device.";
        return false;
      }
      currentSymbols.push({
        ...symbolItem,
        symbol: symUpper,
        ticker: tickerUpper || (exchangeUpper ? `${exchangeUpper}:${symUpper}` : undefined),
        exchange: exchangeUpper,
        currency: symbolItem.currency?.trim().toUpperCase(),
      });
    }

    const next = widgets.value.filter((w) => w.widget_key !== "stock");
    next.push({
      widget_key: "stock",
      enabled: existing ? existing.enabled : true,
      config: {
        symbols: currentSymbols,
        period: parsed?.period,
      },
    });
    return persist(next);
  }

  /** Removes a stock symbol at the given index. */
  async function removeStockSymbol(index: number): Promise<boolean> {
    const existing = widgets.value.find((w) => w.widget_key === "stock");
    if (!existing) return true;
    const parsed = asStockConfig(existing);
    const currentSymbols = parsed ? [...parsed.symbols] : [];
    if (index >= 0 && index < currentSymbols.length) {
      currentSymbols.splice(index, 1);
    }

    const next = widgets.value.filter((w) => w.widget_key !== "stock");
    next.push({
      widget_key: "stock",
      enabled: existing.enabled,
      config: {
        symbols: currentSymbols,
        period: parsed?.period,
      },
    });
    return persist(next);
  }

  /** Sets the default performance period (e.g. 1D, 5D, 1M, etc.) for the stock widget. */
  async function setStockPeriod(period: StockPeriod): Promise<boolean> {
    const existing = widgets.value.find((w) => w.widget_key === "stock");
    const parsed = existing ? asStockConfig(existing) : null;
    const currentSymbols = parsed ? [...parsed.symbols] : [];

    const next = widgets.value.filter((w) => w.widget_key !== "stock");
    next.push({
      widget_key: "stock",
      enabled: existing ? existing.enabled : true,
      config: {
        symbols: currentSymbols,
        period,
      },
    });
    return persist(next);
  }

  async function setWidgetEnabled(key: string, enabled: boolean): Promise<boolean> {
    const next = widgets.value.map((w) =>
      w.widget_key === key ? { ...w, enabled } : w
    );
    return persist(next);
  }

  async function removeWidget(key: string): Promise<boolean> {
    return persist(widgets.value.filter((w) => w.widget_key !== key));
  }

  async function setWalkieTalkieGroup(groupId: string | null): Promise<boolean> {
    const next = widgets.value.filter(w => w.widget_key !== "walkie_talkie");
    next.push({ widget_key: "walkie_talkie", enabled: true, config: { group_id: groupId } });
    return persist(next);
  }

  /** Clears screen widgets (e.g. weather, stock) while preserving phone features like walkie_talkie. */
  async function clearScreenWidgets(): Promise<boolean> {
    const next = widgets.value.filter(w => w.widget_key === "walkie_talkie");
    return persist(next);
  }

  /** Clears every widget (the dropdown's "None" choice). */
  async function clearWidgets(): Promise<boolean> {
    return persist([]);
  }

  const unsubscribe = subscribeHubEvents((event) => {
    if (event.kind === "widgets_updated" && event.token === deviceToken) {
      void load().catch(() => undefined);
    }
  });
  onScopeDispose(unsubscribe);

  return {
    widgets,
    widgetsLoading,
    widgetsError,
    widgetsNotice,
    saving,
    load,
    setWeatherCity,
    addWeatherCity,
    removeWeatherCity,
    addStockSymbol,
    removeStockSymbol,
    setStockPeriod,
    setWidgetEnabled,
    removeWidget,
    clearWidgets,
    clearScreenWidgets,
    setWalkieTalkieGroup,
  };
}
