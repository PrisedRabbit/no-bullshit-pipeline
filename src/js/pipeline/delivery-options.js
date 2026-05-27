// Auto-naming logic for pipeline editor \u2014 keyed off the new Connection model
// (`connection_type` on each step). Processing types are the local
// transforms (CLI agent, Shell); the rest are delivery destinations.

import * as pipelineState from './state.js';

const PROCESSING_TYPES = new Set(['cli_agent', 'shell', 'llm']);

export function suggestPipelineName() {
  const processing = [];
  const delivery = [];
  for (const step of pipelineState.pipelineEditorSteps) {
    const typeKey = step.connection_type || '';
    if (PROCESSING_TYPES.has(typeKey)) {
      const titleCased = (step.name || 'Untitled')
        .replace(/[-_]/g, ' ')
        .replace(/\b\w/g, c => c.toUpperCase());
      processing.push(titleCased);
    } else {
      const label = typeKey
        ? typeKey.charAt(0).toUpperCase() + typeKey.slice(1).replace(/_/g, ' ')
        : 'Unknown';
      delivery.push(label);
    }
  }
  if (processing.length === 0 && delivery.length === 0) return '';
  const parts = [];
  if (processing.length) parts.push(processing.join(', '));
  if (delivery.length) parts.push(delivery.join(', '));
  return parts.join(' \u2192 ');
}

export function maybeAutoName() {
  const nameEl = document.getElementById('pipeline-editor-name');
  if (!nameEl) return;
  const currentVal = nameEl.value.trim();
  if (currentVal === '' || currentVal === pipelineState.lastAutoName) {
    const suggested = suggestPipelineName();
    nameEl.value = suggested;
    pipelineState.setLastAutoName(suggested);
  }
}
