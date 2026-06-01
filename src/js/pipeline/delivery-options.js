// Auto-naming for the pipeline editor. The chain is a linear sequence of
// local processing steps (CLI agent / Shell); the suggested pipeline name is
// just the step names joined in order.

import * as pipelineState from './state.js';

export function suggestPipelineName() {
  const names = pipelineState.pipelineEditorSteps
    .map(step => (step.name || 'Untitled')
      .replace(/[-_]/g, ' ')
      .replace(/\b\w/g, c => c.toUpperCase()));
  return names.join(' → ');
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
