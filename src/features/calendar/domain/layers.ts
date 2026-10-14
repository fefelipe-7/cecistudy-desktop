import type { LayerId, LayerTone } from "./types.ts";

export interface LayerDefinition {
  readonly id: LayerId;
  readonly label: string;
  readonly tone: LayerTone;
  /** `false` só para a camada do Google: item importado é leitura, não edição. */
  readonly editable: boolean;
}

export const LAYERS: Record<LayerId, LayerDefinition> = {
  faculdade: { id: "faculdade", label: "Faculdade", tone: 2, editable: true },
  estudos: { id: "estudos", label: "Estudos", tone: 5, editable: true },
  tcc: { id: "tcc", label: "TCC", tone: 4, editable: true },
  estagio: { id: "estagio", label: "Estágio", tone: 1, editable: true },
  conhecimento: { id: "conhecimento", label: "Conhecimento", tone: 3, editable: true },
  google: { id: "google", label: "Google Calendar", tone: "neutral", editable: false },
};

export const LAYER_TONE: Record<LayerId, LayerTone> = {
  faculdade: LAYERS.faculdade.tone,
  estudos: LAYERS.estudos.tone,
  tcc: LAYERS.tcc.tone,
  estagio: LAYERS.estagio.tone,
  conhecimento: LAYERS.conhecimento.tone,
  google: LAYERS.google.tone,
};

export const LAYER_IDS = Object.keys(LAYERS) as LayerId[];

/**
 * Classes written out one tone at a time on purpose. Tailwind scans the source for
 * complete class strings, so an interpolated `bg-chart-${tone}/12` produces no CSS
 * at all and the box renders unstyled.
 */
const TONE_CLASSES: Record<LayerTone, string> = {
  1: "border-chart-1/30 bg-chart-1/12 text-foreground",
  2: "border-chart-2/30 bg-chart-2/12 text-foreground",
  3: "border-chart-3/30 bg-chart-3/12 text-foreground",
  4: "border-chart-4/30 bg-chart-4/12 text-foreground",
  5: "border-chart-5/30 bg-chart-5/12 text-foreground",
  neutral: "border-dashed border-border/70 bg-muted/50 text-muted-foreground",
};

export function isLayerId(value: string): value is LayerId {
  return Object.hasOwn(LAYERS, value);
}

export function layerLabel(id: LayerId): string {
  return LAYERS[id].label;
}

export function layerTone(id: LayerId): LayerTone {
  return LAYER_TONE[id];
}

export function layerIsEditable(id: LayerId): boolean {
  return LAYERS[id].editable;
}

export function layerClass(tone: LayerTone): string {
  return TONE_CLASSES[tone];
}
