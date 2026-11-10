import { describe, expect, it } from 'vitest';
import * as React from 'react';
import { renderToString } from 'react-dom/server';
import { Tooltip, TooltipContent, TooltipTrigger } from './tooltip';

describe('Tooltip', () => {
  it('renders without an external TooltipProvider', () => {
    // Regression guard for the runtime error "`Tooltip` must be used within
    // `TooltipProvider`": the raw Radix Root throws that error unless a
    // provider is an ancestor, and the site header renders the theme toggle
    // (a Tooltip consumer) on every page without mounting one. The wrapper
    // therefore embeds its own provider so every usage is standalone.
    const html = renderToString(
      React.createElement(
        Tooltip,
        null,
        React.createElement(
          TooltipTrigger,
          { asChild: true },
          React.createElement('button', { type: 'button' }, 'toggle theme'),
        ),
        React.createElement(TooltipContent, null, 'Light theme'),
      ),
    );
    expect(html).toContain('toggle theme');
  });
});
