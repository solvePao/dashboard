/// Open-Meteo geocoding client: resolves a typed city name into
/// selectable options carrying the latitude/longitude the forecast
/// API needs. Kept behind this boundary so a proxy or key can be
/// added later without touching any component.
export interface CityOption {
  name: string;
  country?: string;
  admin1?: string;
  latitude: number;
  longitude: number;
  /** "Berlin, Germany" style display label. */
  label: string;
}

interface GeocodingResult {
  name?: string;
  country?: string;
  admin1?: string;
  latitude?: number;
  longitude?: number;
}

interface GeocodingResponse {
  results?: GeocodingResult[];
}

export async function searchCities(query: string, signal?: AbortSignal): Promise<CityOption[]> {
  const trimmed = query.trim();
  if (trimmed.length < 2) return [];
  const url =
    `https://geocoding-api.open-meteo.com/v1/search` +
    `?name=${encodeURIComponent(trimmed)}&count=5&language=en&format=json`;
  const res = await fetch(url, { signal });
  if (!res.ok) {
    throw new Error("City search failed. Please try again.");
  }
  const body = (await res.json().catch(() => null)) as GeocodingResponse | null;
  const results = Array.isArray(body?.results) ? body.results : [];
  const options: CityOption[] = [];
  for (const item of results) {
    if (
      typeof item.name !== "string" ||
      typeof item.latitude !== "number" ||
      !Number.isFinite(item.latitude) ||
      typeof item.longitude !== "number" ||
      !Number.isFinite(item.longitude)
    ) {
      continue;
    }
    const place = [item.name, item.country].filter(Boolean).join(", ");
    options.push({
      name: item.name,
      country: item.country,
      admin1: item.admin1,
      latitude: item.latitude,
      longitude: item.longitude,
      label: place || item.name,
    });
  }
  return options;
}
