import type { AppErrorDto } from "./api/bindings";

export function isAppErrorDto(value: unknown): value is AppErrorDto {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as Record<string, unknown>;

  return (
    typeof candidate.code === "string" &&
    typeof candidate.message === "string" &&
    (typeof candidate.details === "string" || candidate.details === null)
  );
}

export function normalizeError(error: unknown): AppErrorDto {
  if (isAppErrorDto(error)) {
    return error;
  }

  if (error instanceof Error) {
    return {
      code: "UNEXPECTED",
      message: "Ocorreu um erro inesperado.",
      details: error.message,
    };
  }

  if (typeof error === "string") {
    return {
      code: "UNEXPECTED",
      message: "Ocorreu um erro inesperado.",
      details: error,
    };
  }

  return {
    code: "UNEXPECTED",
    message: "Ocorreu um erro inesperado.",
    details: null,
  };
}
