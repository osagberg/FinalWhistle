/*
 * ECharts wrapper. T0-2 ships an empty hook; real charts ship at T4-2.
 *
 * Lazy-init pattern: ECharts is heavy (~280kb min), and most routes don't
 * need it. The Match + Squad detail pages own the imports; this wrapper is
 * a thin shell so callers don't talk to ECharts directly.
 */

import * as echarts from "echarts/core";
import { LineChart, BarChart, ScatterChart } from "echarts/charts";
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
  TitleComponent,
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import { createEffect, onCleanup, onMount, type JSX } from "solid-js";

echarts.use([
  LineChart,
  BarChart,
  ScatterChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  TitleComponent,
  CanvasRenderer,
]);

export interface StatProps {
  /** Raw ECharts option object. Future iterations may add typed presets. */
  option: echarts.EChartsCoreOption;
  /** Pixel height (default 240). Tailwind class for width is on the wrapper. */
  height?: number;
  class?: string;
}

export default function Stat(props: StatProps): JSX.Element {
  let host!: HTMLDivElement;
  let chart: echarts.ECharts | undefined;

  onMount(() => {
    chart = echarts.init(host, undefined, { renderer: "canvas" });

    // React to option changes (second arg true = notMerge, clears stale series).
    // This is what makes the sortable/filterable controls re-render the chart
    // when the user picks a different stat or club.
    createEffect(() => {
      chart?.setOption(props.option, true);
    });

    // Resize on window-resize. ECharts has no built-in observer.
    const resize = () => chart?.resize();
    window.addEventListener("resize", resize);
    onCleanup(() => window.removeEventListener("resize", resize));
  });

  onCleanup(() => {
    chart?.dispose();
    chart = undefined;
  });

  return (
    <div
      ref={(el) => {
        host = el;
      }}
      class={`fw-panel ${props.class ?? "w-full"}`}
      style={{ height: `${props.height ?? 240}px` }}
    />
  );
}
