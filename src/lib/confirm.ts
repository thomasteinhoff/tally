import { ask } from "@tauri-apps/plugin-dialog";

export function confirmAction(
  message: string,
  title: string,
  okLabel = "Confirmar",
): Promise<boolean> {
  return ask(message, {
    title,
    kind: "warning",
    okLabel,
    cancelLabel: "Cancelar",
  });
}
