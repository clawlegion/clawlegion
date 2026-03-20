import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";

import { App } from "./App";
import { BackendEndpointProvider } from "./components/backend-endpoint-provider";
import { I18nProvider } from "./i18n";
import { queryClient } from "./query-client";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <I18nProvider>
      <QueryClientProvider client={queryClient}>
        <BackendEndpointProvider>
          <App />
        </BackendEndpointProvider>
      </QueryClientProvider>
    </I18nProvider>
  </React.StrictMode>,
);
