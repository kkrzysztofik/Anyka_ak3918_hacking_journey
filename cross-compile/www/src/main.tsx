/**
 * @file Main entry point for the application
 * @description Application entry point with fully embedded React and all dependencies
 */
import React from 'react';

import { createRoot } from 'react-dom/client';

import App from './App';

// Initialize the application
const container = document.getElementById('root');
if (container) {
  const root = createRoot(container);
  root.render(React.createElement(App));
} else {
  console.error('Root container not found');
}
