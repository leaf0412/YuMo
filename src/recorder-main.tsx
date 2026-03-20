import React from 'react';
import ReactDOM from 'react-dom/client';
import RecorderFloat from './windows/RecorderFloat';
import { initI18n } from './i18n';

initI18n().then(() => {
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <RecorderFloat />
    </React.StrictMode>
  );
});
