# Hand-written SVG charts, not a chart library

The PRD planned "a JS chart library" for the dashboard. The charts are a single series of 7 to 31 bars (fewer for Year and All), styled after Apple Health: rounded bar tops anchored to the baseline, one accent colour, recessive dashed grid lines, labels on the right, light and dark themes from the CSS variables in `app.css`. We draw them as an SVG Svelte component (`src/lib/BarChart.svelte`) instead: it is about a hundred lines, has no dependency, follows the theme variables directly, and gives full control of the look, whereas a canvas library would need its own theming and fighting its defaults for every detail.

## Consequences

- Hover, keyboard access and click targets (selecting a bar, next slices) are ours to write.
- If a later need goes beyond bars (zoomable time series, elevation profiles linked to the map), a library can be reconsidered for that chart only.
