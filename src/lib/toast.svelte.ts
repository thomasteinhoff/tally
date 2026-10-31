export type ToastTone = "ok" | "warn";

export type Toast = {
  message: string;
  tone: ToastTone;
};

export const toastState = $state<{ current: Toast | null }>({ current: null });

let timer: ReturnType<typeof setTimeout> | undefined;

export function notify(message: string, tone: ToastTone = "ok"): void {
  if (timer) {
    clearTimeout(timer);
  }

  toastState.current = { message, tone };
  timer = setTimeout(() => {
    toastState.current = null;
  }, 4_500);
}

export function dismissToast(): void {
  if (timer) {
    clearTimeout(timer);
  }
  toastState.current = null;
}
