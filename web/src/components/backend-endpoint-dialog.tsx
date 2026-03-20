import * as Dialog from "@radix-ui/react-dialog";

import { useI18n } from "../i18n";

type BackendEndpointDialogProps = {
  canClose: boolean;
  currentValue: string;
  defaultValue: string;
  effectiveValue: string;
  errorKey: string | null;
  open: boolean;
  onCurrentValueChange: (value: string) => void;
  onOpenChange: (open: boolean) => void;
  onReset: () => void;
  onSave: () => void;
};

export function BackendEndpointDialog({
  canClose,
  currentValue,
  defaultValue,
  effectiveValue,
  errorKey,
  open,
  onCurrentValueChange,
  onOpenChange,
  onReset,
  onSave,
}: BackendEndpointDialogProps) {
  const { t } = useI18n();

  return (
    <Dialog.Root open={open} onOpenChange={canClose ? onOpenChange : undefined}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/45 backdrop-blur-sm" />
        <Dialog.Content
          className="fixed left-1/2 top-1/2 z-50 w-[min(92vw,40rem)] -translate-x-1/2 -translate-y-1/2 rounded-[28px] border border-black/10 bg-[#f7f0e4] p-6 text-graphite shadow-panel outline-none"
          onEscapeKeyDown={(event) => {
            if (!canClose) {
              event.preventDefault();
            }
          }}
          onInteractOutside={(event) => {
            if (!canClose) {
              event.preventDefault();
            }
          }}
        >
          <div className="flex items-start justify-between gap-4">
            <div>
              <Dialog.Title className="text-xl font-semibold">
                {t("backend.endpoint.title")}
              </Dialog.Title>
              <Dialog.Description className="mt-2 text-sm leading-6 text-graphite/70">
                {t("backend.endpoint.description")}
              </Dialog.Description>
            </div>
            {canClose ? (
              <Dialog.Close className="rounded-full border border-black/10 px-3 py-1 text-xs uppercase tracking-[0.2em] text-graphite/60">
                {t("backend.endpoint.close")}
              </Dialog.Close>
            ) : null}
          </div>

          <div className="mt-6 space-y-4">
            <label className="block">
              <span className="text-xs uppercase tracking-[0.24em] text-steel">
                {t("backend.endpoint.label")}
              </span>
              <input
                className="mt-2 w-full rounded-2xl border border-black/10 bg-white/90 px-4 py-3 text-sm outline-none transition focus:border-graphite/30"
                placeholder={t("backend.endpoint.placeholder")}
                value={currentValue}
                onChange={(event) => onCurrentValueChange(event.target.value)}
              />
            </label>

            <div className="rounded-2xl border border-black/10 bg-white/70 px-4 py-3 text-sm text-graphite/70">
              <p>{t("backend.endpoint.current")}</p>
              <p className="mt-1 break-all font-medium text-graphite">{effectiveValue}</p>
              <p className="mt-3 text-xs text-graphite/55">
                {t("backend.endpoint.default", { value: defaultValue })}
              </p>
            </div>

            {errorKey ? (
              <div className="rounded-2xl border border-red-500/20 bg-red-50 px-4 py-3 text-sm text-red-700">
                {t(errorKey)}
              </div>
            ) : null}
          </div>

          <div className="mt-6 flex flex-wrap justify-end gap-3">
            <button
              className="rounded-full border border-black/10 px-4 py-2 text-sm text-graphite/75 transition hover:bg-black/5"
              type="button"
              onClick={onReset}
            >
              {t("backend.endpoint.reset")}
            </button>
            <button
              className="rounded-full bg-graphite px-4 py-2 text-sm text-sand transition hover:bg-black"
              type="button"
              onClick={onSave}
            >
              {t("backend.endpoint.save")}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
