import {
  createContext,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { useQueryClient } from "@tanstack/react-query";

import {
  clearApiBaseUrl,
  getDefaultApiBaseUrl,
  getEffectiveApiBaseUrl,
  hasStoredApiBaseUrl,
  setApiBaseUrl,
} from "../lib/runtime-api-base";
import { BackendEndpointDialog } from "./backend-endpoint-dialog";

type BackendEndpointContextValue = {
  effectiveApiBaseUrl: string;
  openSettings: () => void;
  resetToDefault: () => void;
};

const BackendEndpointContext = createContext<BackendEndpointContextValue | null>(null);

export function BackendEndpointProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const [effectiveApiBaseUrl, setEffectiveApiBaseUrl] = useState(() => getEffectiveApiBaseUrl());
  const [draftValue, setDraftValue] = useState(() => getEffectiveApiBaseUrl());
  const [dialogOpen, setDialogOpen] = useState(() => !hasStoredApiBaseUrl());
  const [requiresConfirmation, setRequiresConfirmation] = useState(() => !hasStoredApiBaseUrl());
  const [errorKey, setErrorKey] = useState<string | null>(null);

  const defaultApiBaseUrl = getDefaultApiBaseUrl();

  async function refreshQueries(nextApiBaseUrl: string) {
    setEffectiveApiBaseUrl(nextApiBaseUrl);
    await queryClient.invalidateQueries();
    await queryClient.refetchQueries();
  }

  async function handleSave() {
    try {
      const nextApiBaseUrl = setApiBaseUrl(draftValue);
      setDraftValue(nextApiBaseUrl);
      setErrorKey(null);
      setDialogOpen(false);
      setRequiresConfirmation(false);
      await refreshQueries(nextApiBaseUrl);
    } catch (saveError) {
      setErrorKey(
        saveError instanceof Error ? saveError.message : "backend.endpoint.error.unknown",
      );
    }
  }

  async function handleReset() {
    clearApiBaseUrl();
    setDraftValue(defaultApiBaseUrl);
    setErrorKey(null);
    setDialogOpen(false);
    await refreshQueries(defaultApiBaseUrl);
  }

  function openSettings() {
    setDraftValue(effectiveApiBaseUrl);
    setErrorKey(null);
    setDialogOpen(true);
  }

  const value = useMemo<BackendEndpointContextValue>(
    () => ({
      effectiveApiBaseUrl,
      openSettings,
      resetToDefault: handleReset,
    }),
    [effectiveApiBaseUrl],
  );

  return (
    <BackendEndpointContext.Provider value={value}>
      {children}
      <BackendEndpointDialog
        canClose={!requiresConfirmation}
        currentValue={draftValue}
        defaultValue={defaultApiBaseUrl}
        effectiveValue={effectiveApiBaseUrl}
        errorKey={errorKey}
        open={dialogOpen}
        onCurrentValueChange={(value) => {
          setDraftValue(value);
          if (errorKey) {
            setErrorKey(null);
          }
        }}
        onOpenChange={(open) => {
          if (!requiresConfirmation) {
            setDialogOpen(open);
          }
        }}
        onReset={handleReset}
        onSave={handleSave}
      />
    </BackendEndpointContext.Provider>
  );
}

export function useBackendEndpoint() {
  const context = useContext(BackendEndpointContext);
  if (!context) {
    throw new Error("useBackendEndpoint must be used within BackendEndpointProvider");
  }

  return context;
}
