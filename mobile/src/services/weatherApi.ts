/// Open-Meteo forecast client. The city search (geocoding) happens
/// on the web dashboard; the mobile only needs latitude/longitude,
/// which arrive inside the weather widget config. Kept behind this
/// boundary so a proxy or API key can be added without touching UI.
export type WeatherEffectType =
  | "sunlight"
  | "day_rain"
  | "night_rain"
  | "cloud"
  | "sandstorm"
  | "snow"
  | "chilly_night"
  | "thunderstorm";

export interface AirQualityData {
  aqi: number;
  category: string;
}

export interface CurrentWeather {
  temperature: number;
  apparentTemperature: number;
  humidity: number;
  windSpeed: number;
  windDirection: number;
  windCardinal: string;
  weatherCode: number;
  isDay: boolean;
  highToday: number;
  lowToday: number;
  precipitation: number;
  precipitationProbability: number;
  uvIndex: number;
  aqi: AirQualityData;
  pressure?: number;
  sunrise: string;
  sunset: string;
  lastUpdated: string;
}

interface ForecastResponse {
  current?: {
    temperature_2m?: number;
    relative_humidity_2m?: number;
    apparent_temperature?: number;
    is_day?: number;
    weather_code?: number;
    wind_speed_10m?: number;
    wind_direction_10m?: number;
    surface_pressure?: number;
    precipitation?: number;
  };
  daily?: {
    temperature_2m_max?: number[];
    temperature_2m_min?: number[];
    uv_index_max?: number[];
    precipitation_probability_max?: number[];
    sunrise?: string[];
    sunset?: string[];
  };
}

interface AirQualityResponse {
  current?: {
    us_aqi?: number;
  };
}

function asFinite(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function getAqiCategory(aqi: number): string {
  if (aqi <= 50) return "Good";
  if (aqi <= 100) return "Moderate";
  if (aqi <= 150) return "Unhealthy (Sens.)";
  if (aqi <= 200) return "Unhealthy";
  if (aqi <= 300) return "Very Unhealthy";
  return "Hazardous";
}

export function getWindCardinal(deg: number): string {
  const directions = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
  return directions[Math.round(((deg % 360) + 360) % 360 / 45) % 8] || "N";
}

function formatIsoTime(iso?: string): string {
  if (!iso) return "--:--";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    const parts = iso.split("T");
    return parts[1] || iso;
  }
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export async function fetchCurrentWeather(
  latitude: number,
  longitude: number,
  signal?: AbortSignal
): Promise<CurrentWeather> {
  const forecastUrl =
    `https://api.open-meteo.com/v1/forecast` +
    `?latitude=${latitude}&longitude=${longitude}` +
    `&current=temperature_2m,relative_humidity_2m,apparent_temperature,is_day,precipitation,weather_code,surface_pressure,wind_speed_10m,wind_direction_10m` +
    `&daily=temperature_2m_max,temperature_2m_min,uv_index_max,precipitation_probability_max,sunrise,sunset` +
    `&timezone=auto&forecast_days=1`;

  const aqiUrl =
    `https://air-quality-api.open-meteo.com/v1/air-quality` +
    `?latitude=${latitude}&longitude=${longitude}` +
    `&current=us_aqi`;

  const [forecastRes, aqiRes] = await Promise.all([
    fetch(forecastUrl, { signal }),
    fetch(aqiUrl, { signal }).catch(() => null),
  ]);

  if (!forecastRes.ok) {
    throw new Error("Weather service is unavailable. Please try again.");
  }
  const body = (await forecastRes.json().catch(() => null)) as ForecastResponse | null;
  const current = body?.current;
  const temperature = current ? asFinite(current.temperature_2m) : null;
  const code = current ? asFinite(current.weather_code) : null;
  if (temperature === null || code === null) {
    throw new Error("Weather service returned an unexpected response.");
  }

  let rawAqi = 0;
  if (aqiRes && aqiRes.ok) {
    const aqiBody = (await aqiRes.json().catch(() => null)) as AirQualityResponse | null;
    rawAqi = asFinite(aqiBody?.current?.us_aqi) ?? 0;
  }
  const roundedAqi = Math.round(rawAqi);
  const aqi: AirQualityData = {
    aqi: roundedAqi,
    category: getAqiCategory(roundedAqi),
  };

  const windDeg = asFinite(current?.wind_direction_10m) ?? 0;
  const now = new Date();
  const lastUpdated = now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

  return {
    temperature,
    apparentTemperature: asFinite(current?.apparent_temperature) ?? temperature,
    humidity: asFinite(current?.relative_humidity_2m) ?? 0,
    windSpeed: asFinite(current?.wind_speed_10m) ?? 0,
    windDirection: windDeg,
    windCardinal: getWindCardinal(windDeg),
    weatherCode: code,
    isDay: (current?.is_day ?? 1) === 1,
    highToday: asFinite(body?.daily?.temperature_2m_max?.[0]) ?? temperature,
    lowToday: asFinite(body?.daily?.temperature_2m_min?.[0]) ?? temperature,
    precipitation: asFinite(current?.precipitation) ?? 0,
    precipitationProbability: asFinite(body?.daily?.precipitation_probability_max?.[0]) ?? 0,
    uvIndex: asFinite(body?.daily?.uv_index_max?.[0]) ?? 0,
    aqi,
    pressure: asFinite(current?.surface_pressure) ?? 1013,
    sunrise: formatIsoTime(body?.daily?.sunrise?.[0]),
    sunset: formatIsoTime(body?.daily?.sunset?.[0]),
    lastUpdated,
  };
}
