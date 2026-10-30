import { invoke } from "@tauri-apps/api/core";

import type { BootstrapDto, IntegrityReportDto } from "./api/bindings";

export function getBootstrap(): Promise<BootstrapDto> {
  return invoke<BootstrapDto>("get_bootstrap");
}

export function runIntegrityCheck(): Promise<IntegrityReportDto> {
  return invoke<IntegrityReportDto>("run_integrity_check");
}
