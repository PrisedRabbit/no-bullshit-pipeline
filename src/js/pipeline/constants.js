// Pipeline constants: SVG icons + step-type metadata.
//
// The pipeline is a linear chain of self-contained steps — each step is a
// local processing primitive (CLI agent or Shell). No external delivery
// connectors: glue your own delivery with a Shell step (`curl`, `cp`, …).

export const CLI_SVG = `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>`;

export const SHELL_SVG = `<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>`;

export const SAVE_SVG = `<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>`;

// Keyed by StepType (snake_case to match Rust serde). Used by the pipeline
// chip renderer for the type-coloured tile.
export const CONNECTOR_META = {
  cli_agent:  { svg: CLI_SVG,   textColor: '#fff',     bgColor: 'rgba(99,102,241,0.9)' },
  shell:      { svg: SHELL_SVG, textColor: '#fff',     bgColor: 'rgba(82,82,91,0.9)' },
  save_local: { svg: SAVE_SVG,  textColor: '#10b981',  bgColor: 'rgba(16,185,129,0.15)' },
};
